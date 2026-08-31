// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Turning what the host concluded into something it could offer to do about it.
//!
//! Deterministic, and from the finding rather than from prose. What is proposed for storage
//! exhaustion is a property of the finding; a model may later word the offer better, and it does not
//! get to decide what the offer is.
//!
//! Every proposal names the insight that caused it. That is not bookkeeping: an action whose cause
//! cannot be named is an action nobody can argue with afterwards, and the whole reason this path
//! exists is so that *why are you offering to do that* has an answer made of readings.

use cybou_protocol::action::{ActionProposal, Proposer};
use cybou_protocol::telemetry::{Finding, MetricKey, SystemInsight};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::operation::Operation;

/// What Cybou could offer to do about one finding, in the order it would offer them.
///
/// Cheapest and most reversible first. An operator reading a list wants the least committal thing
/// at the top, and a system that led with the most effective remedy would be optimising for the
/// problem rather than for the person deciding.
#[must_use]
pub fn remedies_for(finding: Finding) -> Vec<Operation> {
    match finding {
        Finding::StorageExhaustion => vec![
            Operation::CleanPackageCache,
            Operation::RotateLogs,
            Operation::TrimTemporaryFiles,
        ],
        // A declared service that is not running is the same situation as one that failed, and the
        // same three things are worth offering. Which one it is affects what the host says, not
        // what it could do about it.
        Finding::ServiceFailure | Finding::ServiceInactive => vec![
            Operation::InspectServiceStatus,
            Operation::ReloadService,
            Operation::RestartService,
        ],
        // Looking, and nothing else, for both. Something is holding memory or descriptors it is not
        // releasing, and this build cannot say which — a restart proposal would name a placeholder
        // unit, which is a proposal to restart whatever came to mind.
        Finding::MemoryPressure | Finding::FileDescriptorExhaustion => {
            vec![Operation::InspectServiceStatus]
        }
        // Nothing here is a remedy for waiting on a disk or a CPU, and nothing here renews a
        // certificate. Renewal is a deadline met outside this machine's control — by an ACME client,
        // a registrar, a person — and an offer would be offering to do something in order to be seen
        // doing something, which is how an operator learns to stop reading what a system suggests.
        // Nothing here runs a backup either. What would relieve a stale backup is the backup
        // succeeding, and this build has no operation that could make it.
        Finding::BackupStale
        | Finding::CertificateExpiring
        | Finding::IoSaturation
        | Finding::CpuSaturation
        | Finding::UnexplainedDeviation => Vec::new(),
    }
}

/// Build the proposals for one insight.
///
/// Identities are supplied rather than generated, for the same reason as every planner here: a
/// producer that reached for a random source would be reaching for something.
#[must_use]
pub fn propose(
    insight: &SystemInsight,
    now: OffsetDateTime,
    id: impl Fn(Operation) -> Uuid,
) -> Vec<ActionProposal> {
    remedies_for(insight.finding)
        .into_iter()
        .map(|operation| ActionProposal {
            proposal_id: id(operation),
            proposed_by: Proposer::Mind,
            // The insight, always. An action whose cause cannot be named is an action nobody can
            // argue with afterwards.
            cause_id: Some(insight.insight_id),
            intent: format!("relieve {}", insight.finding.name()),
            operation: operation.verb().to_owned(),
            target_resource: target_for(operation, insight),
            parameters: Vec::new(),
            // Taken from the operation, never supplied. Something arguing for its own proposal is
            // the wrong party to assess it.
            risk_level: operation.risk(),
            reversible: operation.reversible(),
            proposed_at: now,
        })
        .collect()
}

