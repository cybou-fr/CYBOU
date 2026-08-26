// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What a restarted Action1 still knows.
//!
//! The unit tests in `journal` prove a hand-built record round-trips. This one starts from the real
//! lifecycle — a finding, criticised and decided by an actual `ActionCore` — writes it, drops the
//! owner entirely, and asks a fresh one what happened. That is the thing a person needs a month
//! later, and it is the thing an in-memory owner could not do.

use cybou_actiond::{ActionCore, journal};
use cybou_protocol::action::{AttemptReport, AuthorizationVerdict};
use cybou_protocol::telemetry::{EvidenceStrength, Finding, MetricKey, Subject, SystemInsight};
use cybou_remediation::initiative::{Initiative, Settled, Tried, initiative};
use cybou_remediation::{Operation, StandingPolicy};
use time::OffsetDateTime;
use uuid::Uuid;

fn at() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_787_000_000).expect("a fixed instant")
}

fn insight() -> SystemInsight {
    SystemInsight {
        insight_id: Uuid::from_u128(0x0b01),
        finding: Finding::ServiceFailure,
        about: Some(MetricKey::named(
            Subject::ServiceActive,
            "nginx.service".to_owned(),
        )),
        because: Vec::new(),
        strength: EvidenceStrength::Strong,
        concluded_at: at(),
        since: at(),
    }
}

fn policy() -> StandingPolicy {
    let mut policy = StandingPolicy::nothing_pre_authorized();
    policy.pre_authorized.push(Operation::RestartService);
    policy
}

#[test]
fn a_decided_action_outlives_the_process_that_decided_it() {
    let decided = {
        let core = ActionCore::new(policy());
        core.evaluate_insight(&insight(), "service.restart", at())
            .expect("a decision")
    };
    let proposal_id = decided.proposal.proposal_id;
    assert!(
        journal::was_granted(&decided),
        "the standing policy pre-authorized this operation"
    );

    // Everything the Journal would hold. The owner that decided it is gone by the next line.
    let written = journal::contributions(&decided, at()).expect("the finding caused it");

    let restarted = ActionCore::new(policy());
    assert!(
        restarted.record(proposal_id).is_none(),
        "a fresh owner starts knowing nothing"
    );

    let restored = journal::replay(&written).expect("the record reads back");
    restarted.restore(restored).expect("restores");

    let remembered = restarted
        .record(proposal_id)
        .expect("the restarted owner knows what it authorized");
    assert_eq!(remembered.proposal, decided.proposal);
    assert_eq!(remembered.checks, decided.checks);
    assert_eq!(remembered.decision, decided.decision);
    assert_eq!(remembered.decision.verdict, AuthorizationVerdict::Granted);
}

#[test]
fn the_permit_does_not_come_back_and_cannot_be_claimed() {
    // The half that must not survive. A single-use capability restored across a restart would be a
    // permission reissued by a crash, and the crash is not who a person granted it to.
    let core = ActionCore::new(policy());
    let decided = core
        .evaluate_insight(&insight(), "service.restart", at())
        .expect("a decision");
    let permit_id = decided.permit_id.expect("a granted decision issues one");

    let restarted = ActionCore::new(policy());
    restarted
        .restore(
            journal::replay(&journal::contributions(&decided, at()).expect("caused"))
                .expect("reads back"),
        )
        .expect("restores");

    assert!(
        restarted
            .record(decided.proposal.proposal_id)
            .expect("the decision is there")
            .permit_id
            .is_none(),
        "the record says no permit is outstanding"
    );
    assert!(
        restarted.claim_permit(permit_id, at()).is_err(),
        "a permit issued before the restart is not claimable after it"
    );
}

#[test]
fn a_refusal_is_remembered_as_a_refusal() {
    // Why nothing happened is as much a question as why something did, and an owner that only
    // recorded the actions it authorized could not answer the first one.
    let core = ActionCore::new(StandingPolicy::nothing_pre_authorized());
    let decided = core
        .evaluate_insight(&insight(), "service.restart", at())
        .expect("a decision was still reached");
    assert!(!journal::was_granted(&decided));

    let restarted = ActionCore::new(policy());
    restarted
        .restore(
            journal::replay(&journal::contributions(&decided, at()).expect("caused"))
                .expect("reads back"),
        )
        .expect("restores");

    let remembered = restarted
        .record(decided.proposal.proposal_id)
        .expect("the refusal is there");
    assert_eq!(remembered.decision.verdict, decided.decision.verdict);
    assert!(
        !journal::was_granted(&remembered),
        "a refusal restored under a laxer policy is still a refusal"
    );
}

#[test]
fn effect_may_have_happened_then_reply_was_lost_so_recovery_never_repeats_it() {
    let core = ActionCore::new(policy());
    let decided = core
        .evaluate_insight(&insight(), "service.restart", at())
        .expect("a decision");
    core.claim_permit(decided.permit_id.expect("permit"), at())
        .expect("claim establishes the durable execution boundary");
    let started = core
        .record(decided.proposal.proposal_id)
        .expect("record")
        .execution_started
        .expect("execution may now begin");

    // The Body effect may happen here. No ExecutionAttempt reply ever reaches Action1.
    let written = journal::contributions(
        &core.record(decided.proposal.proposal_id).expect("record"),
        at(),
    )
    .expect("the start is journalled");
    let restarted = ActionCore::new(policy());
    restarted
        .restore(journal::replay(&written).expect("start replays"))
        .expect("restores");

    let recovered = restarted
        .episode_for_cause(insight().insight_id)
        .expect("a started execution counts as an attempted episode");
    let attempt = recovered
        .attempt
        .expect("recovery materializes uncertainty");
    assert_eq!(attempt.attempt_id, started.attempt_id);
    assert_eq!(attempt.report, AttemptReport::DidNotFinish);
    assert_eq!(
        initiative(
            Some(&insight()),
            Some(&Tried {
                attempt,
                outcome: None,
            }),
        ),
        Initiative::Leave {
            because: Settled::OutcomeUnknown,
        },
        "a lost report is never permission for a second Body mutation"
    );
}
