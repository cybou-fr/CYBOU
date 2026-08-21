// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Bounded, read-only HTTP boundary between Living Canvas and Presence.

use std::{convert::Infallible, path::PathBuf, sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{any, get, post},
};
use cybou_web_contracts::{
    MindProjection, SessionMode, SessionProjection, ShellExecRequest, ShellExecResponse,
    SnapshotProjection, WEB_SCHEMA_V1,
};
use serde::Serialize;
use thiserror::Error;
use time::{Duration as TimeDuration, OffsetDateTime, format_description::well_known::Rfc3339};
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};
use uuid::Uuid;

pub mod access;
#[cfg(target_os = "linux")]
pub mod auth_socket;
pub mod disclose;
pub mod fixture;
#[cfg(target_os = "linux")]
pub mod presence_zbus;

pub use access::{CredentialVerifier, LoginOutcome, LoginRequest, Session, Sessions};
pub use disclose::Disclosures;

use cybou_protocol::{
    canonical::CanonicalEnvelope,
    disclosure::{ConsumerTrust, Destination},
};

/// Maximum time the gateway permits one Presence projection request to occupy.
pub const SNAPSHOT_BUDGET: Duration = Duration::from_millis(1_500);

/// Interval used until Presence exposes a native changed-event subscription.
pub const EVENT_POLL_INTERVAL: Duration = Duration::from_secs(2);

const MAX_CURSOR_BYTES: usize = 256;

/// Typed read-only source behind the HTTP boundary.
/// What one projection supplied, and what it held back.
///
/// Read after a projection is built, by whoever is going to record the delivery. It is separate
/// from the projection itself because a consumer must not be told what was withheld from it: that
/// a concept exists is frequently the sensitive part of it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Delivered {
    /// The contributions the supplied items were derived from.
    pub items: Vec<Uuid>,
    /// How many items were supplied, whether or not their provenance was known.
    pub item_count: u32,
    /// What was held back, and why.
    pub withheld: Vec<cybou_protocol::disclosure::Withheld>,
}

/// Where a disclosure record goes.
///
/// A trait so the gateway's own tests can watch what it would have recorded without a Journal, and
/// so a fixture-backed gateway can have none at all rather than pretend.
#[async_trait]
pub trait DisclosureSink: Send + Sync + 'static {
    /// Record one disclosure, answering whether it was accepted.
    ///
    /// `false` is a fact worth acting on rather than an error to swallow: the reader is still
    /// entitled to what they asked for, and the operator is entitled to know the audit trail has a
    /// hole in it.
    async fn record(&self, envelope: &CanonicalEnvelope) -> bool;
}

/// Where the gateway reads Mind from.
#[async_trait]
pub trait PresenceSource: Send + Sync + 'static {
    /// Return one atomic, presentation-ready snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError`] when transport, decoding, or owner availability prevents a typed
    /// projection. It must never turn those failures into an empty successful snapshot.
    async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError>;

    /// What the last projection this source built supplied, and what it left out.
    ///
    /// Defaulting to nothing is safe rather than convenient: a source that does not filter has
    /// nothing to report, and a source that does must say so or the withholding is invisible —
    /// which is the exact failure ADR-0030's B6 exists to prevent.
    fn last_delivery(&self) -> Delivered {
        Delivered::default()
    }

    /// Wait until the source may have a newer projection.
    ///
    /// Sources with a native change signal override this method. Deterministic fixtures retain a
    /// bounded polling fallback so the same gateway contract remains testable without D-Bus.
    async fn wait_for_change(&self) -> Result<(), GatewayError> {
        tokio::time::sleep(EVENT_POLL_INTERVAL).await;
        Ok(())
    }

    /// Return what the owners behind Mind actually hold right now.
    ///
    /// Defaulting to unavailable is deliberate: a source that cannot reach the owners must say so
    /// rather than answer with a projection full of absent sections, which a reader could mistake
    /// for a Mind that holds nothing.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError`] when the source cannot produce the projection at all.
    async fn mind(&self) -> Result<MindProjection, GatewayError> {
        Err(GatewayError::Unavailable)
    }
}