/// What an operation would act on.
///
/// Taken from the finding wherever the finding knows. An insight about a declared service carries
/// the unit it is about, and the proposal names that unit; an insight about the *count* of failed
/// units does not know which one failed, and the proposal keeps the placeholder. The two cases must
/// stay visibly different — a placeholder that quietly became a real unit name would be a proposal
/// to restart whatever came to mind, wearing the look of one somebody chose.
fn target_for(operation: Operation, insight: &SystemInsight) -> String {
    let about = insight.about.as_ref().and_then(MetricKey::target);
    match operation {
        Operation::CleanPackageCache => "apt:archives".to_owned(),
        Operation::RotateLogs => "journald:logs".to_owned(),
        Operation::TrimTemporaryFiles => "path:/tmp".to_owned(),
        Operation::InspectServiceStatus
        | Operation::ReloadService
        | Operation::RestartService
        | Operation::StartService
        | Operation::StopService
        | Operation::DeleteServiceData => about.unwrap_or_else(|| "systemd:<unit>".to_owned()),
        Operation::FormatFilesystem => "filesystem:<device>".to_owned(),
        Operation::PowerOff => "system:self".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use cybou_protocol::action::RiskLevel;
    use cybou_protocol::telemetry::EvidenceStrength;

    use super::*;

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
            about: None,
            because: Vec::new(),
            strength: EvidenceStrength::Strong,
            concluded_at: at(),
            since: at(),
        }
    }

    #[test]
    fn nothing_forbidden_is_ever_proposed() {
        // The refusal below in `authorize` is the second line of defence. This is the first: the
        // remedy tables do not contain them at all, so a forbidden operation cannot arrive by
        // somebody adding a finding and forgetting.
        for finding in [
            Finding::StorageExhaustion,
            Finding::ServiceFailure,
            Finding::MemoryPressure,
            Finding::IoSaturation,
            Finding::CpuSaturation,
            Finding::UnexplainedDeviation,
        ] {
            for operation in remedies_for(finding) {
                assert!(!operation.forbidden(), "{finding:?} offered {operation:?}");
            }
        }
    }

    #[test]
    fn the_least_committal_thing_is_at_the_top() {
        // An operator reading a list wants the safest option first. Leading with the most effective
        // remedy would be optimising for the problem rather than for the person deciding.
        let offered = remedies_for(Finding::ServiceFailure);
        assert_eq!(offered[0], Operation::InspectServiceStatus);
        assert_eq!(offered[offered.len() - 1], Operation::RestartService);

        let storage = remedies_for(Finding::StorageExhaustion);
        let risks: Vec<RiskLevel> = storage.iter().map(|op| op.risk()).collect();
        assert_eq!(
            risks,
            vec![RiskLevel::Medium, RiskLevel::Medium, RiskLevel::High]
        );
    }

    #[test]
    fn a_finding_with_no_remedy_produces_no_proposal_rather_than_a_gesture() {
        // Offering a restart for I/O saturation would be offering to do something in order to be
        // seen doing something, which is how an operator learns to stop reading what a system
        // suggests.
        assert!(propose(&insight(Finding::IoSaturation), at(), ids()).is_empty());
        assert!(propose(&insight(Finding::CpuSaturation), at(), ids()).is_empty());
        assert!(propose(&insight(Finding::UnexplainedDeviation), at(), ids()).is_empty());
    }

    #[test]
    fn every_proposal_names_the_insight_that_caused_it() {
        // An action whose cause cannot be named is an action nobody can argue with afterwards.
        let proposals = propose(&insight(Finding::StorageExhaustion), at(), ids());
        assert!(!proposals.is_empty());
        for proposal in &proposals {
            assert_eq!(proposal.cause_id, Some(Uuid::from_u128(42)));
            assert!(proposal.intent.contains("storage-exhaustion"));
        }
    }

    #[test]
    fn a_proposal_carries_the_risk_of_its_operation_and_not_one_it_chose() {
        let proposals = propose(&insight(Finding::ServiceFailure), at(), ids());
        for proposal in &proposals {
            let operation = crate::operation::ALL_OPERATIONS
                .iter()
                .find(|op| op.verb() == proposal.operation)
                .expect("a known operation");
            assert_eq!(proposal.risk_level, operation.risk());
            assert_eq!(proposal.reversible, operation.reversible());
        }
    }

    #[test]
    fn a_target_that_only_an_investigation_could_supply_is_left_unfilled() {
        // A proposal that guessed a unit name would be a proposal to restart whatever came to mind.
        let proposals = propose(&insight(Finding::ServiceFailure), at(), ids());
        assert!(
            proposals
                .iter()
                .all(|proposal| proposal.target_resource.contains("<unit>")),
            "{proposals:?}"
        );

        let storage = propose(&insight(Finding::StorageExhaustion), at(), ids());
        assert!(
            storage
                .iter()
                .all(|proposal| !proposal.target_resource.contains('<')),
            "a knowable target was left as a placeholder: {storage:?}"
        );
    }

    #[test]
    fn the_same_finding_always_produces_the_same_offer() {
        let first = propose(&insight(Finding::StorageExhaustion), at(), ids());
        for _ in 0..8 {
            assert_eq!(
                propose(&insight(Finding::StorageExhaustion), at(), ids()),
                first
            );
        }
    }
}
