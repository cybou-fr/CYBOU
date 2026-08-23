// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Criticising a proposal, and deciding what may be done with it.
//!
//! ADR-0022: *the proposal carries no inherent permission to perform it.* That sentence is easy to
//! agree with and easy to lose, because the natural shape of the code — build a proposal, hand it to
//! an executor — has no place for the decision to live. So it lives here, before any executor
//! exists, and the executor will arrive to find the gate already closed.
//!
//! ## Nothing is granted in this build, and that is not a placeholder
//!
//! [`AuthorizationVerdict::Granted`] means *carry this out without asking*, and it is reachable only
//! through a standing policy a person set. The default policy grants nothing, so on any installation
//! that has not been configured, every proposal is either refused or put to the person. That is the
//! correct behaviour and not a stub waiting to be relaxed: pre-authorisation is a thing an operator
//! decides about their own machine, and a default that granted anything would be this system
//! deciding it for them.
//!
//! ## The critics run before the verdict, and a failed critic cannot be outvoted
//!
//! A check that objects makes the proposal refusable, whatever the risk level says. The alternative
//! — weighing objections against confidence — is how a system talks itself into something, and the
//! objection that matters most is usually the one nobody weighted highly enough.

use cybou_protocol::action::{
    ActionProposal, AuthorizationDecision, AuthorizationVerdict, CriticismCheck,
};
use cybou_protocol::telemetry::SystemInsight;
use time::OffsetDateTime;

use crate::operation::{ALL_OPERATIONS, Operation};

/// What an operator has decided may happen without being asked.
///
/// Empty by default. Every field is something a person turned on about their own machine.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StandingPolicy {
    /// Operations that may be carried out without confirmation.
    ///
    /// A forbidden operation listed here is still refused. A policy cannot grant what the operation
    /// table says is off the table — otherwise the forbidden list would be advisory, which is the
    /// same as absent.
    pub pre_authorized: Vec<Operation>,
}

impl StandingPolicy {
    /// A policy that grants nothing. The state of an installation nobody has configured.
    #[must_use]
    pub const fn nothing_pre_authorized() -> Self {
        Self {
            pre_authorized: Vec::new(),
        }
    }
}

/// Run every critic against a proposal.
///
/// The insight is passed because most of what is worth objecting to is a mismatch between the
/// action and the evidence, and a critic that could not see the evidence could only check the
/// action against itself.
#[must_use]
pub fn criticise(proposal: &ActionProposal, insight: &SystemInsight) -> Vec<CriticismCheck> {
    let operation = ALL_OPERATIONS
        .iter()
        .copied()
        .find(|candidate| candidate.verb() == proposal.operation);

    let mut checks = Vec::new();

    checks.push(match operation {
        Some(_) => CriticismCheck {
            rule_id: "operation-is-known".to_owned(),
            description: "The operation is one this build can express.".to_owned(),
            passed: true,
            objection: None,
        },
        None => CriticismCheck {
            rule_id: "operation-is-known".to_owned(),
            description: "The operation is one this build can express.".to_owned(),
            passed: false,
            // An unknown verb is not an operation with unknown risk; it is not an operation. A
            // system that let one through would be accepting a string as an instruction, which is
            // the shape this whole boundary exists to refuse.
            objection: Some(format!("'{}' is not an operation", proposal.operation)),
        },
    });

    let Some(operation) = operation else {
        return checks;
    };

    checks.push(CriticismCheck {
        rule_id: "risk-matches-operation".to_owned(),
        description: "The stated risk is the operation's own.".to_owned(),
        passed: proposal.risk_level == operation.risk()
            && proposal.reversible == operation.reversible(),
        objection: (proposal.risk_level != operation.risk()
            || proposal.reversible != operation.reversible())
        .then(|| {
            // Something arguing for its own proposal understating its danger. The check exists
            // because `ActionProposal` carries these as ordinary fields, so anything that builds
            // one by hand can fill them in freely.
            format!(
                "stated {:?}/reversible={} but {} is {:?}/reversible={}",
                proposal.risk_level,
                proposal.reversible,
                operation.verb(),
                operation.risk(),
                operation.reversible()
            )
        }),
    });

    let addresses = operation.relieves().contains(&insight.finding);
    checks.push(CriticismCheck {
        rule_id: "action-addresses-the-finding".to_owned(),
        description: "The action would relieve what was actually found.".to_owned(),
        // Reading something changes nothing, so it needs no justification from the evidence. Every
        // mutation does: proposing to clear a cache because memory is under pressure is a plausible
        // sentence and an unrelated act.
        passed: addresses || operation == Operation::InspectServiceStatus,
        objection: (!addresses && operation != Operation::InspectServiceStatus).then(|| {
            format!(
                "{} does not relieve {}",
                operation.verb(),
                insight.finding.name()
            )
        }),
    });

    checks.push(CriticismCheck {
        rule_id: "cause-is-named".to_owned(),
        description: "The proposal names what gave rise to it.".to_owned(),
        passed: proposal.cause_id == Some(insight.insight_id),
        objection: (proposal.cause_id != Some(insight.insight_id))
            .then(|| "the proposal does not name this insight as its cause".to_owned()),
    });

    checks
}