/// Failure response safe to expose at the browser boundary.
#[derive(Clone, Debug, Error)]
pub enum GatewayError {
    /// Presence did not answer within the gateway's outer budget.
    #[error("presence snapshot exceeded the gateway budget")]
    Timeout,
    /// Presence transport is unavailable.
    #[error("presence transport is unavailable")]
    Unavailable,
    /// Presence returned data that cannot satisfy the web contract.
    #[error("presence projection is incompatible with the web contract")]
    InvalidProjection,
    /// A resume cursor is malformed or exceeds the bounded header budget.
    #[error("event resume cursor is invalid")]
    InvalidCursor,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody {
    schema_version: cybou_protocol::SchemaVersion,
    error: &'static str,
    retryable: bool,
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let (status, error, retryable) = match self {
            Self::Timeout => (StatusCode::GATEWAY_TIMEOUT, "presenceTimeout", true),
            Self::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "presenceUnavailable", true),
            Self::InvalidProjection => {
                (StatusCode::BAD_GATEWAY, "invalidPresenceProjection", false)
            }
            Self::InvalidCursor => (StatusCode::BAD_REQUEST, "invalidEventCursor", false),
        };
        (
            status,
            Json(ErrorBody {
                schema_version: WEB_SCHEMA_V1,
                error,
                retryable,
            }),
        )
            .into_response()
    }
}

#[derive(Clone)]
struct GatewayState {
    presence: Arc<dyn PresenceSource>,
    /// The unfiltered source, for a reader who has signed in.
    ///
    /// `None` when this deployment cannot authenticate anybody, which is a valid way to run a
    /// public demo: every reader is then served the filtered source and there is no way in.
    privileged: Option<Arc<dyn PresenceSource>>,
    /// Who can say whether an account accepts a secret, if anything can.
    verifier: Option<Arc<dyn CredentialVerifier>>,
    /// The sessions this process is honouring.
    sessions: Arc<Sessions>,
    /// What has already been recorded as supplied to each consumer.
    disclosures: Arc<Disclosures>,
    /// Where a disclosure record is written, when there is one to write.
    ///
    /// `None` leaves deliveries unrecorded, which is what a fixture-backed gateway does: there is
    /// no Journal behind it to record into. A deployment without this is a deployment that cannot
    /// say who read what, and that is a fact about the deployment rather than a silent default.
    journal: Option<Arc<dyn DisclosureSink>>,
    session: SessionProjection,
    shell: Arc<tokio::sync::Mutex<cybou_shelld::ShellEngine>>,
}

impl GatewayState {
    /// Who this request is, as a consumer.
    ///
    /// The identity is the one the gateway established, never the one the caller offers: a
    /// consumer that could name itself could also name somebody else, and the record would say so.
    fn destination_for(session: Option<&Session>) -> Destination {
        session.map_or_else(
            || Destination {
                id: "living-canvas:public".to_owned(),
                trust: ConsumerTrust::Public,
                // A browser renders and forgets; what makes this recordable is the boundary it
                // crossed on the way out, which no later decision can call back.
                retains: false,
                external_boundary: true,
            },
            |session| Destination {
                id: format!("living-canvas:{}", session.username),
                trust: ConsumerTrust::Owner,
                retains: false,
                external_boundary: true,
            },
        )
    }

    /// Record that this consumer was supplied what the source just built, if that is new.
    async fn record_delivery(&self, destination: &Destination, delivered: &Delivered) {
        let Some(journal) = &self.journal else {
            return;
        };
        let Some(record) =
            self.disclosures
                .record_for(destination, delivered, OffsetDateTime::now_utc())
        else {
            return;
        };
        // A delivery that could not be recorded is not a reason to withhold the answer: the reader
        // is entitled to it either way, and refusing would trade an incomplete record for an
        // outage. It is said out loud instead, because an audit trail with silent holes is worse
        // than one with known ones.
        if !journal.record(&record).await {
            eprintln!(
                "a delivery to {} could not be recorded; the audit trail is incomplete",
                destination.id
            );
        }
    }

    /// The session this request carries, if it carries a live one.
    fn session_for(&self, headers: &HeaderMap) -> Option<Session> {
        let token = headers
            .get(axum::http::header::COOKIE)
            .and_then(|value| value.to_str().ok())
            .and_then(access::token_in)?;
        self.sessions.resolve(token, OffsetDateTime::now_utc())
    }

