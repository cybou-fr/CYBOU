// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Snapshot and Mind atomic projection route handlers.

use axum::{Json, extract::State, http::HeaderMap, response::IntoResponse};
use cybou_web_contracts::{MindProjection, SnapshotProjection};

use crate::state::{GatewayError, GatewayState, SNAPSHOT_BUDGET};

/// Return one atomic snapshot projection.
///
/// # Errors
///
/// Returns [`GatewayError`] if snapshot production times out or fails.
pub async fn snapshot_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<SnapshotProjection>, axum::response::Response> {
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required().into_response());
    }
    tokio::time::timeout(SNAPSHOT_BUDGET, state.source_for(&headers).snapshot())
        .await
        .map_err(|_| GatewayError::Timeout.into_response())?
        .map(Json)
        .map_err(IntoResponse::into_response)
}

/// Return the full Mind organ system state.
///
/// # Errors
///
/// Returns [`GatewayError`] if mind production times out or fails.
pub async fn mind_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<MindProjection>, axum::response::Response> {
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required().into_response());
    }
    let session = state.session_for(&headers);
    let source = state.source_for(&headers);
    let projection = tokio::time::timeout(SNAPSHOT_BUDGET, source.mind())
        .await
        .map_err(|_| GatewayError::Timeout.into_response())?
        .map_err(IntoResponse::into_response)?;

    state
        .record_delivery(
            &GatewayState::destination_for(session.as_ref()),
            &source.last_delivery(),
        )
        .await;
    Ok(Json(projection))
}
