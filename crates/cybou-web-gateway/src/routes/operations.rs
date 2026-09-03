// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! HTTP endpoints for querying and managing server operations.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use cybou_protocol::operation::CancelOutcome;
use cybou_web_contracts::{
    OperationCancelRequest, OperationLogsProjection, OperationsListProjection,
};
use uuid::Uuid;

use crate::state::{GatewayError, GatewayState};

/// GET `/api/v1/operations`
pub async fn list_operations(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<OperationsListProjection>, GatewayError> {
    if !state.has_private_seat(&headers) {
        return Err(GatewayError::Refused);
    }
    Ok(Json(state.operations.list().await?))
}

/// GET `/api/v1/operations/{id}`
pub async fn get_operation(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<cybou_protocol::operation::OperationRecord>, GatewayError> {
    if !state.has_private_seat(&headers) {
        return Err(GatewayError::Refused);
    }
    Ok(Json(state.operations.get(id).await?))
}

/// GET `/api/v1/operations/{id}/logs`
pub async fn get_operation_logs(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<OperationLogsProjection>, GatewayError> {
    if !state.has_private_seat(&headers) {
        return Err(GatewayError::Refused);
    }
    Ok(Json(state.operations.get_logs(id).await?))
}

/// POST `/api/v1/operations/cancel`
pub async fn cancel_operation(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<OperationCancelRequest>,
) -> Result<StatusCode, GatewayError> {
    if !state.has_private_seat(&headers) {
        return Err(GatewayError::Refused);
    }
    let _ = request.reason;
    // 202 says the request was recorded and signalled; only the worker may publish Cancelled.
    // 200 is reserved for a teardown the executing authority already confirmed.
    match state.operations.cancel(request.operation_id).await? {
        CancelOutcome::CancellationConfirmed => Ok(StatusCode::OK),
        _ => Ok(StatusCode::ACCEPTED),
    }
}