    /// The source this request is entitled to.
    ///
    /// Defaulting to the filtered source is what makes a missing, expired or invented session
    /// harmless: none of them is an entitlement, and they do not need to be told apart to be
    /// refused. Nothing is rejected outright, because a public surface that answered 401 to
    /// strangers would stop being a public surface.
    fn source_for(&self, headers: &HeaderMap) -> &Arc<dyn PresenceSource> {
        let Some(privileged) = &self.privileged else {
            return &self.presence;
        };
        if self.session_for(headers).is_some() {
            privileged
        } else {
            &self.presence
        }
    }
}

/// Server-established browser trust context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionContext {
    /// Trust mode presented to the frontend.
    pub mode: SessionMode,
    /// Stable named consumer used by projection policy.
    pub consumer_id: String,
}

impl SessionContext {
    /// Device-bound context used by the desktop shell.
    #[must_use]
    pub fn local_desktop() -> Self {
        Self {
            mode: SessionMode::LocalDesktop,
            consumer_id: "cybou-desktop".into(),
        }
    }

    /// Read-only public preview context.
    #[must_use]
    pub fn public_preview() -> Self {
        Self {
            mode: SessionMode::PublicPreview,
            consumer_id: "public-preview".into(),
        }
    }
}

/// Build the v1 router around a single source.
pub fn router(presence: Arc<dyn PresenceSource>) -> Router {
    router_with_verifier_and_access(presence, None, None, None, SessionContext::local_desktop())
}

/// Build the v1 router serving static assets when configured.
pub fn router_with_assets(presence: Arc<dyn PresenceSource>, web_root: Option<PathBuf>) -> Router {
    router_with_verifier_and_access(
        presence,
        None,
        None,
        web_root,
        SessionContext::local_desktop(),
    )
}

/// Build the v1 router with an explicit server-established trust context.
pub fn router_with_assets_and_session(
    presence: Arc<dyn PresenceSource>,
    web_root: Option<PathBuf>,
    session_context: SessionContext,
) -> Router {
    router_with_verifier_and_access(presence, None, None, web_root, session_context)
}

/// Build the v1 router with a filtered source for everyone and a full source for a signed-in
/// reader.
///
/// The two sources are separate objects rather than one source and a flag, so a route cannot
/// ask the wrong question and get the privileged answer: what a request is entitled to is
/// decided once, where the session is read.
///
/// # Panics
///
/// If the sandbox the shell surface runs in cannot be created.
///
/// # Panics
///
/// If the sandbox jail the shell surface runs in cannot be created.
pub fn router_with_verifier_and_access(
    presence: Arc<dyn PresenceSource>,
    privileged: Option<Arc<dyn PresenceSource>>,
    verifier: Option<Arc<dyn CredentialVerifier>>,
    web_root: Option<PathBuf>,
    session_context: SessionContext,
) -> Router {
    router_recording_disclosures(
        presence,
        privileged,
        verifier,
        None,
        web_root,
        session_context,
    )
}

