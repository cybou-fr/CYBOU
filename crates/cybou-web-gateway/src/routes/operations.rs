// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! HTTP endpoints for querying and managing server operations.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use cybou_web_contracts::{
    OperationCancelRequest, OperationLogsProjection, OperationsListProjection,
};
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
    state
        .operations
        .cancel(request.operation_id, request.reason)?;
    Ok(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use crate::operations_hub::OperationsHub;
    use cybou_protocol::{
        action::Proposer,
        operation::{OperationKind, OperationProgress, OperationRecord, OperationState},
    };
    use time::OffsetDateTime;
    use uuid::Uuid;

    #[test]
    fn operations_hub_tracks_progress_and_logs() {
        let hub = OperationsHub::new();
        let list = hub.list();
        assert_eq!(list.active_count, 0);

        let op_id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();
        hub.register(OperationRecord {
            id: op_id,
            kind: OperationKind::IndexWorkspace,
            state: OperationState::Running,
            label: "Test Task".to_owned(),
            initiator: Proposer::Mind,
            subject: None,
            progress: OperationProgress {
                percent: Some(50.0),
                step: "Processing".to_owned(),
                total_steps: Some(2),
                current_step: Some(1),
                detail: None,
            },
            cancellable: true,
            started_at: now,
            updated_at: now,
            finished_at: None,
        });

        let list_after = hub.list();
        assert_eq!(list_after.active_count, 1);

        hub.append_log(op_id, "stdout", "Test log output");
        let logs = hub.get_logs(op_id);
        assert!(logs.logs.iter().any(|l| l.text == "Test log output"));

        assert!(hub.complete(op_id).is_ok());
        let updated = hub.get(op_id).expect("operation exists");
        assert_eq!(updated.state, OperationState::Completed);
    }
}
