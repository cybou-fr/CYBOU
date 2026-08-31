// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The action lifecycle, written down so it survives the process that decided it.
//!
//! Action1 held proposals, criticism and decisions in memory. For a first vertical that was enough;
//! for a host that acts on its own, and much more for one that lets an agent ask it to, it is not.
//! The question a person asks a month later is not *what is Action1 holding* — it is:
//!
//! ```text
//! Why did nginx restart on the fourteenth?
//! ```
//!
//! and answering it means the proposal, the objections raised against it, and the decision are still
//! there. A restart of `cybou-actiond` destroyed all three, which made the causal chain a property of
//! a process's uptime. A Persistent Mind that forgets what it authorized has not kept its promise.
//!
//! ## The Journal already had the words
//!
//! Nothing is invented here. The contribution kinds this repository has carried all along line up
//! with the lifecycle exactly, so an action becomes ordinary Journal content rather than a private
//! log beside it:
//!
//! ```text
//! ActionProposal           PlanProposal   what was proposed
//! a criticism that failed  Objection      what was said against it
//! AuthorizationDecision    Decision       what was decided
//! ExecutionStarted         Intention      when an effect first became possible
//! ExecutionAttempt         Intention      what the executor finally reported
//! ActionOutcome            Outcome        what it independently saw afterwards
//! ```
//!
//! Both execution steps are `Intention`s because the Journal has no kind for *acting*. The first is
//! the critical one: the host has bound itself to a stable attempt identity before the Body may be
//! touched. The second adds the executor's final account. An outcome is an `Outcome`, which the
//! Journal treats as terminal and permits once per cause — exactly right for an action, which
//! happens once and is answered for once.
//!
//! The attempt and the outcome are separate contributions rather than one. What a thing says about
//! itself and what the readings say afterwards are two accounts, and the entire value of
//! re-observation is that they can disagree; folding them together would delete the disagreement,
//! which is the only part that could ever surprise anybody.
//!
//! Only failed checks become objections. A criticism that passed is not an objection to anything, and
//! writing one would fill the record with disagreement nobody expressed.
//!
//! Causally they are one episode: every envelope shares the proposal's identity as its correlation,
//! and each cites the step before it. So a reader following causation arrives at the decision from
//! the proposal without needing to know that Action1 exists.
//!
//! ## The proposal is not a root, and finding that out cost a working feature
//!
//! It was written as one — a contribution citing nothing, on the reasoning that nothing Action1 can
//! name caused it. The Journal disagrees, and it is right: only `Observation` and `ContextDisclosed`
//! record something that happened outside the Journal. Everything else is derived and must cite a
//! cause that *exists*, or evidence that does.
//!
//! So every contribution this module wrote was refused, every time, and because recording is best
//! effort the refusal went to stderr and nothing else changed. The feature was inert from the day it
//! was written and the tests could not see it, because they tested the shape of the envelopes and
//! never handed one to a Journal.
//!
//! What did cause a proposal is the finding, which `ActionProposal::cause_id` has always carried. The
//! proposal cites it, and a proposal with no cause cannot be recorded at all — said out loud, because
//! the alternative is what happened before: submitting something certain to be refused and calling
//! the result best effort.
//!
//! ## What is deliberately not written
//!
//! The permit. It is a single-use capability with a sixty-second life, and a durable record of one
//! would be a durable record of a key — worse, one whose presence in the Journal could be mistaken
//! for the authority itself. Losing permits on restart is correct: an authorization that was never
//! claimed simply was not, and the decision that produced it is still there to be read.

use cybou_protocol::action::{
    ActionOutcome, ActionProposal, AuthorizationDecision, CriticismCheck, ExecutionAttempt,
    ExecutionStarted,
};
use cybou_protocol::admission::Kind;
use cybou_protocol::canonical::CanonicalEnvelope;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::ActionRecord;

/// The organ these contributions came from.
const ORIGIN: &str = "actiond";

/// Schema of the envelopes this module writes.
const SCHEMA: u16 = 3;

/// Namespace for the contribution that marks an attempt as able to begin.
const EXECUTION_STARTED_NAMESPACE: Uuid =
    Uuid::from_u128(0x0063_7962_6f75_5f73_7461_7274_6564_5f31);

