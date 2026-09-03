// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Stateless HTTP-boundary client for Operation1.

use crate::state::GatewayError;
use cybou_protocol::operation::{CancelOutcome, OperationLogEntry, OperationRecord};
use cybou_web_contracts::{OperationLogsProjection, OperationsListProjection, WEB_SCHEMA_V1};
use uuid::Uuid;

/// Holds no operation lifecycle, progress, logs, or cancellation state.
#[derive(Debug, Default)]
pub struct OperationsHub;

impl OperationsHub {
    /// Create a stateless proxy.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    #[cfg(target_os = "linux")]
    async fn call(
        &self,
        method: &str,
        body: &(impl serde::Serialize + zbus::zvariant::DynamicType),
    ) -> Result<Vec<u8>, GatewayError> {
        zbus::Connection::session()
            .await
            .map_err(|_| GatewayError::Unavailable)?
            .call_method(
                Some(cybou_fabric::OPERATION.service),
                cybou_fabric::OPERATION.object_path,
                Some(cybou_fabric::OPERATION.interface),
                method,
                body,
            )
            .await
            .map_err(|_| GatewayError::Unavailable)?
            .body()
            .deserialize()
            .map_err(|_| GatewayError::InvalidProjection)
    }

    /// List canonical operations.
    pub async fn list(&self) -> Result<OperationsListProjection, GatewayError> {
        #[cfg(target_os = "linux")]
        {
            let operations: Vec<OperationRecord> =
                cybou_fabric::decode(&self.call("List", &()).await?)
                    .map_err(|_| GatewayError::InvalidProjection)?;
            let active_count = operations.iter().filter(|v| !v.state.is_terminal()).count();
            Ok(OperationsListProjection {
                schema_version: WEB_SCHEMA_V1,
                active_count,
                operations,
            })
        }
        #[cfg(not(target_os = "linux"))]
        Err(GatewayError::Unavailable)
    }

    /// Get one canonical operation.
    pub async fn get(&self, id: Uuid) -> Result<OperationRecord, GatewayError> {
        #[cfg(target_os = "linux")]
        {
            let bytes = self.call("Get", &(id.to_string(),)).await?;
            if bytes.is_empty() {
                return Err(GatewayError::NotFound);
            }
            cybou_fabric::decode(&bytes).map_err(|_| GatewayError::InvalidProjection)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            Err(GatewayError::Unavailable)
        }
    }

    /// Get owner-held logs.
    pub async fn get_logs(&self, id: Uuid) -> Result<OperationLogsProjection, GatewayError> {
        #[cfg(target_os = "linux")]
        {
            let logs: Vec<OperationLogEntry> =
                cybou_fabric::decode(&self.call("Logs", &(id.to_string(),)).await?)
                    .map_err(|_| GatewayError::InvalidProjection)?;
            Ok(OperationLogsProjection {
                schema_version: WEB_SCHEMA_V1,
                operation_id: id,
                logs,
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            Err(GatewayError::Unavailable)
        }
    }

    /// Ask Operation1 to signal the real worker's cancellation token.
    ///
    /// Returns the owner's distinction between an accepted request and a confirmed teardown; the
    /// gateway never upgrades one into the other.
    pub async fn cancel(&self, id: Uuid) -> Result<CancelOutcome, GatewayError> {
        #[cfg(target_os = "linux")]
        {
            let result: CancelOutcome =
                cybou_fabric::decode(&self.call("Cancel", &(id.to_string(),)).await?)
                    .map_err(|_| GatewayError::InvalidProjection)?;
            match result {
                outcome @ (CancelOutcome::CancellationAccepted
                | CancelOutcome::CancellationConfirmed) => Ok(outcome),
                CancelOutcome::NotFound => Err(GatewayError::NotFound),
                CancelOutcome::Conflict => Err(GatewayError::Conflict),
                CancelOutcome::Refused => Err(GatewayError::Refused),
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = id;
            Err(GatewayError::Unavailable)
        }
    }
}
