// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Owner of the authorized-action lifecycle described by ADR-0022.

use std::{collections::HashMap, sync::Mutex};

use cybou_protocol::{
    action::{
        ActionProposal, AuthorizationDecision, AuthorizationVerdict, CriticismCheck,
        ExecutableAction, ExecutionPermit,
    },
    telemetry::SystemInsight,
};
use cybou_remediation::{Operation, StandingPolicy, authorize, criticise, propose};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

#[cfg(target_os = "linux")]
pub mod service;

/// How long an authorization remains a claimable capability.
const PERMIT_LIFETIME: Duration = Duration::seconds(60);

/// One proposal after Action1 has criticised and decided it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionRecord {
    /// The lifecycle identity and requested operation.
    pub proposal: ActionProposal,
    /// Every criticism that ran.
    pub checks: Vec<CriticismCheck>,
    /// The policy decision.
    pub decision: AuthorizationDecision,
    /// Present only for a granted decision with a first-executor adapter.
    pub permit_id: Option<Uuid>,
}

/// A refusal at the action boundary.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ActionError {
    /// The requested operation is not one this build can propose.
    #[error("'{0}' is not an operation")]
    UnknownOperation(String),
    /// The action does not name a concrete executor target.
    #[error("the executor target is not concrete: {0}")]
    InvalidTarget(String),
    /// The permit is absent, expired, or was already consumed.
    #[error("permit is absent, expired, or already consumed")]
    PermitUnavailable,
    /// Internal lifecycle state could not be accessed.
    #[error("action lifecycle state is unavailable")]
    StateUnavailable,
}

/// In-memory lifecycle owner. Durable Journal recording is a separate accepted-protocol step.
pub struct ActionCore {
    policy: StandingPolicy,
    records: Mutex<HashMap<Uuid, ActionRecord>>,
    permits: Mutex<HashMap<Uuid, ExecutionPermit>>,
}

impl ActionCore {
    /// Start Action1 with the standing policy an operator supplied.
    #[must_use]
    pub fn new(policy: StandingPolicy) -> Self {
        Self {
            policy,
            records: Mutex::new(HashMap::new()),
            permits: Mutex::new(HashMap::new()),
        }
    }

    /// Create, criticise, decide, and retain one proposal from a Mind finding.
    ///
    /// # Errors
    ///
    /// Refuses unknown verbs, invalid concrete targets, or unavailable internal state.
    pub fn evaluate_insight(
        &self,
        insight: &SystemInsight,
        verb: &str,
        now: OffsetDateTime,
    ) -> Result<ActionRecord, ActionError> {
        let operation = operation_for(verb)?;
        let proposal = propose(insight, now, |_| Uuid::new_v4())
            .into_iter()
            .find(|proposal| proposal.operation == verb)
            .ok_or_else(|| ActionError::UnknownOperation(verb.to_owned()))?;
        let mut checks = criticise(&proposal, insight);
        let adapter = executable_action(&proposal, operation);
        checks.push(CriticismCheck {
            rule_id: "executor-adapter-exists".to_owned(),
            description: "The first executor has a typed adapter for this action and target."
                .to_owned(),
            passed: adapter.is_ok(),
            objection: adapter.as_ref().err().map(ToString::to_string),
        });
        let decision = authorize(
            &proposal,
            &checks,
            insight.strength == cybou_protocol::telemetry::EvidenceStrength::Weak,
            &self.policy,
            now,
        );
        let permit = if matches!(decision.verdict, AuthorizationVerdict::Granted) {
            adapter
                .ok()
                .map(|action| permit_for(&proposal, &decision, action, now))
        } else {
            None
        };
        let permit_id = permit.as_ref().map(|permit| permit.permit_id);
        if let Some(permit) = permit {
            self.permits
                .lock()
                .map_err(|_| ActionError::StateUnavailable)?
                .insert(permit.permit_id, permit);
        }
        let record = ActionRecord {
            proposal,
            checks,
            decision,
            permit_id,
        };
        self.records
            .lock()
            .map_err(|_| ActionError::StateUnavailable)?
            .insert(record.proposal.proposal_id, record.clone());
        Ok(record)
    }

    /// Atomically consume a permit. A second claim receives nothing.
    ///
    /// # Errors
    ///
    /// Refuses unknown, consumed, expired permits and unavailable internal state.
    pub fn claim_permit(
        &self,
        permit_id: Uuid,
        now: OffsetDateTime,
    ) -> Result<ExecutionPermit, ActionError> {
        let permit = self
            .permits
            .lock()
            .map_err(|_| ActionError::StateUnavailable)?
            .remove(&permit_id)
            .ok_or(ActionError::PermitUnavailable)?;
        if now > permit.expires_at {
            return Err(ActionError::PermitUnavailable);
        }
        Ok(permit)
    }

    /// Read one retained lifecycle record.
    #[must_use]
    pub fn record(&self, proposal_id: Uuid) -> Option<ActionRecord> {
        self.records.lock().ok()?.get(&proposal_id).cloned()
    }
}

fn operation_for(verb: &str) -> Result<Operation, ActionError> {
    cybou_remediation::ALL_OPERATIONS
        .iter()
        .copied()
        .find(|operation| operation.verb() == verb)
        .ok_or_else(|| ActionError::UnknownOperation(verb.to_owned()))
}

fn concrete_service(target: &str) -> Result<String, ActionError> {
    let unit = target
        .strip_prefix("systemd:")
        .ok_or_else(|| ActionError::InvalidTarget(target.to_owned()))?;
    let valid = unit.ends_with(".service")
        && unit != "<unit>.service"
        && !unit.contains('<')
        && !unit.contains('>')
        && unit.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'@' | b':')
        });
    if !valid {
        return Err(ActionError::InvalidTarget(target.to_owned()));
    }
    Ok(unit.to_owned())
}

