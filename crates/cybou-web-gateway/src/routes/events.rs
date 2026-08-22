// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Server-Sent Events (SSE) live projection stream.

use std::{convert::Infallible, time::Duration};

use axum::{
    extract::State,
    http::HeaderMap,
    response::sse::{Event, KeepAlive, Sse},
};
use cybou_web_contracts::WEB_SCHEMA_V1;

use axum::response::IntoResponse;

use crate::state::{
    EVENT_POLL_INTERVAL, GatewayError, GatewayState, MAX_CURSOR_BYTES, SNAPSHOT_BUDGET,
};

/// Extract and validate resume cursor from request headers.
///
/// # Errors
///
/// Returns [`GatewayError::InvalidCursor`] if the header is malformed, contains control characters, or exceeds size limits.
pub fn resume_cursor(headers: &HeaderMap) -> Result<Option<String>, GatewayError> {
    let Some(value) = headers.get("last-event-id") else {
        return Ok(None);
    };
    let value = value.to_str().map_err(|_| GatewayError::InvalidCursor)?;
    if value.is_empty() || value.len() > MAX_CURSOR_BYTES || value.chars().any(char::is_control) {
        return Err(GatewayError::InvalidCursor);
    }
    Ok(Some(value.to_owned()))
}

/// SSE stream handler emitting snapshot events and projection errors.
///
/// # Errors
///
/// Returns [`GatewayError`] if initial cursor parsing fails.
pub async fn events_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<
    Sse<impl futures_core::Stream<Item = Result<Event, Infallible>>>,
    axum::response::Response,
> {
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required().into_response());
    }
    let initial_cursor = resume_cursor(&headers).map_err(IntoResponse::into_response)?;
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
