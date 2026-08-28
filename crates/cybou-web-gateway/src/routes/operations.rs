// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! HTTP endpoints for querying and managing server operations.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use cybou_web_contracts::{OperationCancelRequest, OperationLogsProjection, OperationsListProjection};
use uuid::Uuid;

use crate::state::{GatewayError, GatewayState};

/// GET `/api/v1/operations`
pub async fn list_operations(
    State(state): State<GatewayState>,
) -> Result<Json<OperationsListProjection>, GatewayError> {
    Ok(Json(state.operations.list()))
}

/// GET `/api/v1/operations/{id}`
pub async fn get_operation(
    State(state): State<GatewayState>,
    Path(id): Path<Uuid>,
) -> Result<Json<cybou_protocol::operation::OperationRecord>, GatewayError> {
    state
        .operations
        .get(id)
        .map(Json)
        .ok_or(GatewayError::NotFound)
}

/// GET `/api/v1/operations/{id}/logs`
pub async fn get_operation_logs(
    State(state): State<GatewayState>,
    Path(id): Path<Uuid>,
) -> Result<Json<OperationLogsProjection>, GatewayError> {
    Ok(Json(state.operations.get_logs(id)))
}

/// POST `/api/v1/operations/cancel`
pub async fn cancel_operation(
    State(state): State<GatewayState>,
    Json(request): Json<OperationCancelRequest>,
) -> Result<StatusCode, GatewayError> {
    state.operations.cancel(request.operation_id, request.reason)?;
    Ok(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use crate::operations_hub::OperationsHub;

    #[test]
    fn operations_hub_tracks_progress_and_logs() {
        let hub = OperationsHub::new();
        let list = hub.list();
        assert!(list.active_count >= 1);
        let first_op = &list.operations[0];
        let op_id = first_op.id;

        hub.append_log(op_id, "stdout", "Test log output");
        let logs = hub.get_logs(op_id);
        assert!(logs.logs.iter().any(|l| l.text == "Test log output"));

        assert!(hub.complete(op_id).is_ok());
        let updated = hub.get(op_id).expect("operation exists");
        assert_eq!(updated.state, cybou_protocol::operation::OperationState::Completed);
    }
}