fn permit_for(
    proposal: &ActionProposal,
    decision: &AuthorizationDecision,
    action: ExecutableAction,
    now: OffsetDateTime,
) -> ExecutionPermit {
    ExecutionPermit {
        permit_id: Uuid::new_v4(),
        decision_id: decision.decision_id,
        proposal_id: proposal.proposal_id,
        action,
        issued_at: now,
        expires_at: now + PERMIT_LIFETIME,
    }
}

fn executable_action(
    proposal: &ActionProposal,
    operation: Operation,
) -> Result<ExecutableAction, ActionError> {
    Ok(match operation {
        Operation::InspectServiceStatus => ExecutableAction::ServiceStatus {
            unit: concrete_service(&proposal.target_resource)?,
        },
        Operation::CleanPackageCache => {
            if proposal.target_resource != "apt:archives" {
                return Err(ActionError::InvalidTarget(proposal.target_resource.clone()));
            }
            ExecutableAction::PackageCacheClean
        }
        Operation::RestartService => ExecutableAction::ServiceRestart {
            unit: concrete_service(&proposal.target_resource)?,
        },
        _ => return Err(ActionError::UnknownOperation(proposal.operation.clone())),
    })
}

#[cfg(test)]
mod tests {
    use cybou_protocol::telemetry::{EvidenceStrength, Finding, MetricKey, Subject, SystemInsight};

    use super::*;

    fn insight() -> SystemInsight {
        SystemInsight {
            insight_id: Uuid::new_v4(),
            finding: Finding::ServiceInactive,
            about: Some(MetricKey::named(
                Subject::ServiceActive,
                "cybou-action-test.service".to_owned(),
            )),
            because: Vec::new(),
            strength: EvidenceStrength::Strong,
            concluded_at: OffsetDateTime::UNIX_EPOCH,
            since: OffsetDateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn granted_decision_mints_one_short_lived_typed_permit() {
        let core = ActionCore::new(StandingPolicy {
            pre_authorized: vec![Operation::RestartService],
            pre_authorized_for_agents: Vec::new(),
        });
        let now = OffsetDateTime::UNIX_EPOCH;
        let record = core
            .evaluate_insight(&insight(), "service.restart", now)
            .expect("evaluate");
        let id = record.permit_id.expect("permit");
        let permit = core.claim_permit(id, now).expect("first claim");
        assert_eq!(
            permit.action,
            ExecutableAction::ServiceRestart {
                unit: "cybou-action-test.service".to_owned()
            }
        );
        assert_eq!(
            core.claim_permit(id, now),
            Err(ActionError::PermitUnavailable)
        );
    }

    #[test]
    fn placeholder_unit_never_becomes_a_permit() {
        let core = ActionCore::new(StandingPolicy {
            pre_authorized: vec![Operation::RestartService],
            pre_authorized_for_agents: Vec::new(),
        });
        let mut insight = insight();
        insight.about = None;
        let record = core
            .evaluate_insight(&insight, "service.restart", OffsetDateTime::UNIX_EPOCH)
            .expect("a refusal is still a lifecycle record");
        assert!(matches!(
            record.decision.verdict,
            AuthorizationVerdict::Denied { .. }
        ));
        assert!(record.permit_id.is_none());
    }

    #[test]
    fn default_policy_stops_before_the_executor() {
        let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());
        let record = core
            .evaluate_insight(&insight(), "service.restart", OffsetDateTime::UNIX_EPOCH)
            .expect("evaluate");
        assert!(matches!(
            record.decision.verdict,
            AuthorizationVerdict::RequiresUserConfirmation { .. }
        ));
        assert!(record.permit_id.is_none());
    }

    #[test]
    fn expired_permit_is_consumed_and_refused() {
        let core = ActionCore::new(StandingPolicy {
            pre_authorized: vec![Operation::InspectServiceStatus],
            pre_authorized_for_agents: Vec::new(),
        });
        let now = OffsetDateTime::UNIX_EPOCH;
        let record = core
            .evaluate_insight(&insight(), "service.status", now)
            .expect("evaluate");
        assert_eq!(
            core.claim_permit(
                record.permit_id.expect("permit"),
                now + Duration::seconds(61)
            ),
            Err(ActionError::PermitUnavailable)
        );
    }

    #[test]
    fn operation_without_one_of_the_three_adapters_is_denied() {
        let core = ActionCore::new(StandingPolicy {
            pre_authorized: vec![Operation::ReloadService],
            pre_authorized_for_agents: Vec::new(),
        });
        let record = core
            .evaluate_insight(&insight(), "service.reload", OffsetDateTime::UNIX_EPOCH)
            .expect("refusal record");
        assert!(matches!(
            record.decision.verdict,
            AuthorizationVerdict::Denied { .. }
        ));
        assert!(record.permit_id.is_none());
    }

    #[test]
    fn package_cache_clean_is_the_one_fixed_non_systemd_target() {
        let core = ActionCore::new(StandingPolicy {
            pre_authorized: vec![Operation::CleanPackageCache],
            pre_authorized_for_agents: Vec::new(),
        });
        let mut storage = insight();
        storage.finding = Finding::StorageExhaustion;
        storage.about = None;
        let record = core
            .evaluate_insight(&storage, "package.cache.clean", OffsetDateTime::UNIX_EPOCH)
            .expect("evaluate");
        let permit = core
            .claim_permit(
                record.permit_id.expect("permit"),
                OffsetDateTime::UNIX_EPOCH,
            )
            .expect("claim");
        assert_eq!(permit.action, ExecutableAction::PackageCacheClean);
    }
}