/// Decide what may be done with a proposal.
///
/// # Order
///
/// Forbidden first, then objections, then policy, then risk. Deliberately: a critic that objects to
/// a pre-authorised operation must still stop it, and a forbidden operation must be refused before
/// anything gets the chance to weigh it against anything.
#[must_use]
pub fn authorize(
    proposal: &ActionProposal,
    checks: &[CriticismCheck],
    insight_strength_is_weak: bool,
    policy: &StandingPolicy,
    now: OffsetDateTime,
) -> AuthorizationDecision {
    let operation = ALL_OPERATIONS
        .iter()
        .copied()
        .find(|candidate| candidate.verb() == proposal.operation);

    let verdict = decide(
        operation,
        proposal,
        checks,
        insight_strength_is_weak,
        policy,
    );

    AuthorizationDecision {
        proposal_id: proposal.proposal_id,
        verdict,
        checked_capabilities: operation
            .map(|operation| vec![operation.verb().to_owned()])
            .unwrap_or_default(),
        decided_at: now,
    }
}

/// The verdict itself.
fn decide(
    operation: Option<Operation>,
    proposal: &ActionProposal,
    checks: &[CriticismCheck],
    insight_strength_is_weak: bool,
    policy: &StandingPolicy,
) -> AuthorizationVerdict {
    let Some(operation) = operation else {
        return AuthorizationVerdict::Denied {
            reason: format!("'{}' is not an operation", proposal.operation),
        };
    };

    if operation.forbidden() {
        return AuthorizationVerdict::Denied {
            reason: format!(
                "{} is {:?} and is refused regardless of evidence",
                operation.verb(),
                operation.risk()
            ),
        };
    }

    if let Some(failed) = checks.iter().find(|check| !check.passed) {
        return AuthorizationVerdict::Denied {
            reason: failed
                .objection
                .clone()
                .unwrap_or_else(|| failed.rule_id.clone()),
        };
    }

    if insight_strength_is_weak && operation != Operation::InspectServiceStatus {
        // One reading out of range with nothing corroborating it is a reason to look, not a reason
        // to change something. A system that acted on its weakest conclusions would spend its
        // credibility on the cases it is least sure about.
        return AuthorizationVerdict::Denied {
            reason: "the evidence is one uncorroborated reading; look before changing anything"
                .to_owned(),
        };
    }

    if policy.pre_authorized.contains(&operation) {
        return AuthorizationVerdict::Granted;
    }

    AuthorizationVerdict::RequiresUserConfirmation {
        prompt: format!(
            "{} on {} ({:?}, {}). Proceed?",
            operation.verb(),
            proposal.target_resource,
            operation.risk(),
            if operation.reversible() {
                "reversible"
            } else {
                "cannot be undone"
            }
        ),
    }
}

/// Whether a decision permits anything to happen without asking a person.
#[must_use]
pub const fn permits_unattended(decision: &AuthorizationDecision) -> bool {
    matches!(decision.verdict, AuthorizationVerdict::Granted)
}