/// Build the v1 router that records what it supplies to whom.
///
/// The sink is where a disclosure record goes. Without one, deliveries are not recorded — which is
/// what a fixture-backed gateway does, because there is no Journal behind it to record into.
///
/// # Panics
///
/// If the sandbox the shell surface runs in cannot be created.
pub fn router_recording_disclosures(
    presence: Arc<dyn PresenceSource>,
    privileged: Option<Arc<dyn PresenceSource>>,
    verifier: Option<Arc<dyn CredentialVerifier>>,
    journal: Option<Arc<dyn DisclosureSink>>,
    web_root: Option<PathBuf>,
    session_context: SessionContext,
) -> Router {
    let now = OffsetDateTime::now_utc();
    let expires_at = (now + TimeDuration::hours(8))
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into());

    let sandbox_path = std::env::temp_dir().join(format!("cybou_sandbox_{}", std::process::id()));
    let jail = cybou_jailfs::JailFs::new(&sandbox_path).expect("initialize gateway sandbox jail");
    let shell = Arc::new(tokio::sync::Mutex::new(cybou_shelld::ShellEngine::new(
        jail,
    )));

    let state = GatewayState {
        presence,
        privileged,
        verifier,
        sessions: Arc::new(Sessions::default()),
        disclosures: Arc::new(Disclosures::new()),
        journal,
        session: SessionProjection {
            schema_version: WEB_SCHEMA_V1,
            session_id: Uuid::new_v4(),
            mode: session_context.mode,
            consumer_id: session_context.consumer_id,
            expires_at,
        },
        shell,
    };

    let app = Router::new()
        .route("/api/v1/session", get(session))
        .route("/api/v1/login", post(login))
        .route("/api/v1/logout", post(logout))
        .route("/api/v1/snapshot", get(snapshot))
        .route("/api/v1/mind", get(mind))
        .route("/api/v1/events", get(events))
        .route("/api/v1/shell/exec", post(shell_exec))
        .route("/api/{*path}", any(api_not_found))
        .with_state(state)
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; connect-src 'self'; img-src 'self' data:; font-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
            ),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("cache-control"),
            HeaderValue::from_static("no-store"),
        ));

    match web_root {
        Some(root) => app.fallback_service(
            ServeDir::new(&root).not_found_service(ServeFile::new(root.join("index.html"))),
        ),
        None => app,
    }
}

async fn api_not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}

async fn session(State(state): State<GatewayState>, headers: HeaderMap) -> Json<SessionProjection> {
    let mut projection = state.session.clone();
    // A signed-in reader is a different trust context, and the contract already has a name for it.
    // Leaving it at `publicPreview` would make the page unable to tell that anyone had signed in,
    // and would state a trust level that is no longer the one that was established.
    if let Some(session) = state.session_for(&headers) {
        projection.mode = SessionMode::RemoteBrowser;
        projection.consumer_id.clone_from(&session.username);
        projection.expires_at = session
            .expires_at
            .format(&Rfc3339)
            .unwrap_or_else(|_| projection.expires_at.clone());
    }
    Json(projection)
}

/// Establish a session for a Linux account.
///
/// The gateway never learns whether the account exists: it asks the privileged helper and receives
/// one bit. A deployment with no helper answers the same as a wrong password, because in both
/// cases nothing was established.
async fn login(
    State(state): State<GatewayState>,
    Json(request): Json<LoginRequest>,
) -> impl IntoResponse {
    let authenticated = match &state.verifier {
        Some(verifier) => verifier.verify(&request.username, &request.password).await,
        None => false,
    };
    let username = request.username.clone();
    // Dropped here rather than at the end of the function: the secret is gone from this process
    // before the response is built.
    drop(request);

    if !authenticated {
        return (
            StatusCode::UNAUTHORIZED,
            Json(LoginOutcome {
                authenticated: false,
            }),
        )
            .into_response();
    }

    let token = state.sessions.begin(&username, OffsetDateTime::now_utc());
    (
        StatusCode::OK,
        [(
            axum::http::header::SET_COOKIE,
            access::session_cookie(&token),
        )],
        Json(LoginOutcome {
            authenticated: true,
        }),
    )
        .into_response()
}

/// End the session this request carries, if it carries one.
///
/// Answers the same either way. Whether a token named a live session is not something a caller
/// needs told, and saying so would make this a way to test tokens.
async fn logout(State(state): State<GatewayState>, headers: HeaderMap) -> impl IntoResponse {
    if let Some(token) = headers
        .get(axum::http::header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(access::token_in)
    {
        state.sessions.end(token);
    }
    (
        StatusCode::OK,
        [(axum::http::header::SET_COOKIE, access::cleared_cookie())],
    )
}

async fn snapshot(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<SnapshotProjection>, GatewayError> {
    tokio::time::timeout(SNAPSHOT_BUDGET, state.source_for(&headers).snapshot())
        .await
        .map_err(|_| GatewayError::Timeout)?
        .map(Json)
}

async fn mind(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<MindProjection>, GatewayError> {
    let session = state.session_for(&headers);
    let source = state.source_for(&headers);
    let projection = tokio::time::timeout(SNAPSHOT_BUDGET, source.mind())
        .await
        .map_err(|_| GatewayError::Timeout)??;

    // Recorded after the projection is built and before it is answered, so what the record says
    // was supplied is what this reader is about to receive.
    state
        .record_delivery(
            &GatewayState::destination_for(session.as_ref()),
            &source.last_delivery(),
        )
        .await;
    Ok(Json(projection))
}

async fn shell_exec(
    State(state): State<GatewayState>,
    Json(payload): Json<ShellExecRequest>,
) -> Result<Json<ShellExecResponse>, (StatusCode, Json<ErrorBody>)> {
    if state.session.mode == SessionMode::PublicPreview {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorBody {
                schema_version: WEB_SCHEMA_V1,
                error: "shellExecutionForbiddenInPublicPreview",
                retryable: false,
            }),
        ));
    }

    let mut engine = state.shell.lock().await;
    let output = engine.execute(&payload.command);

    Ok(Json(ShellExecResponse {
        schema_version: WEB_SCHEMA_V1,
        exit_code: output.exit_code,
        stdout: output.stdout,
        stderr: output.stderr,
        cwd: output.cwd,
    }))
}

