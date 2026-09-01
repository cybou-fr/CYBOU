// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! HTTP endpoints for querying and managing server operations.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
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
    if state
        .session_for(&headers)
        .and_then(|session| session.uid)
        .is_none()
    {
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
    if state
        .session_for(&headers)
        .and_then(|session| session.uid)
        .is_none()
    {
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
    if state
        .session_for(&headers)
        .and_then(|session| session.uid)
        .is_none()
    {
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
    if state
        .session_for(&headers)
        .and_then(|session| session.uid)
        .is_none()
    {
        return Err(GatewayError::Refused);
    }
    let _ = request.reason;
    state.operations.cancel(request.operation_id).await?;
    Ok(StatusCode::OK)
}
