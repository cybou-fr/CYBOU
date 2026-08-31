// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The Body side of ADR-0022: typed execution without authorization policy.

use async_trait::async_trait;
use cybou_protocol::action::{
    AttemptReport, BodyReading, ExecutableAction, ExecutionAttempt, ExecutionClaim,
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
    /// Action1 could not retain the executor's final report.
    #[error("Action1 did not retain the execution report: {0}")]
    ReportNotRecorded(String),
}

/// The one authority question the executor is allowed to ask.
#[async_trait]
pub trait PermitSource: Send + Sync {
    /// Atomically claim the complete action for an opaque permit identity.
    async fn claim(&self, permit_id: Uuid) -> Result<ExecutionClaim, ExecutorError>;
    /// Put the executor's final account beside the authorization that produced it.
    async fn record_attempt(&self, attempt: &ExecutionAttempt) -> Result<(), ExecutorError>;
}

/// The physical adapters, one method each. There is intentionally no program-and-arguments method:
/// a trait that could be handed a command line would be a shell with extra steps, and every check
/// above it would be checking a string.
#[async_trait]
pub trait Body: Send + Sync {
    /// Read one concrete service's state.
    async fn service_status(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError>;
    /// Delete the fixed package archive cache.
    async fn clean_package_cache(&self) -> Result<Vec<BodyReading>, ExecutorError>;
    /// Restart one concrete service.
    async fn restart_service(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError>;
    /// Start one concrete service that is not running.
    async fn start_service(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError>;
    /// Stop one concrete service that is.
    async fn stop_service(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError>;
    /// Ask one concrete service to re-read its configuration.
    async fn reload_service(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError>;
}

/// Send one typed action to the one adapter that performs it.
///
/// Separate from [`execute`] so the mapping can be exercised on its own. Four of these differ by a
/// single word to systemd, and a dispatch that sent two of them to the same adapter would be a Stop
/// button that restarts — which looks, to the person who pressed it, exactly like it worked.
async fn perform(
    body: &impl Body,
    action: &ExecutableAction,
) -> Result<Vec<BodyReading>, ExecutorError> {
    match action {
        ExecutableAction::ServiceStatus { unit } => body.service_status(unit).await,
        ExecutableAction::PackageCacheClean => body.clean_package_cache().await,
        ExecutableAction::ServiceRestart { unit } => body.restart_service(unit).await,
        ExecutableAction::ServiceStart { unit } => body.start_service(unit).await,
        ExecutableAction::ServiceStop { unit } => body.stop_service(unit).await,
        ExecutableAction::ServiceReload { unit } => body.reload_service(unit).await,
    }
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
) -> Result<ExecutionAttempt, ExecutorError> {
    let claim = permits.claim(permit_id).await?;
    let result = perform(body, &claim.permit.action).await;
    let (report, body_readings) = match result {
        Ok(readings) => (AttemptReport::Completed, readings),
        Err(error) => (
            AttemptReport::Failed {
                because: error.to_string(),
            },
            Vec::new(),
        ),
    };
    let attempt = claim
        .started
        .finish(report, body_readings, Some(OffsetDateTime::now_utc()));
    permits.record_attempt(&attempt).await?;
    Ok(attempt)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use cybou_protocol::action::{ExecutionPermit, ExecutionStarted};
    use time::OffsetDateTime;

    struct OnePermit {
        claim: Mutex<Option<ExecutionClaim>>,
        reports: Mutex<Vec<ExecutionAttempt>>,
        lose_report: bool,
    }

    fn one_permit(id: Uuid, action: ExecutableAction) -> OnePermit {
        let permit = ExecutionPermit {
            permit_id: id,
            decision_id: Uuid::new_v4(),
            proposal_id: Uuid::new_v4(),
            action,
            issued_at: OffsetDateTime::UNIX_EPOCH,
            expires_at: OffsetDateTime::UNIX_EPOCH + time::Duration::minutes(1),
        };
        let started =
            ExecutionStarted::from_permit(&permit, Uuid::new_v4(), OffsetDateTime::UNIX_EPOCH);
        OnePermit {
            claim: Mutex::new(Some(ExecutionClaim { permit, started })),
            reports: Mutex::new(Vec::new()),
            lose_report: false,
        }
    }

    #[async_trait]
    impl PermitSource for OnePermit {
        async fn claim(&self, _: Uuid) -> Result<ExecutionClaim, ExecutorError> {
            self.claim
                .lock()
                .expect("permit lock")
                .take()
                .ok_or_else(|| ExecutorError::PermitRefused("consumed".to_owned()))
        }

        async fn record_attempt(&self, attempt: &ExecutionAttempt) -> Result<(), ExecutorError> {
            if self.lose_report {
                return Err(ExecutorError::ReportNotRecorded("reply lost".to_owned()));
            }
            self.reports
                .lock()
                .expect("report lock")
                .push(attempt.clone());
            Ok(())
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
        async fn start_service(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError> {
            self.0
                .lock()
                .expect("body lock")
                .push(format!("start:{unit}"));
            Ok(Vec::new())
        }
        async fn stop_service(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError> {
            self.0
                .lock()
                .expect("body lock")
                .push(format!("stop:{unit}"));
            Ok(Vec::new())
        }
        async fn reload_service(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError> {
            self.0
                .lock()
                .expect("body lock")
                .push(format!("reload:{unit}"));
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn every_action_reaches_the_adapter_it_names_and_no_other() {
        // Four unit operations that differ only in one word to systemd. A dispatch that sent two of
        // them to the same adapter would be a Stop button that restarts, which looks like it worked.
        for (action, expected) in [
            (
                ExecutableAction::ServiceRestart {
                    unit: "a.service".to_owned(),
                },
                "restart:a.service",
            ),
            (
                ExecutableAction::ServiceStart {
                    unit: "b.service".to_owned(),
                },
                "start:b.service",
            ),
            (
                ExecutableAction::ServiceStop {
                    unit: "c.service".to_owned(),
                },
                "stop:c.service",
            ),
            (
                ExecutableAction::ServiceReload {
                    unit: "d.service".to_owned(),
                },
                "reload:d.service",
            ),
        ] {
            let body = RecordingBody::default();
            perform(&body, &action).await.expect("the adapter runs");
            assert_eq!(
                body.0.lock().expect("body lock").as_slice(),
                [expected.to_owned()],
                "{action:?} reached the wrong adapter"
            );
        }
    }

    #[tokio::test]
    async fn caller_supplies_only_identity_and_permit_supplies_the_action() {
        let id = Uuid::new_v4();
        let permits = one_permit(
            id,
            ExecutableAction::ServiceRestart {
                unit: "cybou-action-test.service".to_owned(),
            },
        );
        let body = RecordingBody::default();
        let attempt = execute(&permits, &body, id).await.expect("execute");
        assert_eq!(attempt.operation, "service.restart");
        assert_eq!(
            body.0.lock().expect("body lock").as_slice(),
            ["restart:cybou-action-test.service"]
        );
        assert_eq!(
            permits.reports.lock().expect("report lock").as_slice(),
            [attempt],
            "the returned report is the report Action1 retained"
        );
        assert!(execute(&permits, &body, id).await.is_err());
    }

    #[tokio::test]
    async fn service_status_returns_the_state_the_adapter_read() {
        let id = Uuid::new_v4();
        let permits = one_permit(
            id,
            ExecutableAction::ServiceStatus {
                unit: "cybou-action-test.service".to_owned(),
            },
        );
        let attempt = execute(&permits, &RecordingBody::default(), id)
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

    #[tokio::test]
    async fn a_body_effect_can_happen_before_its_final_report_is_lost() {
        let id = Uuid::new_v4();
        let mut permits = one_permit(
            id,
            ExecutableAction::ServiceRestart {
                unit: "cybou-action-test.service".to_owned(),
            },
        );
        permits.lose_report = true;
        let body = RecordingBody::default();

        assert!(
            matches!(
                execute(&permits, &body, id).await,
                Err(ExecutorError::ReportNotRecorded(_))
            ),
            "the caller receives no success when Action1 did not retain the final report"
        );
        assert_eq!(
            body.0.lock().expect("body lock").as_slice(),
            ["restart:cybou-action-test.service"],
            "the adversarial window is real: the effect preceded the lost report"
        );
    }
}
