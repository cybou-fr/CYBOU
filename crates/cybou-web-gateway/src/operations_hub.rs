// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Operations Hub for tracking and managing server-owned asynchronous tasks.

use std::{
    collections::HashMap,
    sync::RwLock,
};
use cybou_protocol::operation::{
    OperationLogEntry, OperationProgress, OperationRecord, OperationState,
};
use cybou_web_contracts::{OperationLogsProjection, OperationsListProjection, WEB_SCHEMA_V1};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::state::GatewayError;

/// Maximum log lines retained per operation in memory.
const MAX_LOG_LINES_PER_OP: usize = 500;

/// Maximum completed operations retained in history.
const MAX_HISTORY_OPS: usize = 100;

/// Server-side hub for tracking active and historical operations.
#[derive(Debug)]
pub struct OperationsHub {
    operations: RwLock<Vec<OperationRecord>>,
    logs: RwLock<HashMap<Uuid, Vec<OperationLogEntry>>>,
}

impl Default for OperationsHub {
    fn default() -> Self {
        Self {
            operations: RwLock::new(Vec::new()),
            logs: RwLock::new(HashMap::new()),
        }
    }
}

impl OperationsHub {
    /// Create a new empty OperationsHub.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// List all active and historical operations.
    #[must_use]
    pub fn list(&self) -> OperationsListProjection {
        let ops = self.operations.read().expect("read operations").clone();
        let active_count = ops
            .iter()
            .filter(|op| !op.state.is_terminal())
            .count();
        OperationsListProjection {
            schema_version: WEB_SCHEMA_V1,
            active_count,
            operations: ops,
        }
    }

    /// Retrieve an operation by ID.
    #[must_use]
    pub fn get(&self, id: Uuid) -> Option<OperationRecord> {
        self.operations
            .read()
            .expect("read operations")
            .iter()
            .find(|op| op.id == id)
            .cloned()
    }

    /// Retrieve logs for an operation.
    #[must_use]
    pub fn get_logs(&self, id: Uuid) -> OperationLogsProjection {
        let logs = self
            .logs
            .read()
            .expect("read logs")
            .get(&id)
            .cloned()
            .unwrap_or_default();
        OperationLogsProjection {
            schema_version: WEB_SCHEMA_V1,
            operation_id: id,
            logs,
        }
    }

    /// Register a new operation.
    pub fn register(&self, op: OperationRecord) {
        let id = op.id;
        let mut ops = self.operations.write().expect("write operations");
        ops.insert(0, op);
        if ops.len() > MAX_HISTORY_OPS {
            ops.truncate(MAX_HISTORY_OPS);
        }
        let mut logs = self.logs.write().expect("write logs");
        logs.entry(id).or_default();
    }

    /// Update progress on an active operation.
    pub fn update_progress(&self, id: Uuid, progress: OperationProgress) -> Result<(), GatewayError> {
        let mut ops = self.operations.write().expect("write operations");
        let op = ops
            .iter_mut()
            .find(|op| op.id == id)
            .ok_or(GatewayError::NotFound)?;
        op.progress = progress;
        op.updated_at = OffsetDateTime::now_utc();
        Ok(())
    }

    /// Append log output to an operation.
    pub fn append_log(&self, id: Uuid, stream: &str, text: &str) {
        let mut logs = self.logs.write().expect("write logs");
        let list = logs.entry(id).or_default();
        list.push(OperationLogEntry {
            timestamp: OffsetDateTime::now_utc(),
            stream: stream.to_owned(),
            text: text.to_owned(),
        });
        if list.len() > MAX_LOG_LINES_PER_OP {
            list.remove(0);
        }
    }

    /// Mark an operation as completed.
    pub fn complete(&self, id: Uuid) -> Result<(), GatewayError> {
        let mut ops = self.operations.write().expect("write operations");
        let op = ops
            .iter_mut()
            .find(|op| op.id == id)
            .ok_or(GatewayError::NotFound)?;
        op.state = OperationState::Completed;
        op.progress.percent = Some(100.0);
        op.updated_at = OffsetDateTime::now_utc();
        op.finished_at = Some(OffsetDateTime::now_utc());
        Ok(())
    }

    /// Mark an operation as failed.
    pub fn fail(&self, id: Uuid, error: String) -> Result<(), GatewayError> {
        let mut ops = self.operations.write().expect("write operations");
        let op = ops
            .iter_mut()
            .find(|op| op.id == id)
            .ok_or(GatewayError::NotFound)?;
        op.state = OperationState::Failed { error: error.clone() };
        op.updated_at = OffsetDateTime::now_utc();
        op.finished_at = Some(OffsetDateTime::now_utc());
        drop(ops);
        self.append_log(id, "stderr", &format!("Operation failed: {error}"));
        Ok(())
    }

    /// Cancel a running operation.
    pub fn cancel(&self, id: Uuid, reason: Option<String>) -> Result<(), GatewayError> {
        let mut ops = self.operations.write().expect("write operations");
        let op = ops
            .iter_mut()
            .find(|op| op.id == id)
            .ok_or(GatewayError::NotFound)?;
        if op.state.is_terminal() {
            return Err(GatewayError::Conflict);
        }
        if !op.cancellable {
            return Err(GatewayError::Refused);
        }
        op.state = OperationState::Cancelled;
        op.updated_at = OffsetDateTime::now_utc();
        op.finished_at = Some(OffsetDateTime::now_utc());
        drop(ops);
        let msg = reason.unwrap_or_else(|| "Operation cancelled by user".to_owned());
        self.append_log(id, "system", &msg);
        Ok(())
    }
}
