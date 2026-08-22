// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Authentication routes: session inspection, login, and logout.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use cybou_web_contracts::{SessionMode, SessionProjection, WEB_SCHEMA_V1};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::access::{self, LoginOutcome, LoginRequest};
use crate::shells::ShellOwner;
use crate::state::{ErrorBody, GatewayState};

/// Read back active session projection.
pub async fn session_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Json<SessionProjection> {
    let mut projection = state.session.clone();
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

/// Establish a session for an authenticated Linux account.
pub async fn login_handler(
    State(state): State<GatewayState>,
    Json(request): Json<LoginRequest>,
) -> impl IntoResponse {
    let authenticated = match &state.verifier {
        Some(verifier) => verifier.verify(&request.username, &request.password).await,
        None => false,
    };
    let username = request.username.clone();
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

    let Some(token) = state.sessions.begin(&username, OffsetDateTime::now_utc()) else {
        // Authentication succeeded and the session did not. Saying so plainly, and retryably, is
        // the only honest answer: the alternative was a token that was not unpredictable.
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody {
                schema_version: WEB_SCHEMA_V1,
                error: "sessionCouldNotBeIssued",
                retryable: true,
            }),
        )
            .into_response();
    };
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

/// End the session this request carries.
pub async fn logout_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Some(token) = headers
        .get(axum::http::header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(access::token_in)
    {
        state.sessions.end(token);
        // Every shell the session opened goes with it, not the one that happens to be numbered
        // zero. Leaving them behind would keep working directories for a token nobody holds, and
        // hand them back if that token were ever issued again.
        state.shells.end_seat(&ShellOwner::Session {
            session: access::digest(token),
            instance: 0,
        });
    }
    (
        StatusCode::OK,
        [(axum::http::header::SET_COOKIE, access::cleared_cookie())],
    )
}
