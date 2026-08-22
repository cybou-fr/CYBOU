// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Bounded, read-only HTTP boundary between Living Canvas and Presence.

use std::{path::PathBuf, sync::Arc};

use axum::{
    Router,
    http::{HeaderName, HeaderValue},
    routing::{any, get, post},
};
use cybou_web_contracts::{SessionProjection, WEB_SCHEMA_V1};
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
pub mod routes;
pub mod shells;
pub mod state;

pub use access::{CredentialVerifier, LoginOutcome, LoginRequest, Session, Sessions};
pub use disclose::Disclosures;
pub use shells::{SHELL_IDLE_LIFETIME, ShellOwner, Shells, sandbox_root};
pub use state::{
    Delivered, DisclosureSink, EVENT_POLL_INTERVAL, GatewayError, PresenceSource, SNAPSHOT_BUDGET,
    SessionContext,
};

use routes::{
    api_not_found, disclosure_handler, events_handler, login_handler, logout_handler, mind_handler,
    session_handler, shell_exec_handler, snapshot_handler,
};
use state::GatewayState;

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
    router_in_sandbox(
        presence,
        privileged,
        verifier,
        journal,
        web_root,
        session_context,
        &shells::sandbox_root(),
    )
}

/// Build the v1 router around a named sandbox root.
///
/// The public builders resolve the root from the deployment. This one is told, because a test must
/// not reach into whatever sandbox the host happens to have: the deployed host has a real
/// `/home/demo` that the build user cannot write to, and a test that wrote there passed on a
/// developer machine and failed on the one that matters.
///
/// # Panics
///
/// If the sandbox the shell surface runs in cannot be created.
pub(crate) fn router_in_sandbox(
    presence: Arc<dyn PresenceSource>,
    privileged: Option<Arc<dyn PresenceSource>>,
    verifier: Option<Arc<dyn CredentialVerifier>>,
    journal: Option<Arc<dyn DisclosureSink>>,
    web_root: Option<PathBuf>,
    session_context: SessionContext,
    sandbox_path: &std::path::Path,
) -> Router {
    let now = OffsetDateTime::now_utc();
    let expires_at = (now + TimeDuration::hours(8))
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into());

    let jail = cybou_jailfs::JailFs::new(sandbox_path).expect("initialize gateway sandbox jail");
    let shells = Arc::new(shells::Shells::new(jail));

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
        shells,
    };

    let app = Router::new()
        .route("/api/v1/session", get(session_handler))
        .route("/api/v1/login", post(login_handler))
        .route("/api/v1/logout", post(logout_handler))
        .route("/api/v1/snapshot", get(snapshot_handler))
        .route("/api/v1/mind", get(mind_handler))
        .route("/api/v1/events", get(events_handler))
        .route("/api/v1/disclosure", get(disclosure_handler))
        .route("/api/v1/shell/exec", post(shell_exec_handler))
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
        DisclosureProjection, MindProjection, SessionMode, SessionProjection, ShellExecRequest,
        ShellExecResponse, SnapshotProjection,
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::{
        CredentialVerifier, GatewayError, PresenceSource, SessionContext, router,
        router_with_assets, router_with_assets_and_session, router_with_verifier_and_access,
    };
    use crate::fixture::FixturePresenceSource;

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

    /// A source that reports a delivery with a gap in it.
    ///
    /// The fixture source supplies nothing and withholds nothing, which is the one case where the
    /// disclosure surface has no work to do. This one supplies more than it can account for and
    /// refuses two items for two different reasons, because those are the facts the surface exists
    /// to show.
    struct PartlyAccountedSource;

    #[async_trait]
    impl PresenceSource for PartlyAccountedSource {
        async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError> {
            FixturePresenceSource::nominal().snapshot().await
        }

        async fn mind(&self) -> Result<MindProjection, GatewayError> {
            FixturePresenceSource::nominal().mind().await
        }

        fn last_delivery(&self) -> crate::Delivered {
            crate::Delivered {
                // Two source contributions, cited between them by the two items that could say
                // where they came from. A set of sources, not a count of items.
                items: vec![Uuid::from_u128(1), Uuid::from_u128(2)],
                // Three supplied, two of which named their provenance. The third is the case the
                // two numbers exist for.
                item_count: 3,
                accounted_for: 2,
                withheld: vec![
                    cybou_protocol::disclosure::Withheld {
                        subject: Some("disk-pressure".to_owned()),
                        because: cybou_protocol::disclosure::WithheldBecause::AboveConsumerTrust,
                    },
                    cybou_protocol::disclosure::Withheld {
                        subject: None,
                        because: cybou_protocol::disclosure::WithheldBecause::BelongsToThePerson,
                    },
                ],
            }
        }
    }

    /// A guarded router whose shell sandbox is a directory this test owns.
    ///
    /// The `TempDir` is returned rather than dropped: dropping it removes the sandbox out from
    /// under the router. Tests bind it to `_sandbox` so it lives to the end of the test.
    fn shell_router_over_a_temporary_sandbox() -> (Router, tempfile::TempDir) {
        let sandbox = tempfile::tempdir().expect("a sandbox root");
        std::fs::create_dir(sandbox.path().join("somewhere")).expect("a directory to enter");
        let app = crate::router_in_sandbox(
            Arc::new(FixturePresenceSource::nominal()),
            None,
            Some(Arc::new(OneAccount)),
            None,
            None,
            SessionContext::public_preview(),
            sandbox.path(),
        );
        (app, sandbox)
    }

    /// Read the disclosure surface as whoever holds this cookie, if any.
    async fn disclosure_for(app: &Router, cookie: Option<&str>) -> DisclosureProjection {
        let mut request = Request::builder().uri("/api/v1/disclosure");
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).expect("a request"))
            .await
            .expect("a response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("a body");
        serde_json::from_slice(&body).expect("a disclosure projection")
    }

    /// Fetch the Mind projection, which is what records a delivery.
    async fn read_mind(app: &Router, cookie: Option<&str>) {
        let mut request = Request::builder().uri("/api/v1/mind");
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).expect("a request"))
            .await
            .expect("a response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn nothing_supplied_yet_is_not_the_same_as_an_empty_delivery() {
        // A surface that answered "you were supplied nothing" before anything had been supplied
        // would be making a claim about a delivery that never happened.
        let app = router(Arc::new(PartlyAccountedSource));
        let disclosure = disclosure_for(&app, None).await;
        assert!(!disclosure.delivered);
        assert_eq!(disclosure.supplied, 0);
        assert!(disclosure.withheld.is_empty());
    }

    #[tokio::test]
    async fn a_reader_is_shown_that_something_was_kept_from_them_and_why() {
        // This router has no Journal sink, which is deliberate: what was supplied must be
        // remembered whether or not there is anywhere durable to write it down.
        //
        // It also carries no session, so the reader is the public consumer even though the
        // deployment is a local desktop — which is correct, because a reader without a session is
        // served the filtered source. They are told what was refused and why; the subjects are the
        // business of the separate test below.
        let app = router(Arc::new(PartlyAccountedSource));
        read_mind(&app, None).await;

        let disclosure = disclosure_for(&app, None).await;
        assert!(disclosure.delivered);
        assert_eq!(disclosure.withheld.len(), 2);
        assert!(
            disclosure
                .withheld
                .iter()
                .any(|item| item.because == "aboveConsumerTrust")
        );
        assert!(
            disclosure
                .withheld
                .iter()
                .any(|item| item.because == "belongsToThePerson")
        );
    }

    /// A source whose items cite far more contributions than there are items.
    ///
    /// The realistic shape: one belief can name hundreds of contributions. This is what made the
    /// provenance set unusable as a count, and what made the response a hundred kilobytes.
    struct WidelyDerivedSource;

    #[async_trait]
    impl PresenceSource for WidelyDerivedSource {
        async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError> {
            FixturePresenceSource::nominal().snapshot().await
        }

        async fn mind(&self) -> Result<MindProjection, GatewayError> {
            FixturePresenceSource::nominal().mind().await
        }

        fn last_delivery(&self) -> crate::Delivered {
            crate::Delivered {
                items: (0..500).map(Uuid::from_u128).collect(),
                item_count: 10,
                accounted_for: 10,
                withheld: Vec::new(),
            }
        }
    }

    #[tokio::test]
    async fn a_wide_provenance_set_is_sampled_and_its_true_size_reported() {
        let app = router(Arc::new(WidelyDerivedSource));
        read_mind(&app, None).await;

        let disclosure = disclosure_for(&app, None).await;
        assert_eq!(disclosure.supplied, 10);
        assert_eq!(disclosure.accounted_for, 10);
        // The total is never truncated, only the sample is.
        assert_eq!(disclosure.provenance_count, 500);
        assert_eq!(
            disclosure.items.len(),
            cybou_web_contracts::DISCLOSURE_ITEM_SAMPLE
        );
    }

    #[tokio::test]
    async fn a_stranger_is_told_how_much_was_refused_and_never_what_it_was_about() {
        // The subject of a refused concept is that concept's label. Publishing it to explain the
        // refusal would perform the disclosure the refusal prevented, so the surface that reports
        // a filter must not be a way around it.
        let app = router_with_assets_and_session(
            Arc::new(PartlyAccountedSource),
            None,
            SessionContext::public_preview(),
        );
        read_mind(&app, None).await;

        let disclosure = disclosure_for(&app, None).await;
        assert!(!disclosure.subjects_visible);
        assert_eq!(
            disclosure.withheld.len(),
            2,
            "the count of refusals is a fact about the system and is still given"
        );
        assert!(
            disclosure
                .withheld
                .iter()
                .all(|item| item.subject.is_none()),
            "a public reader was named a withheld subject"
        );
        // The grounds are still stated: how much was refused and why are not the person's secrets.
        assert!(
            disclosure
                .withheld
                .iter()
                .any(|item| item.because == "aboveConsumerTrust")
        );
    }

    #[tokio::test]
    async fn the_person_the_record_is_about_is_named_the_subjects() {
        let app = router_with_verifier_and_access(
            Arc::new(PartlyAccountedSource),
            Some(Arc::new(PartlyAccountedSource)),
            Some(Arc::new(OneAccount)),
            None,
            SessionContext::public_preview(),
        );

        let cookie = sign_in(&app, "alice", "hunter2").await.expect("a session");
        read_mind(&app, Some(&cookie)).await;

        let disclosure = disclosure_for(&app, Some(&cookie)).await;
        assert!(disclosure.subjects_visible);
        assert!(
            disclosure
                .withheld
                .iter()
                .any(|item| item.subject.as_deref() == Some("disk-pressure")),
            "the owner could not see what was held back from them"
        );
    }

    #[tokio::test]
    async fn a_delivery_says_what_it_cannot_account_for() {
        // The whole reason two numbers are carried: three items crossed the boundary and only two
        // of them can name where they came from. A surface reporting "two" would be claiming
        // provenance it does not have.
        let app = router(Arc::new(PartlyAccountedSource));
        read_mind(&app, None).await;

        let disclosure = disclosure_for(&app, None).await;
        assert_eq!(disclosure.supplied, 3);
        assert_eq!(disclosure.accounted_for, 2);
        // Never more accounted for than supplied. The first live deployment of this surface
        // reported ten supplied and three thousand accounted for, because the length of the
        // provenance set was being read as a count of items. It is its own number now.
        assert!(disclosure.accounted_for <= disclosure.supplied);
        assert_eq!(disclosure.provenance_count, 2);
        assert_eq!(disclosure.items.len(), 2);
        assert!(disclosure.external_boundary);
        assert!(!disclosure.retains);
    }

    #[tokio::test]
    async fn a_reader_is_shown_their_own_deliveries_and_not_someone_elses() {
        // The record is keyed by consumer. A surface that answered for whoever read last would be
        // a log of what was done to everyone, which is a different and much worse thing.
        let app = router_with_verifier_and_access(
            Arc::new(PartlyAccountedSource),
            None,
            Some(Arc::new(OneAccount)),
            None,
            SessionContext::public_preview(),
        );

        // A stranger reads, and is recorded as the public consumer.
        read_mind(&app, None).await;
        let stranger = disclosure_for(&app, None).await;
        assert!(stranger.delivered);
        assert_eq!(stranger.consumer_id, "living-canvas:public");

        // Someone who signs in is a different consumer, and has been supplied nothing yet.
        let cookie = sign_in(&app, "alice", "hunter2").await.expect("a session");
        let owner = disclosure_for(&app, Some(&cookie)).await;
        assert_eq!(owner.consumer_id, "living-canvas:alice");
        assert!(!owner.delivered);
    }

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

    /// Run one command as the holder of this cookie, and return what the shell said.
    async fn shell_exec_as(app: &Router, cookie: &str, command: &str) -> ShellExecResponse {
        let payload = serde_json::to_vec(&ShellExecRequest {
            command: command.into(),
        })
        .expect("serialize request");
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/shell/exec")
                    .header("content-type", "application/json")
                    .header("cookie", cookie)
                    .body(Body::from(payload))
                    .expect("a request"),
            )
            .await
            .expect("a response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("a body")
            .to_bytes();
        serde_json::from_slice(&body).expect("a shell response")
    }

    #[tokio::test]
    async fn one_session_does_not_move_another_sessions_shell() {
        // Two sign-ins are two seats even when they are the same account, because a shell is where
        // one person is standing. Before shells were split per session, the `cd` below moved the
        // second caller too, and its `pwd` answered for somebody else.
        let (app, _sandbox) = shell_router_over_a_temporary_sandbox();

        let first = sign_in(&app, "alice", "hunter2").await.expect("a session");
        let second = sign_in(&app, "alice", "hunter2").await.expect("a session");
        assert_ne!(first, second, "two sign-ins must be two sessions");

        let moved = shell_exec_as(&app, &first, "cd somewhere").await;
        assert_eq!(moved.exit_code, 0);
        assert_eq!(moved.cwd, "/somewhere");

        let untouched = shell_exec_as(&app, &second, "pwd").await;
        assert_eq!(untouched.stdout.trim(), "/");
        assert_eq!(untouched.cwd, "/");

        // And the session that moved is still where it put itself.
        let still_there = shell_exec_as(&app, &first, "pwd").await;
        assert_eq!(still_there.stdout.trim(), "/somewhere");
    }

    #[tokio::test]
    async fn signing_out_forgets_where_the_session_was_standing() {
        let (app, _sandbox) = shell_router_over_a_temporary_sandbox();

        let cookie = sign_in(&app, "alice", "hunter2").await.expect("a session");
        assert_eq!(
            shell_exec_as(&app, &cookie, "cd somewhere").await.cwd,
            "/somewhere"
        );

        let signed_out = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/logout")
                    .header("cookie", cookie.clone())
                    .body(Body::empty())
                    .expect("a request"),
            )
            .await
            .expect("a response");
        assert_eq!(signed_out.status(), StatusCode::OK);

        // The cookie no longer names a session, so the shell surface refuses rather than handing
        // the caller whatever shell that token used to own.
        let payload = serde_json::to_vec(&ShellExecRequest {
            command: "pwd".into(),
        })
        .expect("serialize request");
        let refused = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/shell/exec")
                    .header("content-type", "application/json")
                    .header("cookie", cookie)
                    .body(Body::from(payload))
                    .expect("a request"),
            )
            .await
            .expect("a response");
        assert_eq!(refused.status(), StatusCode::FORBIDDEN);
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