fn resume_cursor(headers: &HeaderMap) -> Result<Option<String>, GatewayError> {
    let Some(value) = headers.get("last-event-id") else {
        return Ok(None);
    };
    let value = value.to_str().map_err(|_| GatewayError::InvalidCursor)?;
    if value.is_empty() || value.len() > MAX_CURSOR_BYTES || value.chars().any(char::is_control) {
        return Err(GatewayError::InvalidCursor);
    }
    Ok(Some(value.to_owned()))
}

async fn events(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Sse<impl futures_core::Stream<Item = Result<Event, Infallible>>>, GatewayError> {
    let initial_cursor = resume_cursor(&headers)?;
    let stream = futures_util::stream::unfold(
        (state, initial_cursor),
        |(state, mut last_cursor)| async move {
            loop {
                let result = tokio::time::timeout(SNAPSHOT_BUDGET, state.presence.snapshot()).await;
                match result {
                    Ok(Ok(snapshot))
                        if last_cursor.as_deref() != Some(snapshot.cursor.as_str()) =>
                    {
                        let cursor = snapshot.cursor.clone();
                        if let Ok(data) = serde_json::to_string(&snapshot) {
                            let event = Event::default()
                                .event("snapshot")
                                .id(cursor.clone())
                                .data(data);
                            last_cursor = Some(cursor);
                            return Some((Ok::<_, Infallible>(event), (state, last_cursor)));
                        }
                        let data = serde_json::json!({
                            "schemaVersion": WEB_SCHEMA_V1,
                            "error": "invalidPresenceProjection",
                            "retryable": false
                        });
                        let event = Event::default()
                            .event("projection-error")
                            .data(data.to_string());
                        return Some((Ok(event), (state, last_cursor)));
                    }
                    Ok(Ok(_)) => {}
                    Ok(Err(error)) => {
                        let data = serde_json::json!({
                            "schemaVersion": WEB_SCHEMA_V1,
                            "error": error.to_string(),
                            "retryable": !matches!(error, GatewayError::InvalidProjection | GatewayError::InvalidCursor)
                        });
                        let event = Event::default()
                            .event("projection-error")
                            .data(data.to_string());
                        return Some((Ok(event), (state, last_cursor)));
                    }
                    Err(_) => {
                        let data = serde_json::json!({
                            "schemaVersion": WEB_SCHEMA_V1,
                            "error": "presenceTimeout",
                            "retryable": true
                        });
                        let event = Event::default()
                            .event("projection-error")
                            .data(data.to_string());
                        return Some((Ok(event), (state, last_cursor)));
                    }
                }
                if state.presence.wait_for_change().await.is_err() {
                    tokio::time::sleep(EVENT_POLL_INTERVAL).await;
                }
            }
        },
    );
    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("presence-stream"),
    ))
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        },
        time::Duration,
    };

    use async_trait::async_trait;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
    };
    use cybou_protocol::KnowledgeState;
    use cybou_web_contracts::{
        MindProjection, SessionMode, SessionProjection, ShellExecRequest, ShellExecResponse,
        SnapshotProjection,
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::{
        CredentialVerifier, GatewayError, PresenceSource, SessionContext, router,
        router_with_assets, router_with_assets_and_session, router_with_verifier_and_access,
    };
    use crate::fixture::FixturePresenceSource;

    /// A source that reports whether it was the one asked.
    struct NamedSource {
        name: &'static str,
    }

    #[async_trait]
    impl PresenceSource for NamedSource {
        async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError> {
            let mut snapshot = FixturePresenceSource::nominal()
                .snapshot()
                .await
                .expect("the fixture answers");
            snapshot.cursor = self.name.to_owned();
            Ok(snapshot)
        }
    }

    /// A verifier that accepts exactly one account with exactly one secret.
    struct OneAccount;

    #[async_trait]
    impl CredentialVerifier for OneAccount {
        async fn verify(&self, username: &str, password: &str) -> bool {
            username == "alice" && password == "hunter2"
        }
    }

    async fn cursor_for(app: &Router, cookie: Option<&str>) -> String {
        let mut request = Request::builder().uri("/api/v1/snapshot");
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).expect("a request"))
            .await
            .expect("a response");
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("a body");
        let projection: SnapshotProjection = serde_json::from_slice(&body).expect("a projection");
        projection.cursor
    }

    /// Sign in and return the cookie the browser would send back, if it worked.
    async fn sign_in(app: &Router, username: &str, password: &str) -> Option<String> {
        let body = format!(r#"{{"username":"{username}","password":"{password}"}}"#);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/login")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("a request"),
            )
            .await
            .expect("a response");
        if response.status() != StatusCode::OK {
            return None;
        }
        let cookie = response
            .headers()
            .get("set-cookie")?
            .to_str()
            .ok()?
            .split(';')
            .next()?
            .to_owned();
        Some(cookie)
    }

    fn guarded_router() -> Router {
        router_with_verifier_and_access(
            Arc::new(NamedSource { name: "public" }),
            Some(Arc::new(NamedSource { name: "privileged" })),
            Some(Arc::new(OneAccount)),
            None,
            SessionContext::public_preview(),
        )
    }

    #[tokio::test]
    async fn a_reader_who_has_not_signed_in_is_served_the_public_source() {
        let app = guarded_router();
        assert_eq!(cursor_for(&app, None).await, "public");
    }

    #[tokio::test]
    async fn a_session_nobody_issued_is_served_the_public_source_rather_than_refused() {
        // Refusing would stop this being a public surface. A stranger gets what a stranger gets,
        // and an invented cookie is a stranger.
        let app = guarded_router();
        assert_eq!(
            cursor_for(&app, Some("cybou_session=invented")).await,
            "public"
        );
        assert_eq!(cursor_for(&app, Some("cybou_session=")).await, "public");
        assert_eq!(cursor_for(&app, Some("theme=dark")).await, "public");
    }

    #[tokio::test]
    async fn signing_in_is_what_reaches_the_unfiltered_source() {
        let app = guarded_router();
        let cookie = sign_in(&app, "alice", "hunter2")
            .await
            .expect("the account and secret are the ones this verifier accepts");
        assert_eq!(cursor_for(&app, Some(&cookie)).await, "privileged");
    }

    #[tokio::test]
    async fn a_wrong_secret_establishes_nothing() {
        let app = guarded_router();
        assert!(sign_in(&app, "alice", "not-hunter2").await.is_none());
        // And the account that does not exist is answered exactly the same way, so a caller cannot
        // learn from the reply which accounts are real.
        assert!(sign_in(&app, "mallory", "hunter2").await.is_none());
        assert!(sign_in(&app, "alice", "").await.is_none());
    }

    #[tokio::test]
    async fn signing_out_ends_the_session_it_was_given() {
        let app = guarded_router();
        let cookie = sign_in(&app, "alice", "hunter2").await.expect("signed in");
        assert_eq!(cursor_for(&app, Some(&cookie)).await, "privileged");

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/logout")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("a request"),
            )
            .await
            .expect("a response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(cursor_for(&app, Some(&cookie)).await, "public");
    }

    #[tokio::test]
    async fn a_deployment_that_cannot_authenticate_anybody_serves_everyone_the_public_source() {
        let app = router_with_verifier_and_access(
            Arc::new(NamedSource { name: "public" }),
            None,
            None,
            None,
            SessionContext::public_preview(),
        );
        assert_eq!(cursor_for(&app, None).await, "public");
        assert!(sign_in(&app, "alice", "hunter2").await.is_none());
    }

    #[tokio::test]
    async fn a_signed_in_session_says_which_account_it_belongs_to() {
        // The page has to be able to tell that somebody signed in, and as whom. Leaving the mode at
        // publicPreview would state a trust level that is no longer the one established.
        let app = guarded_router();
        let cookie = sign_in(&app, "alice", "hunter2").await.expect("signed in");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/session")
                    .header("cookie", &cookie)
                    .body(Body::empty())
                    .expect("a request"),
            )
            .await
            .expect("a response");
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("a body");
        let projection: SessionProjection = serde_json::from_slice(&body).expect("a projection");
        assert_eq!(projection.mode, SessionMode::RemoteBrowser);
        assert_eq!(projection.consumer_id, "alice");
    }

    #[tokio::test]
    async fn session_is_server_established_local_and_not_cached() {
        let response = router(Arc::new(FixturePresenceSource::nominal()))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/session")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("session body")
            .to_bytes();
        let session: SessionProjection = serde_json::from_slice(&bytes).expect("typed session");
        assert_eq!(session.mode, SessionMode::LocalDesktop);
    }

    #[tokio::test]
    async fn public_preview_never_claims_local_desktop_trust() {
        let response = router_with_assets_and_session(
            Arc::new(FixturePresenceSource::nominal()),
            None,
            SessionContext::public_preview(),
        )
        .oneshot(
            Request::builder()
                .uri("/api/v1/session")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("router response");
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("session body")
            .to_bytes();
        let session: SessionProjection = serde_json::from_slice(&bytes).expect("typed session");
        assert_eq!(session.mode, SessionMode::PublicPreview);
        assert_eq!(session.consumer_id, "public-preview");
    }

    #[tokio::test]
    async fn event_stream_emits_snapshot_with_resumable_cursor() {
        let response = router(Arc::new(FixturePresenceSource::nominal()))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/events")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "text/event-stream");
        let mut body = response.into_body();
        let frame = tokio::time::timeout(Duration::from_millis(100), body.frame())
            .await
            .expect("initial event budget")
            .expect("initial event frame")
            .expect("valid event frame");
        let data = frame.into_data().expect("event data");
        let event = String::from_utf8_lossy(&data);
        assert!(event.contains("event: snapshot"));
        assert!(event.contains("id: fixture:presence:42"));
        assert!(event.contains("\"projectionVersion\":42"));
    }

    #[tokio::test]
    async fn event_stream_rejects_unbounded_resume_cursor() {
        let response = router(Arc::new(FixturePresenceSource::nominal()))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/events")
                    .header("last-event-id", "x".repeat(257))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    struct SignaledPresence {
        version: AtomicU64,
    }

    #[async_trait]
    impl PresenceSource for SignaledPresence {
        async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError> {
            let version = self.version.load(Ordering::Relaxed);
            let mut snapshot = FixturePresenceSource::nominal().snapshot().await?;
            snapshot.projection_version = version;
            snapshot.cursor = format!("signal:{version}");
            Ok(snapshot)
        }

        async fn wait_for_change(&self) -> Result<(), GatewayError> {
            self.version.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[tokio::test]
    async fn event_stream_waits_for_source_signal_before_new_snapshot() {
        let response = router(Arc::new(SignaledPresence {
            version: AtomicU64::new(42),
        }))
        .oneshot(
            Request::builder()
                .uri("/api/v1/events")
                .header("last-event-id", "signal:42")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("router response");
        let mut body = response.into_body();
        let frame = tokio::time::timeout(Duration::from_millis(100), body.frame())
            .await
            .expect("native change notification budget")
            .expect("changed event frame")
            .expect("valid changed event frame");
        let data = frame.into_data().expect("event data");
        let event = String::from_utf8_lossy(&data);
        assert!(event.contains("id: signal:43"));
        assert!(event.contains("\"projectionVersion\":43"));
    }

    #[tokio::test]
    async fn snapshot_round_trips_as_typed_projection() {
        let response = router(Arc::new(FixturePresenceSource::nominal()))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/snapshot")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router response");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("snapshot body")
            .to_bytes();
        let snapshot: SnapshotProjection = serde_json::from_slice(&bytes).expect("typed snapshot");
        assert_eq!(snapshot.projection_version, 42);
    }

    #[tokio::test]
    async fn mind_route_returns_what_the_owners_hold() {
        let response = router(Arc::new(FixturePresenceSource::nominal()))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/mind")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router response");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("mind body")
            .to_bytes();
        let mind: MindProjection = serde_json::from_slice(&bytes).expect("typed mind projection");
        assert_eq!(mind.identity.knowledge, KnowledgeState::Known);
        assert_eq!(mind.journal.contribution_count, Some(134));
        assert_eq!(mind.commitments.open.len(), 1);
    }

    #[tokio::test]
    async fn a_source_that_cannot_reach_the_owners_says_so_rather_than_answering_empty() {
        struct SnapshotOnlySource;

        #[async_trait]
        impl PresenceSource for SnapshotOnlySource {
            async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError> {
                FixturePresenceSource::nominal().snapshot().await
            }
        }

        let response = router(Arc::new(SnapshotOnlySource))
            .oneshot(
                Request::builder()
                    .uri("/api/v1/mind")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router response");
        assert_ne!(response.status(), StatusCode::OK);
    }

    struct SlowPresence;

    #[async_trait]
    impl PresenceSource for SlowPresence {
        async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError> {
            tokio::time::sleep(Duration::from_secs(5)).await;
            Err(GatewayError::Unavailable)
        }
    }

    #[tokio::test]
    async fn outer_budget_returns_typed_timeout_instead_of_empty_success() {
        tokio::time::pause();
        let task = tokio::spawn(async {
            router(Arc::new(SlowPresence))
                .oneshot(
                    Request::builder()
                        .uri("/api/v1/snapshot")
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("router response")
        });
        tokio::time::advance(Duration::from_secs(2)).await;
        let response = task.await.expect("timeout task");
        assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
    }

    #[tokio::test]
    async fn mutation_and_generic_rpc_routes_do_not_exist() {
        for uri in ["/api/v1/rpc", "/api/v1/commands/promise"] {
            let response = router(Arc::new(FixturePresenceSource::nominal()))
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(uri)
                        .body(Body::empty())
                        .expect("request"),
                )
                .await
                .expect("router response");
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
    }

    #[tokio::test]
    async fn living_canvas_and_api_share_an_origin_without_spa_fallback_for_unknown_api() {
        let root = tempfile::tempdir().expect("temporary web root");
        std::fs::write(
            root.path().join("index.html"),
            "<!doctype html><title>Living Canvas</title>",
        )
        .expect("test index");
        let app = router_with_assets(
            Arc::new(FixturePresenceSource::nominal()),
            Some(root.path().to_path_buf()),
        );

        let page = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("page response");
        assert_eq!(page.status(), StatusCode::OK);
        let page_bytes = page
            .into_body()
            .collect()
            .await
            .expect("page body")
            .to_bytes();
        assert!(String::from_utf8_lossy(&page_bytes).contains("Living Canvas"));

        let unknown_api = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/not-a-route")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("unknown API response");
        assert_eq!(unknown_api.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn shell_exec_runs_in_local_desktop_mode() {
        let app = router(Arc::new(FixturePresenceSource::nominal()));
        let payload = serde_json::to_vec(&ShellExecRequest {
            command: "pwd".into(),
        })
        .expect("serialize request");

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/shell/exec")
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let parsed: ShellExecResponse =
            serde_json::from_slice(&body_bytes).expect("deserialize shell response");
        assert_eq!(parsed.exit_code, 0);
        assert_eq!(parsed.stdout.trim(), "/");
    }

    #[tokio::test]
    async fn shell_exec_is_strictly_forbidden_in_public_preview() {
        let app = router_with_assets_and_session(
            Arc::new(FixturePresenceSource::nominal()),
            None,
            SessionContext::public_preview(),
        );
        let payload = serde_json::to_vec(&ShellExecRequest {
            command: "pwd".into(),
        })
        .expect("serialize request");

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/shell/exec")
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