/// What a lifecycle envelope carries, beside the identities on the envelope itself.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "step")]
pub enum LifecycleStep {
    /// What was proposed.
    Proposed {
        /// The proposal exactly as Action1 formed it.
        proposal: Box<ActionProposal>,
    },
    /// One criticism that did not pass.
    Objected {
        /// The failing check, whole, including the rule that raised it.
        check: Box<CriticismCheck>,
    },
    /// What was decided, and every criticism that ran.
    ///
    /// The checks travel with the decision as well as separately, because a decision read on its own
    /// has to be able to say what it was decided against. The objections are the ones a reader
    /// following causation encounters; this is the same evidence available to a reader who found the
    /// decision first.
    Decided {
        /// The policy decision.
        decision: Box<AuthorizationDecision>,
        /// Every criticism that ran, passing and failing alike.
        checks: Vec<CriticismCheck>,
    },
    /// A permit was consumed and this execution may now begin.
    Started {
        /// Stable identity and exact operation established before the first Body effect.
        execution: Box<ExecutionStarted>,
    },
    /// What was carried out under that decision.
    Attempted {
        /// The attempt, whole, including what the executor said about itself.
        attempt: Box<ExecutionAttempt>,
    },
    /// What the host saw for itself afterwards.
    Concluded {
        /// The outcome, including whether the two accounts agree.
        outcome: Box<ActionOutcome>,
    },
}

/// Why a replayed contribution could not be read back as part of a lifecycle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CannotReplay {
    /// The payload was not a lifecycle step this build writes.
    UnreadablePayload,
    /// A decision arrived with no proposal before it.
    DecisionWithoutProposal(Uuid),
}

impl core::fmt::Display for CannotReplay {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnreadablePayload => {
                formatter.write_str("a contribution was not a readable lifecycle step")
            }
            Self::DecisionWithoutProposal(id) => {
                write!(formatter, "decision for proposal {id} has no proposal")
            }
        }
    }
}

impl core::error::Error for CannotReplay {}

/// Every contribution one decided action produces, in causal order.
///
/// # Errors
///
/// Returns [`CannotRecord::ProposalHasNoCause`] for a proposal that names nothing that gave rise to
/// it. Such a lifecycle cannot enter the Journal at all — a derived contribution must cite a cause,
/// and one invented here would point at nothing.
///
/// # Panics
///
/// Never: encoding a lifecycle step to CBOR cannot fail for these types, and the expectation is
/// written out rather than swallowed so a future field that could fail is caught by a test.
pub fn contributions(
    record: &ActionRecord,
    now: OffsetDateTime,
) -> Result<Vec<CanonicalEnvelope>, CannotRecord> {
    let episode = record.proposal.proposal_id;
    let cause = record
        .proposal
        .cause_id
        .ok_or(CannotRecord::ProposalHasNoCause(episode))?;
    let mut out = Vec::with_capacity(2 + record.checks.len());

    // The proposal cites what gave rise to it. It is not a root: only a contribution recording
    // something that happened outside the Journal is one, and a proposal is a conclusion about a
    // finding that is already in there.
    out.push(envelope(
        episode,
        episode,
        cause,
        Kind::PlanProposal,
        &LifecycleStep::Proposed {
            proposal: Box::new(record.proposal.clone()),
        },
        now,
    ));

    let mut previous = episode;
    for check in record.checks.iter().filter(|check| !check.passed) {
        let id = Uuid::new_v4();
        out.push(envelope(
            id,
            episode,
            previous,
            Kind::Objection,
            &LifecycleStep::Objected {
                check: Box::new(check.clone()),
            },
            now,
        ));
        previous = id;
    }

    out.push(envelope(
        record.decision.decision_id,
        episode,
        previous,
        Kind::Decision,
        &LifecycleStep::Decided {
            decision: Box::new(record.decision.clone()),
            checks: record.checks.clone(),
        },
        now,
    ));

    // What was done, and then what was seen. Each cites the one before, so a reader arrives at the
    // outcome from the finding without having to know which organ wrote any of it.
    let mut previous = record.decision.decision_id;
    if let Some(started) = &record.execution_started {
        let started_message_id = execution_started_message_id(started.attempt_id);
        out.push(envelope(
            started_message_id,
            episode,
            previous,
            Kind::Intention,
            &LifecycleStep::Started {
                execution: Box::new(started.clone()),
            },
            now,
        ));
        previous = started_message_id;
    }
    if let Some(attempt) = &record.attempt {
        out.push(envelope(
            attempt.attempt_id,
            episode,
            previous,
            Kind::Intention,
            &LifecycleStep::Attempted {
                attempt: Box::new(attempt.clone()),
            },
            now,
        ));
        previous = attempt.attempt_id;
    }
    if let Some(outcome) = &record.outcome {
        out.push(envelope(
            outcome.outcome_id,
            episode,
            previous,
            Kind::Outcome,
            &LifecycleStep::Concluded {
                outcome: Box::new(outcome.clone()),
            },
            now,
        ));
    }
    Ok(out)
}