#[cfg(test)]
mod tests {
    use cybou_protocol::action::RiskLevel;
    use cybou_protocol::telemetry::{EvidenceStrength, Finding};
    use uuid::Uuid;

    use super::*;
    use crate::propose::propose;

    fn at() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000).expect("a fixed instant")
    }

    fn ids() -> impl Fn(Operation) -> Uuid {
        |operation| Uuid::from_u128(operation.verb().len() as u128)
    }

    fn insight(finding: Finding) -> SystemInsight {
        SystemInsight {
            insight_id: Uuid::from_u128(42),
            finding,
            because: Vec::new(),
            strength: EvidenceStrength::Strong,
            concluded_at: at(),
            since: at(),
        }
    }

    fn first_proposal(finding: Finding) -> ActionProposal {
        propose(&insight(finding), at(), ids())
            .into_iter()
            .next()
            .expect("this finding has a remedy")
    }

    fn verdict_for(proposal: &ActionProposal, insight: &SystemInsight) -> AuthorizationVerdict {
        let checks = criticise(proposal, insight);
        authorize(
            proposal,
            &checks,
            insight.strength == EvidenceStrength::Weak,
            &StandingPolicy::nothing_pre_authorized(),
            at(),
        )
        .verdict
    }

    #[test]
    fn an_unconfigured_installation_grants_nothing() {
        // Not a stub waiting to be relaxed. Pre-authorisation is a thing an operator decides about
        // their own machine, and a default that granted anything would be this system deciding it
        // for them.
        let insight = insight(Finding::StorageExhaustion);
        for proposal in propose(&insight, at(), ids()) {
            let decision = authorize(
                &proposal,
                &criticise(&proposal, &insight),
                false,
                &StandingPolicy::nothing_pre_authorized(),
                at(),
            );
            assert!(
                !permits_unattended(&decision),
                "{} was granted on a machine nobody configured",
                proposal.operation
            );
        }
    }

    #[test]
    fn a_forbidden_operation_is_refused_however_it_arrives() {
        // The example from the product discussion: deleting a database's data directory to free
        // space. It is never proposed, and it is refused if something else builds it.
        let hand_built = ActionProposal {
            proposal_id: Uuid::from_u128(1),
            cause_id: Some(Uuid::from_u128(42)),
            intent: "relieve storage-exhaustion".to_owned(),
            operation: Operation::DeleteServiceData.verb().to_owned(),
            target_resource: "systemd:postgresql.service".to_owned(),
            parameters: Vec::new(),
            risk_level: RiskLevel::Critical,
            reversible: false,
            proposed_at: at(),
        };
        match verdict_for(&hand_built, &insight(Finding::StorageExhaustion)) {
            AuthorizationVerdict::Denied { reason } => {
                assert!(reason.contains("regardless of evidence"), "{reason}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_policy_cannot_grant_what_the_operation_table_forbids() {
        // Otherwise the forbidden list is advisory, which is the same as absent.
        let permissive = StandingPolicy {
            pre_authorized: ALL_OPERATIONS.to_vec(),
        };
        let hand_built = ActionProposal {
            operation: Operation::PowerOff.verb().to_owned(),
            risk_level: RiskLevel::Critical,
            reversible: false,
            ..first_proposal(Finding::StorageExhaustion)
        };
        let decision = authorize(
            &hand_built,
            &criticise(&hand_built, &insight(Finding::StorageExhaustion)),
            false,
            &permissive,
            at(),
        );
        assert!(matches!(
            decision.verdict,
            AuthorizationVerdict::Denied { .. }
        ));
    }

    #[test]
    fn a_proposal_understating_its_own_danger_is_caught() {
        // `ActionProposal` carries risk as an ordinary field, so anything building one by hand can
        // fill it in freely. This is the check that makes that harmless.
        let understated = ActionProposal {
            risk_level: RiskLevel::Low,
            reversible: true,
            ..first_proposal(Finding::StorageExhaustion)
        };
        let checks = criticise(&understated, &insight(Finding::StorageExhaustion));
        let failed = checks
            .iter()
            .find(|check| !check.passed)
            .expect("the understatement is caught");
        assert_eq!(failed.rule_id, "risk-matches-operation");
        assert!(matches!(
            verdict_for(&understated, &insight(Finding::StorageExhaustion)),
            AuthorizationVerdict::Denied { .. }
        ));
    }

    #[test]
    fn an_action_that_does_not_address_what_was_found_is_objected_to() {
        // Clearing a package cache because memory is under pressure is a plausible sentence and an
        // unrelated act.
        let unrelated = ActionProposal {
            intent: "relieve memory-pressure".to_owned(),
            ..first_proposal(Finding::StorageExhaustion)
        };
        let checks = criticise(&unrelated, &insight(Finding::MemoryPressure));
        assert!(
            checks
                .iter()
                .any(|check| check.rule_id == "action-addresses-the-finding" && !check.passed),
            "{checks:?}"
        );
    }

    #[test]
    fn looking_at_something_needs_no_justification_from_the_evidence() {
        // Reading a unit's state changes nothing. Requiring it to relieve the finding would make
        // investigation harder than mutation, which is exactly backwards.
        let insight = insight(Finding::MemoryPressure);
        let inspect = propose(&insight, at(), ids())
            .into_iter()
            .find(|proposal| proposal.operation == Operation::InspectServiceStatus.verb())
            .expect("inspection is offered");
        assert!(
            criticise(&inspect, &insight)
                .iter()
                .all(|check| check.passed)
        );
        assert!(matches!(
            verdict_for(&inspect, &insight),
            AuthorizationVerdict::RequiresUserConfirmation { .. }
        ));
    }

    #[test]
    fn one_uncorroborated_reading_is_a_reason_to_look_and_not_to_change_anything() {
        // A system that acted on its weakest conclusions would spend its credibility on the cases
        // it is least sure about.
        let weak = SystemInsight {
            strength: EvidenceStrength::Weak,
            ..insight(Finding::StorageExhaustion)
        };
        let proposal = first_proposal(Finding::StorageExhaustion);
        match verdict_for(&proposal, &weak) {
            AuthorizationVerdict::Denied { reason } => {
                assert!(reason.contains("look before changing"), "{reason}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_string_that_is_not_an_operation_is_not_an_operation_with_unknown_risk() {
        // A system that let one through would be accepting a string as an instruction.
        let invented = ActionProposal {
            operation: "rm -rf /var/lib/postgresql".to_owned(),
            ..first_proposal(Finding::StorageExhaustion)
        };
        let decision = authorize(
            &invented,
            &criticise(&invented, &insight(Finding::StorageExhaustion)),
            false,
            &StandingPolicy::nothing_pre_authorized(),
            at(),
        );
        assert!(matches!(
            decision.verdict,
            AuthorizationVerdict::Denied { .. }
        ));
        assert!(
            decision.checked_capabilities.is_empty(),
            "an unknown verb was reported as a checked capability"
        );
    }

    #[test]
    fn a_configured_operator_can_pre_authorize_something_ordinary() {
        // The control. Every test above passes on a gate that refuses everything, and a gate that
        // refuses everything is not a policy boundary, it is an off switch.
        let insight = insight(Finding::StorageExhaustion);
        let proposal = first_proposal(Finding::StorageExhaustion);
        let policy = StandingPolicy {
            pre_authorized: vec![Operation::CleanPackageCache],
        };
        let decision = authorize(
            &proposal,
            &criticise(&proposal, &insight),
            false,
            &policy,
            at(),
        );
        assert!(permits_unattended(&decision));
    }

    #[test]
    fn a_failed_critic_stops_a_pre_authorized_operation_too() {
        // The objection that matters most is usually the one nobody weighted highly enough, so
        // objections are not weighed against anything.
        let insight = insight(Finding::StorageExhaustion);
        let understated = ActionProposal {
            risk_level: RiskLevel::Low,
            ..first_proposal(Finding::StorageExhaustion)
        };
        let policy = StandingPolicy {
            pre_authorized: vec![Operation::CleanPackageCache],
        };
        let decision = authorize(
            &understated,
            &criticise(&understated, &insight),
            false,
            &policy,
            at(),
        );
        assert!(!permits_unattended(&decision));
    }
}
