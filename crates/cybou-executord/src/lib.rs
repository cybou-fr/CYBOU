// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The Body side of ADR-0022: typed execution without authorization policy.

use async_trait::async_trait;
use cybou_protocol::action::{
    AttemptReport, BodyReading, ExecutableAction, ExecutionAttempt, ExecutionPermit,
};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub mod service;

/// Executor transport or adapter failure.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ExecutorError {
    /// Action1 did not yield the named single-use capability.
    #[error("Action1 refused the permit: {0}")]
    PermitRefused(String),
    /// A typed Body adapter failed.
    #[error("Body adapter failed: {0}")]
    Adapter(String),
}

/// The one authority question the executor is allowed to ask.
#[async_trait]
pub trait PermitSource: Send + Sync {
    /// Atomically claim the complete action for an opaque permit identity.
    async fn claim(&self, permit_id: Uuid) -> Result<ExecutionPermit, ExecutorError>;
}

/// The three physical adapters. There is intentionally no program-and-arguments method.
#[async_trait]
pub trait Body: Send + Sync {
    /// Read one concrete service's state.
    async fn service_status(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError>;
    /// Delete the fixed package archive cache.
    async fn clean_package_cache(&self) -> Result<Vec<BodyReading>, ExecutorError>;
    /// Restart one concrete service.
    async fn restart_service(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError>;
}

/// Claim and carry out exactly one permit.
///
/// # Errors
///
/// Returns a transport refusal before an attempt exists. Adapter errors are retained as the
/// attempt's typed report, because an operation that started and failed is still an attempt.
pub async fn execute(
    permits: &impl PermitSource,
    body: &impl Body,
    permit_id: Uuid,
    now: OffsetDateTime,
) -> Result<ExecutionAttempt, ExecutorError> {
    let permit = permits.claim(permit_id).await?;
    let operation = match &permit.action {
        ExecutableAction::ServiceStatus { .. } => "service.status",
        ExecutableAction::PackageCacheClean => "package.cache.clean",
        ExecutableAction::ServiceRestart { .. } => "service.restart",
    }
    .to_owned();
    let target_resource = match &permit.action {
        ExecutableAction::ServiceStatus { unit } | ExecutableAction::ServiceRestart { unit } => {
            format!("systemd:{unit}")
        }
        ExecutableAction::PackageCacheClean => "apt:archives".to_owned(),
    };
    let result = match &permit.action {
        ExecutableAction::ServiceStatus { unit } => body.service_status(unit).await,
        ExecutableAction::PackageCacheClean => body.clean_package_cache().await,
        ExecutableAction::ServiceRestart { unit } => body.restart_service(unit).await,
    };
    let (report, body_readings) = match result {
        Ok(readings) => (AttemptReport::Completed, readings),
        Err(error) => (
            AttemptReport::Failed {
                because: error.to_string(),
            },
            Vec::new(),
        ),
    };
    Ok(ExecutionAttempt {
        attempt_id: Uuid::new_v4(),
        proposal_id: permit.proposal_id,
        decision_id: permit.decision_id,
        operation,
        target_resource,
        report,
        body_readings,
        started_at: now,
        ended_at: Some(OffsetDateTime::now_utc()),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct OnePermit(Mutex<Option<ExecutionPermit>>);

    #[async_trait]
    impl PermitSource for OnePermit {
        async fn claim(&self, _: Uuid) -> Result<ExecutionPermit, ExecutorError> {
            self.0
                .lock()
                .expect("permit lock")
                .take()
                .ok_or_else(|| ExecutorError::PermitRefused("consumed".to_owned()))
        }
    }

    #[derive(Default)]
    struct RecordingBody(Mutex<Vec<String>>);

    #[async_trait]
    impl Body for RecordingBody {
        async fn service_status(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError> {
            self.0
                .lock()
                .expect("body lock")
                .push(format!("status:{unit}"));
            Ok(vec![BodyReading {
                field: "systemd.active-state".to_owned(),
                value: "active".to_owned(),
            }])
        }
        async fn clean_package_cache(&self) -> Result<Vec<BodyReading>, ExecutorError> {
            self.0.lock().expect("body lock").push("clean".to_owned());
            Ok(Vec::new())
        }
        async fn restart_service(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError> {
            self.0
                .lock()
                .expect("body lock")
                .push(format!("restart:{unit}"));
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn caller_supplies_only_identity_and_permit_supplies_the_action() {
        let id = Uuid::new_v4();
        let permits = OnePermit(Mutex::new(Some(ExecutionPermit {
            permit_id: id,
            decision_id: Uuid::new_v4(),
            proposal_id: Uuid::new_v4(),
            action: ExecutableAction::ServiceRestart {
                unit: "cybou-action-test.service".to_owned(),
            },
            issued_at: OffsetDateTime::UNIX_EPOCH,
            expires_at: OffsetDateTime::UNIX_EPOCH + time::Duration::minutes(1),
        })));
        let body = RecordingBody::default();
        let attempt = execute(&permits, &body, id, OffsetDateTime::UNIX_EPOCH)
            .await
            .expect("execute");
        assert_eq!(attempt.operation, "service.restart");
        assert_eq!(
            body.0.lock().expect("body lock").as_slice(),
            ["restart:cybou-action-test.service"]
        );
        assert!(
            execute(&permits, &body, id, OffsetDateTime::UNIX_EPOCH)
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn service_status_returns_the_state_the_adapter_read() {
        let id = Uuid::new_v4();
        let permits = OnePermit(Mutex::new(Some(ExecutionPermit {
            permit_id: id,
            decision_id: Uuid::new_v4(),
            proposal_id: Uuid::new_v4(),
            action: ExecutableAction::ServiceStatus {
                unit: "cybou-action-test.service".to_owned(),
            },
            issued_at: OffsetDateTime::UNIX_EPOCH,
            expires_at: OffsetDateTime::UNIX_EPOCH + time::Duration::minutes(1),
        })));
        let attempt = execute(
            &permits,
            &RecordingBody::default(),
            id,
            OffsetDateTime::UNIX_EPOCH,
        )
        .await
        .expect("execute");
        assert_eq!(
            attempt.body_readings,
            [BodyReading {
                field: "systemd.active-state".to_owned(),
                value: "active".to_owned(),
            }]
        );
    }
}
