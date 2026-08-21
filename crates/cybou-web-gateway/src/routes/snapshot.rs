// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Snapshot and Mind atomic projection route handlers.

use axum::{
    Json,
    extract::State,
    http::HeaderMap,
};
use cybou_web_contracts::{MindProjection, SnapshotProjection};

use crate::state::{GatewayError, GatewayState, SNAPSHOT_BUDGET};

/// Return one atomic snapshot projection.
pub async fn snapshot_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<SnapshotProjection>, GatewayError> {
    tokio::time::timeout(SNAPSHOT_BUDGET, state.source_for(&headers).snapshot())
        .await
        .map_err(|_| GatewayError::Timeout)?
        .map(Json)
}

/// Return the full Mind organ system state.
pub async fn mind_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<MindProjection>, GatewayError> {
    let session = state.session_for(&headers);
    let source = state.source_for(&headers);
    let projection = tokio::time::timeout(SNAPSHOT_BUDGET, source.mind())
        .await
        .map_err(|_| GatewayError::Timeout)??;

    state
        .record_delivery(
            &GatewayState::destination_for(session.as_ref()),
            &source.last_delivery(),
        )
        .await;
    Ok(Json(projection))
}