/// The contribution that must be accepted before the executor receives an action.
///
/// # Errors
///
/// Returns [`CannotRecord::NothingToContinue`] when no permit has been claimed for the record.
pub fn execution_started_contribution(
    record: &ActionRecord,
    now: OffsetDateTime,
) -> Result<CanonicalEnvelope, CannotRecord> {
    let started = record
        .execution_started
        .as_ref()
        .ok_or(CannotRecord::NothingToContinue)?;
    Ok(envelope(
        execution_started_message_id(started.attempt_id),
        record.proposal.proposal_id,
        record.decision.decision_id,
        Kind::Intention,
        &LifecycleStep::Started {
            execution: Box::new(started.clone()),
        },
        now,
    ))
}

/// Why one decided action cannot be written down.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CannotRecord {
    /// There is no attempt or outcome on this record to continue the episode with.
    NothingToContinue,
    /// An outcome was recorded for a record that carries no attempt.
    ///
    /// An outcome citing a decision rather than an attempt would say the host observed the effects
    /// of a permission, and a permission has no effects.
    OutcomeWithoutAttempt,
    /// The proposal names nothing that gave rise to it.
    ///
    /// Not recoverable here. A derived contribution must cite a cause that exists, and this module
    /// has nothing to cite: what caused a proposal is the finding, and a proposal that named none
    /// left no thread to follow back.
    ProposalHasNoCause(Uuid),
}

impl core::fmt::Display for CannotRecord {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NothingToContinue => {
                formatter.write_str("this record has nothing further to add to its episode")
            }
            Self::OutcomeWithoutAttempt => {
                formatter.write_str("an outcome was recorded for an action nobody attempted")
            }
            Self::ProposalHasNoCause(id) => {
                write!(formatter, "proposal {id} names nothing that caused it")
            }
        }
    }
}

impl core::error::Error for CannotRecord {}

/// The one contribution an attempt adds to an episode already in the Journal.
///
/// Separate from [`contributions`] because the rest of the episode is already there. Re-publishing
/// the whole record would resubmit the proposal and the decision under identities the Journal
/// already holds, and it would refuse them as duplicates — so the continuation would be lost behind
/// a rejection of something that was never the point.
///
/// # Errors
///
/// Returns [`CannotRecord::NothingToContinue`] when the record carries no attempt.
pub fn attempt_contribution(
    record: &ActionRecord,
    now: OffsetDateTime,
) -> Result<CanonicalEnvelope, CannotRecord> {
    let attempt = record
        .attempt
        .as_ref()
        .ok_or(CannotRecord::NothingToContinue)?;
    Ok(envelope(
        attempt.attempt_id,
        record.proposal.proposal_id,
        // The decision it was carried out under. An attempt traceable to a proposal but not to the
        // authorization for it is an attempt nobody can argue with afterwards.
        record
            .execution_started
            .as_ref()
            .map_or(record.decision.decision_id, |started| {
                execution_started_message_id(started.attempt_id)
            }),
        Kind::Intention,
        &LifecycleStep::Attempted {
            attempt: Box::new(attempt.clone()),
        },
        now,
    ))
}

