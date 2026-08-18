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
    routing::{any, get},
};
use cybou_web_contracts::{SessionMode, SessionProjection, SnapshotProjection, WEB_SCHEMA_V1};
use serde::Serialize;
use thiserror::Error;
use time::{Duration as TimeDuration, OffsetDateTime, format_description::well_known::Rfc3339};
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};
use uuid::Uuid;

pub mod fixture;
#[cfg(target_os = "linux")]
pub mod presence_zbus;

/// Maximum time the gateway permits one Presence projection request to occupy.
pub const SNAPSHOT_BUDGET: Duration = Duration::from_millis(1_500);

/// Interval used until Presence exposes a native changed-event subscription.
pub const EVENT_POLL_INTERVAL: Duration = Duration::from_secs(2);

const MAX_CURSOR_BYTES: usize = 256;

/// Typed read-only source behind the HTTP boundary.
#[async_trait]
pub trait PresenceSource: Send + Sync + 'static {
    /// Return one atomic, presentation-ready snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError`] when transport, decoding, or owner availability prevents a typed
    /// projection. It must never turn those failures into an empty successful snapshot.
    async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError>;
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
    session: SessionProjection,
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
            consumer_id: "living-canvas:local-desktop".into(),
        }
    }

    /// Unauthenticated context permitted only for deterministic, non-personal fixtures.
    #[must_use]
    pub fn public_preview() -> Self {
        Self {
            mode: SessionMode::PublicPreview,
            consumer_id: "living-canvas:public-preview".into(),
        }
    }
}

/// Build the read-only v1 router around a typed Presence source.
pub fn router(presence: Arc<dyn PresenceSource>) -> Router {
    router_with_assets(presence, None)
}

/// Build the read-only v1 router and optionally serve a Living Canvas build from the same origin.
pub fn router_with_assets(presence: Arc<dyn PresenceSource>, web_root: Option<PathBuf>) -> Router {
    router_with_assets_and_session(presence, web_root, SessionContext::local_desktop())
}

/// Build the v1 router with an explicit server-established trust context.
pub fn router_with_assets_and_session(
    presence: Arc<dyn PresenceSource>,
    web_root: Option<PathBuf>,
    session_context: SessionContext,
) -> Router {
    let now = OffsetDateTime::now_utc();
    let expires_at = (now + TimeDuration::hours(8))
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into());
    let state = GatewayState {
        presence,
        session: SessionProjection {
            schema_version: WEB_SCHEMA_V1,
            session_id: Uuid::new_v4(),
            mode: session_context.mode,
            consumer_id: session_context.consumer_id,
            expires_at,
        },
    };

    let app = Router::new()
        .route("/api/v1/session", get(session))
        .route("/api/v1/snapshot", get(snapshot))
        .route("/api/v1/events", get(events))
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

async fn session(State(state): State<GatewayState>) -> Json<SessionProjection> {
    Json(state.session)
}

async fn snapshot(
    State(state): State<GatewayState>,
) -> Result<Json<SnapshotProjection>, GatewayError> {
    tokio::time::timeout(SNAPSHOT_BUDGET, state.presence.snapshot())
        .await
        .map_err(|_| GatewayError::Timeout)?
        .map(Json)
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
                tokio::time::sleep(EVENT_POLL_INTERVAL).await;
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
    use std::{sync::Arc, time::Duration};

    use async_trait::async_trait;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use cybou_web_contracts::{SessionMode, SessionProjection, SnapshotProjection};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::{
        GatewayError, PresenceSource, SessionContext, router, router_with_assets,
        router_with_assets_and_session,
    };
    use crate::fixture::FixturePresenceSource;

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
        assert_eq!(session.consumer_id, "living-canvas:public-preview");
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
}