/// The one contribution an outcome adds, terminating the episode.
///
/// # Errors
///
/// Returns [`CannotRecord::NothingToContinue`] when the record carries no outcome, and
/// [`CannotRecord::OutcomeWithoutAttempt`] when it carries one with nothing to conclude about. An
/// outcome citing a decision rather than an attempt would say the host observed the effects of a
/// permission, which is not a thing that has effects.
pub fn outcome_contribution(
    record: &ActionRecord,
    now: OffsetDateTime,
) -> Result<CanonicalEnvelope, CannotRecord> {
    let outcome = record
        .outcome
        .as_ref()
        .ok_or(CannotRecord::NothingToContinue)?;
    let attempt = record
        .attempt
        .as_ref()
        .ok_or(CannotRecord::OutcomeWithoutAttempt)?;
    Ok(envelope(
        outcome.outcome_id,
        record.proposal.proposal_id,
        attempt.attempt_id,
        Kind::Outcome,
        &LifecycleStep::Concluded {
            outcome: Box::new(outcome.clone()),
        },
        now,
    ))
}

/// Rebuild the lifecycle records a replay contains.
///
/// Permits are not rebuilt and cannot be: see this module's header. A record whose proposal was
/// written but whose decision was not is left out rather than completed with a guess — a proposal
/// nobody decided is exactly the state a reader needs to be able to see, and inventing a verdict for
/// it would replace *we do not know* with *it was refused*.
///
/// # Errors
///
/// Returns [`CannotReplay`] when a decision cites a proposal the replay does not contain. That is a
/// gap in the record rather than a malformed one, and it is reported rather than skipped, because a
/// decision whose proposal is missing is the one case where reading on would build a record whose
/// authority nobody can trace.
pub fn replay(envelopes: &[CanonicalEnvelope]) -> Result<Vec<ActionRecord>, CannotReplay> {
    let mut proposals: Vec<(Uuid, ActionProposal)> = Vec::new();
    let mut records = Vec::new();

    for envelope in envelopes {
        if envelope.origin_organ != ORIGIN {
            continue;
        }
        let Ok(step) = ciborium::from_reader::<LifecycleStep, _>(envelope.payload.as_slice())
        else {
            // Not every unreadable payload is a fault: a Journal written by a later build may carry
            // steps this one has no type for. What must not happen is treating one as something it
            // is not, so it is passed over rather than guessed at.
            continue;
        };
        match step {
            LifecycleStep::Proposed { proposal } => {
                proposals.push((envelope.correlation_id, *proposal));
            }
            LifecycleStep::Objected { .. } => {}
            LifecycleStep::Started { execution } => {
                if let Some(record) = records.iter_mut().find(|record: &&mut ActionRecord| {
                    record.proposal.proposal_id == execution.proposal_id
                }) {
                    record.execution_started = Some(*execution);
                }
            }
            LifecycleStep::Attempted { attempt } => {
                if let Some(record) = records.iter_mut().find(|record: &&mut ActionRecord| {
                    record.proposal.proposal_id == attempt.proposal_id
                }) {
                    record.attempt = Some(*attempt);
                }
            }
            LifecycleStep::Concluded { outcome } => {
                if let Some(record) = records.iter_mut().find(|record: &&mut ActionRecord| {
                    record.proposal.proposal_id == outcome.proposal_id
                }) {
                    record.outcome = Some(*outcome);
                }
            }
            LifecycleStep::Decided { decision, checks } => {
                let proposal = proposals
                    .iter()
                    .find(|(episode, _)| *episode == envelope.correlation_id)
                    .map(|(_, proposal)| proposal.clone())
                    .ok_or(CannotReplay::DecisionWithoutProposal(
                        envelope.correlation_id,
                    ))?;
                records.push(ActionRecord {
                    proposal,
                    checks,
                    attempt: None,
                    outcome: None,
                    // A permit that existed is gone with the process that held it, and saying so is
                    // the truthful reading: nothing here may be claimed.
                    permit_id: None,
                    execution_started: None,
                    decision: *decision,
                });
            }
        }
    }
    Ok(records)
}

fn execution_started_message_id(attempt_id: Uuid) -> Uuid {
    Uuid::new_v5(&EXECUTION_STARTED_NAMESPACE, attempt_id.as_bytes())
}

/// Whether this decision authorized anything, for a reader that has only the record.
#[must_use]
pub fn was_granted(record: &ActionRecord) -> bool {
    record.decision.verdict.permits_execution()
}

fn envelope(
    message_id: Uuid,
    correlation_id: Uuid,
    causation_id: Uuid,
    kind: Kind,
    step: &LifecycleStep,
    now: OffsetDateTime,
) -> CanonicalEnvelope {
    let mut payload = Vec::new();
    ciborium::into_writer(step, &mut payload).expect("a lifecycle step encodes");
    CanonicalEnvelope {
        schema_version: SCHEMA,
        message_id,
        correlation_id,
        causation_id,
        origin_organ: ORIGIN.to_owned(),
        origin_node: String::new(),
        kind: kind as u16,
        wall_time_ms: i64::try_from(now.unix_timestamp_nanos() / 1_000_000).unwrap_or(i64::MAX),
        monotonic_time: 0,
        logical_clock: 1,
        confidence: 1.0,
        evidence: Vec::new(),
        payload,
        // Node: what this host was asked to do to itself is about this host.
        privacy: 1,
        capability_scope: String::new(),
        sealed: false,
        key_domain_id: Uuid::nil(),
        key_epoch: 0,
        retention_class: 2,
        retention_policy_version: 0,
        retain_until_ms: 0,
        // Operational rather than personal. What a host was asked to restart is not a fact about
        // whoever asked.
        sensitivity: 0,
    }
}

#[cfg(test)]
mod tests {
    use cybou_protocol::action::{
        ActionProposal, Agreement, AttemptReport, AuthorizationDecision, AuthorizationVerdict,
        Proposer, Relief, RiskLevel,
    };

    use super::*;

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    const CAUSE: Uuid = Uuid::from_u128(0x0a00);

    fn record(passed: bool) -> ActionRecord {
        let proposal_id = Uuid::from_u128(0x0a01);
        ActionRecord {
            proposal: ActionProposal {
                proposal_id,
                proposed_by: Proposer::Mind,
                cause_id: Some(CAUSE),
                intent: "reload the deployed application".to_owned(),
                operation: "service.restart".to_owned(),
                target_resource: "systemd:nginx.service".to_owned(),
                parameters: Vec::new(),
                risk_level: RiskLevel::Medium,
                reversible: true,
                proposed_at: at(0),
            },
            checks: vec![
                CriticismCheck {
                    rule_id: "evidence-is-strong".to_owned(),
                    description: "The finding rests on more than one observation.".to_owned(),
                    passed: true,
                    objection: None,
                },
                CriticismCheck {
                    rule_id: "executor-adapter-exists".to_owned(),
                    description: "The first executor has a typed adapter.".to_owned(),
                    passed,
                    objection: (!passed).then(|| "no adapter".to_owned()),
                },
            ],
            decision: AuthorizationDecision {
                decision_id: Uuid::from_u128(0x0a02),
                proposal_id,
                verdict: AuthorizationVerdict::Granted,
                checked_capabilities: vec!["service.restart".to_owned()],
                decided_at: at(1),
            },
            permit_id: Some(Uuid::from_u128(0x0a03)),
            execution_started: None,
            attempt: None,
            outcome: None,
        }
    }

    #[test]
    fn a_decided_action_becomes_one_causal_episode() {
        // One episode, so a reader following causation arrives at the decision from the proposal
        // without having to know that Action1 exists.
        let written = contributions(&record(true), at(2)).expect("it has a cause");

        assert_eq!(written.len(), 2, "a passing check is not an objection");
        assert_eq!(written[0].kind, Kind::PlanProposal as u16);
        assert_eq!(written[1].kind, Kind::Decision as u16);
        assert!(
            written
                .iter()
                .all(|envelope| envelope.correlation_id == Uuid::from_u128(0x0a01)),
            "every step belongs to the proposal's episode"
        );
        assert_eq!(
            written[0].causation_id, CAUSE,
            "the proposal cites the finding that gave rise to it, because it is not a root"
        );
        assert_eq!(written[1].causation_id, written[0].message_id);
    }

    #[test]
    fn only_a_criticism_that_failed_becomes_an_objection() {
        // A check that passed is not an objection to anything. Writing one would fill the record
        // with disagreement nobody expressed.
        let written = contributions(&record(false), at(2)).expect("it has a cause");

        let objections: Vec<&CanonicalEnvelope> = written
            .iter()
            .filter(|envelope| envelope.kind == Kind::Objection as u16)
            .collect();
        assert_eq!(objections.len(), 1);
        assert_eq!(objections[0].causation_id, written[0].message_id);
        assert_eq!(
            written.last().expect("a decision").causation_id,
            objections[0].message_id,
            "the decision follows what was said against the proposal"
        );
    }

    fn attempted() -> ExecutionAttempt {
        ExecutionAttempt {
            attempt_id: Uuid::from_u128(0x0a04),
            proposal_id: Uuid::from_u128(0x0a01),
            decision_id: Uuid::from_u128(0x0a02),
            operation: "service.restart".to_owned(),
            target_resource: "systemd:nginx.service".to_owned(),
            report: AttemptReport::Completed,
            body_readings: Vec::new(),
            started_at: at(3),
            ended_at: Some(at(4)),
        }
    }

    fn started() -> ExecutionStarted {
        let attempt = attempted();
        ExecutionStarted {
            attempt_id: attempt.attempt_id,
            proposal_id: attempt.proposal_id,
            decision_id: attempt.decision_id,
            operation: attempt.operation,
            target_resource: attempt.target_resource,
            started_at: attempt.started_at,
        }
    }

    fn concluded() -> ActionOutcome {
        ActionOutcome {
            outcome_id: Uuid::from_u128(0x0a05),
            attempt_id: Uuid::from_u128(0x0a04),
            proposal_id: Uuid::from_u128(0x0a01),
            cause_id: Some(CAUSE),
            reported: AttemptReport::Completed,
            observed: Relief::Relieved,
            agreement: Agreement::Agree,
            rollback_available: true,
            concluded_at: at(4),
        }
    }

    #[test]
    fn an_episode_runs_from_the_finding_to_what_the_host_saw_afterwards() {
        // The question durable authorization only half answered. "Why was this allowed" was
        // recoverable; "was it done, and what did the host independently see" was not.
        let mut whole = record(true);
        whole.execution_started = Some(started());
        whole.attempt = Some(attempted());
        whole.outcome = Some(concluded());
        let written = contributions(&whole, at(5)).expect("it has a cause");

        let kinds: Vec<u16> = written.iter().map(|envelope| envelope.kind).collect();
        assert_eq!(
            kinds,
            vec![
                Kind::PlanProposal as u16,
                Kind::Decision as u16,
                Kind::Intention as u16,
                Kind::Intention as u16,
                Kind::Outcome as u16,
            ]
        );
        // Each cites the one before, so a reader arrives at the outcome from the finding without
        // knowing which organ wrote any of it.
        for pair in written.windows(2) {
            assert_eq!(pair[1].causation_id, pair[0].message_id);
        }
        assert!(
            written
                .iter()
                .all(|envelope| envelope.correlation_id == whole.proposal.proposal_id)
        );
    }

    #[test]
    fn what_was_reported_and_what_was_seen_stay_two_accounts() {
        // The whole value of re-observation is that they can disagree. One contribution carrying
        // both would delete the disagreement, which is the only part that could ever surprise
        // anybody.
        let mut disagreeing = record(true);
        disagreeing.execution_started = Some(started());
        disagreeing.attempt = Some(attempted());
        disagreeing.outcome = Some(ActionOutcome {
            observed: Relief::StillPresent,
            agreement: Agreement::Disagree {
                about: "the executor reported completion and the service is still down".to_owned(),
            },
            ..concluded()
        });

        let written = contributions(&disagreeing, at(5)).expect("written");
        let replayed = replay(&written).expect("reads back");
        let outcome = replayed[0].outcome.as_ref().expect("an outcome");

        assert_eq!(outcome.reported, AttemptReport::Completed);
        assert_eq!(outcome.observed, Relief::StillPresent);
        assert!(
            matches!(outcome.agreement, Agreement::Disagree { .. }),
            "the executor said it completed and the readings say otherwise, and both survive"
        );
    }

    #[test]
    fn a_decision_nobody_acted_on_stays_one() {
        // Absent is a real answer and a common one. Filling it in would answer "was it done" with a
        // guess.
        let written = contributions(&record(true), at(2)).expect("written");
        let replayed = replay(&written).expect("reads back");

        assert!(replayed[0].attempt.is_none());
        assert!(replayed[0].outcome.is_none());
    }

    #[test]
    fn an_attempt_survives_the_restart_that_loses_the_permit() {
        // The permit is gone by design; what was done with it is not.
        let mut whole = record(true);
        whole.execution_started = Some(started());
        whole.attempt = Some(attempted());
        whole.outcome = Some(concluded());

        let replayed = replay(&contributions(&whole, at(5)).expect("written")).expect("reads back");
        assert_eq!(
            replayed[0].attempt.as_ref().expect("an attempt").attempt_id,
            attempted().attempt_id
        );
        assert_eq!(replayed[0].permit_id, None);
    }

    #[test]
    fn the_permit_is_never_written_down() {
        // A durable record of a single-use sixty-second capability would be a durable record of a
        // key, and one whose presence could be mistaken for the authority itself.
        let record = record(true);
        let permit = record.permit_id.expect("a permit").to_string();
        for envelope in contributions(&record, at(2)).expect("it has a cause") {
            let rendered = String::from_utf8_lossy(&envelope.payload).into_owned();
            assert!(!rendered.contains(&permit));
        }
    }

    #[test]
    fn a_lifecycle_survives_the_wire_and_comes_back_whole() {
        // The point of the exercise: this is what a restarted Action1 reads.
        let original = record(false);
        let written = contributions(&original, at(2)).expect("it has a cause");
        let replayed = replay(&written).expect("replays");

        assert_eq!(replayed.len(), 1);
        assert_eq!(replayed[0].proposal, original.proposal);
        assert_eq!(replayed[0].checks, original.checks);
        assert_eq!(replayed[0].decision, original.decision);
        assert_eq!(
            replayed[0].permit_id, None,
            "an authorization nobody claimed was not claimed"
        );
    }

    #[test]
    fn a_proposal_nobody_decided_is_left_as_one() {
        // Completing it with a guess would replace "we do not know" with "it was refused", which is
        // the one substitution a record of authority must never make.
        let written = contributions(&record(true), at(2)).expect("it has a cause");
        let only_proposal = &written[..1];

        assert!(replay(only_proposal).expect("replays").is_empty());
    }

    #[test]
    fn a_decision_whose_proposal_is_missing_is_reported_rather_than_skipped() {
        // Reading on would build a record whose authority nobody can trace back to a request.
        let written = contributions(&record(true), at(2)).expect("it has a cause");
        let orphan = &written[1..];

        assert_eq!(
            replay(orphan),
            Err(CannotReplay::DecisionWithoutProposal(Uuid::from_u128(
                0x0a01
            )))
        );
    }

    #[test]
    fn a_proposal_with_no_cause_is_refused_rather_than_submitted_to_be_rejected() {
        // The failure this returns a Result to make visible. Before, an unciteable lifecycle was
        // submitted anyway, refused by the Journal, and logged as best effort — so the whole feature
        // was inert and nothing said so.
        let mut uncaused = record(true);
        uncaused.proposal.cause_id = None;

        assert_eq!(
            contributions(&uncaused, at(2)),
            Err(CannotRecord::ProposalHasNoCause(
                uncaused.proposal.proposal_id
            ))
        );
    }

    #[test]
    fn contributions_from_another_organ_are_not_read_as_lifecycle_steps() {
        // A replay is of the whole Journal. Everything in it that is not this organ's is somebody
        // else's, and reading it here would be this organ inventing actions nobody proposed.
        let mut written = contributions(&record(true), at(2)).expect("it has a cause");
        for envelope in &mut written {
            envelope.origin_organ = "intentiond".to_owned();
        }
        assert!(replay(&written).expect("replays").is_empty());
    }
}
