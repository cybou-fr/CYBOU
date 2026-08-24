// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Deciding whether an action worked, from readings rather than from what it said about itself.
//!
//! This is the last stage of `observe -> understand -> remember -> diagnose -> explain -> propose ->
//! authorize -> act -> observe outcome`, and it is written before the stage before it exists. That
//! is deliberate, for the same reason the gate was written before the executor: the natural shape of
//! an executor is one that returns whether it worked, and an executor written first arrives with
//! that answer already built into its return type. Written second, it arrives to find that its own
//! report is one of two fields and not the deciding one.
//!
//! ## An exit code is not an outcome
//!
//! `apt clean` exits zero on a filesystem that is still full. A unit restarts successfully and dies
//! four seconds later. A cache is emptied and the thing filling the disk fills it again by morning.
//! In every case the operation is entitled to say it completed, and in every case the condition it
//! was proposed to relieve is still there.
//!
//! So what the executor said is recorded as *what was claimed*, and what decides is a comparison of
//! findings taken before and after — by the telemetry organ, which did not carry the action out and
//! has no notion that one was carried out at all.
//!
//! ## Not being able to tell is an answer
//!
//! Three of the states here are ways of not knowing, and none of them collapses into failure. A
//! watched thing that went unreadable, a restart read one second after it began, an operation that
//! was declined and never ran — reporting any of those as *it did not work* would be reporting a
//! conclusion the readings do not support, on the surface a person uses to decide what to try next.
//!
//! **Nothing here can carry anything out.** It compares two sets of findings and returns a value.

use cybou_protocol::action::{
    ActionOutcome, ActionProposal, Agreement, AttemptReport, CannotTell, ExecutionAttempt, Relief,
};
use cybou_protocol::telemetry::{SystemInsight, WatchedResource};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

/// How long after an attempt the readings are allowed to be inconclusive.
///
/// A restart takes longer than a sample interval, and a filesystem measure is taken every ten
/// seconds. Reading immediately afterwards and declaring failure would condemn every operation that
/// is not instantaneous — which is all of the interesting ones.
///
/// Ninety seconds is roughly nine samples at the rate this host observes itself. Long enough for
/// anything the closed operation set contains to have taken effect and been seen, short enough that
/// a person waiting for an answer gets one.
pub const TOO_SOON_AFTER: Duration = Duration::seconds(90);

/// What the host saw on either side of an attempt.
///
/// Gathered by the telemetry organ, which did not carry the action out and has no notion that one
/// happened. A struct rather than four arguments because it is one thing — a look taken
/// independently — and because four loose slices at a call site are four chances to pass the second
/// set where the first belongs, which would report every action as having changed nothing.
#[derive(Clone, Copy, Debug)]
pub struct Reobservation<'a> {
    /// What the host concluded about itself before the attempt.
    pub before: &'a [SystemInsight],
    /// What it concludes now.
    pub after: &'a [SystemInsight],
    /// What it could see when `after` was taken.
    ///
    /// Needed because a finding can vanish for two reasons, and they are opposites: the condition
    /// cleared, or nothing could read the thing it was about.
    pub watched_after: &'a [WatchedResource],
    /// When this look was taken.
    pub at: OffsetDateTime,
}

/// Conclude what an attempt actually did.
///
/// The attempt is consulted only for what it claimed; it is not consulted for whether it worked.
///
/// The identity is supplied rather than generated, for the reason it is supplied everywhere else in
/// this tree: something that reached for a random source would be reaching for something.
#[must_use]
pub fn observe_outcome(
    attempt: &ExecutionAttempt,
    proposal: &ActionProposal,
    cause: Option<&SystemInsight>,
    seen: &Reobservation<'_>,
    outcome_id: Uuid,
) -> ActionOutcome {
    let now = seen.at;
    let observed = relief(attempt, cause, seen);
    let agreement = agreement(&attempt.report, &observed);
    ActionOutcome {
        outcome_id,
        attempt_id: attempt.attempt_id,
        proposal_id: proposal.proposal_id,
        cause_id: proposal.cause_id,
        reported: attempt.report.clone(),
        observed,
        agreement,
        // Reversible in principle, and something was in fact attempted. An operation that never ran
        // has nothing to undo, and saying rollback is available would offer to undo nothing.
        rollback_available: proposal.reversible && attempt.report.was_attempted(),
        concluded_at: now,
    }
}

/// What the readings say about the condition the action claimed to relieve.
fn relief(
    attempt: &ExecutionAttempt,
    cause: Option<&SystemInsight>,
    seen: &Reobservation<'_>,
) -> Relief {
    if !attempt.report.was_attempted() {
        return Relief::NotEstablished {
            because: CannotTell::NothingWasAttempted,
        };
    }

    // A proposal that named no cause has nothing to be checked against. Treated as unreadable
    // rather than as relieved, because "I cannot tell" is the true description of a comparison with
    // nothing on one side of it.
    let Some(cause) = cause else {
        return Relief::NotEstablished {
            because: CannotTell::NotReadAfterwards,
        };
    };

    // Measured from when it stopped, or from when it started if it never did. An operation still
    // running has not had its effects seen yet either.
    let since = attempt.ended_at.unwrap_or(attempt.started_at);
    if seen.at - since < TOO_SOON_AFTER {
        return Relief::NotEstablished {
            because: CannotTell::TooSoon,
        };
    }

    // A finding that cites nothing cannot be checked against anything, and `all` over an empty
    // list is *true* — so this used to answer `Relieved` for it, by arithmetic rather than by
    // evidence. That was reachable by the ordinary route: a categorical finding needs no baseline,
    // so the first reading of a filesystem at 97% produced a `StorageExhaustion` citing nothing at
    // all, and an action against it was reported as having worked whatever happened.
    //
    // Fixed at the source — a finding now cites its observation whether or not a baseline exists —
    // and refused here as well, because a vacuous truth that says "it worked" must not be one
    // upstream change away from returning.
    if cause.because.is_empty() {
        return Relief::NotEstablished {
            because: CannotTell::NotReadAfterwards,
        };
    }

    // Every reading the finding rested on has to be readable again, or the two sides are not
    // comparable. One unreadable measure among them is enough: the finding was concluded from all
    // of them, and its absence afterwards could be the absence of the measurement rather than the
    // absence of the condition.
    let all_readable = cause.because.iter().all(|evidence| {
        seen.watched_after
            .iter()
            .any(|resource| resource.key == evidence.key && resource.state.is_observed())
    });
    if !all_readable {
        return Relief::NotEstablished {
            because: CannotTell::NotReadAfterwards,
        };
    }

    let Some(still) = same_condition(cause, seen.after) else {
        return Relief::Relieved;
    };

    // Worse rather than merely still present, which is the one shape that argues against trying the
    // same thing again. Compared against the finding as it was before rather than against the cause
    // as recorded, so a condition that drifted between the diagnosis and the attempt is measured
    // from where it actually was.
    let was = same_condition(cause, seen.before).unwrap_or(cause);
    match (worst_deviation(still), worst_deviation(was)) {
        (Some(after), Some(before)) if after > before => Relief::Worse,
        // Either side without a baseline. *Worse* is a comparison against what is ordinary here,
        // and a host that has not established one cannot make it — so the answer is the weaker of
        // the two claims, which is still present. Guessing from the raw values would need to know
        // which direction is bad, and that is the threshold's business rather than this module's.
        _ => Relief::StillPresent,
    }
}

/// The same condition, in a later set of findings.
///
/// Matched on what it is and what it is about rather than on identity. The identity a finding
/// carries is derived partly from when the condition began, so a condition that briefly cleared and
/// returned has a different one — and for this question that is a condition that is still present,
/// not a different condition that happens to look identical.
fn same_condition<'a>(
    cause: &SystemInsight,
    among: &'a [SystemInsight],
) -> Option<&'a SystemInsight> {
    among
        .iter()
        .find(|insight| insight.finding == cause.finding && insight.about == cause.about)
}

/// How far from ordinary the furthest of a finding's readings sits, if any of them says.
///
/// The furthest rather than the average, because a finding that cites two measures is as bad as the
/// worse of them: memory pressure easing while swap keeps climbing is not an improvement.
///
/// `None` when nothing it cites has a baseline. A categorical finding on a young host is the whole
/// case: it is a real finding, it is entitled to no opinion about what is unusual here, and a
/// caller that read a missing baseline as zero would compare a real deviation against a fabricated
/// one and call the result an improvement.
fn worst_deviation(insight: &SystemInsight) -> Option<f64> {
    insight
        .because
        .iter()
        .filter_map(|evidence| evidence.deviation)
        .map(|deviation| deviation.spreads_away)
        .fold(None, |worst: Option<f64>, spreads| {
            Some(worst.map_or(spreads, |worst| worst.max(spreads)))
        })
}

/// Whether the claim and the readings tell the same story.
fn agreement(reported: &AttemptReport, observed: &Relief) -> Agreement {
    match (reported, observed) {
        // The case this whole module exists for. Something said it worked, and the condition it
        // addressed is still there — which says the operation is not the remedy somebody thought it
        // was, and is invisible to anything that records only what the executor claimed.
        (AttemptReport::Completed, Relief::StillPresent) => Agreement::Disagree {
            about: "the operation reported success and the condition is still present".to_owned(),
        },
        (AttemptReport::Completed, Relief::Worse) => Agreement::Disagree {
            about: "the operation reported success and the condition is further from ordinary"
                .to_owned(),
        },
        // The mirror case, and worth naming rather than passing over. Something reported failure
        // and the condition cleared: either it did more than it admitted, or something else
        // relieved it, and both are reasons not to take the failure at face value.
        (AttemptReport::Failed { .. } | AttemptReport::DidNotFinish, Relief::Relieved) => {
            Agreement::Disagree {
                about: "the operation did not report success and the condition cleared".to_owned(),
            }
        }
        // One of the two has nothing to say. Comparing them would be inventing a comparison.
        (_, Relief::NotEstablished { .. }) => Agreement::NotComparable,
        _ => Agreement::Agree,
    }
}

#[cfg(test)]
mod tests {
    use cybou_protocol::action::Proposer;
    use cybou_protocol::telemetry::{
        Deviation, EvidenceStrength, Finding, InsightEvidence, MetricKey, Subject, Watching,
    };

    use super::*;

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    fn insight(spreads_away: f64) -> SystemInsight {
        SystemInsight {
            insight_id: Uuid::from_u128(1),
            finding: Finding::StorageExhaustion,
            about: None,
            because: vec![InsightEvidence {
                key: MetricKey::host(Subject::RootFilesystemUsed),
                observed: 0.96,
                deviation: Some(Deviation {
                    ordinary: 0.62,
                    spread: 0.01,
                    observed: 0.96,
                    spreads_away,
                }),
            }],
            strength: EvidenceStrength::Strong,
            concluded_at: at(0),
            since: at(-1000),
        }
    }

    fn readable() -> Vec<WatchedResource> {
        vec![WatchedResource {
            key: MetricKey::host(Subject::RootFilesystemUsed),
            state: Watching::Observed {
                value: 0.40,
                at: at(200),
            },
        }]
    }

    fn proposal() -> ActionProposal {
        ActionProposal {
            proposal_id: Uuid::from_u128(2),
            proposed_by: Proposer::Mind,
            cause_id: Some(Uuid::from_u128(1)),
            intent: "relieve storage.exhaustion".to_owned(),
            operation: "package.cache.clean".to_owned(),
            target_resource: "apt:archives".to_owned(),
            parameters: Vec::new(),
            risk_level: cybou_protocol::action::RiskLevel::Low,
            reversible: true,
            proposed_at: at(0),
        }
    }

    fn attempt(report: AttemptReport) -> ExecutionAttempt {
        ExecutionAttempt {
            attempt_id: Uuid::from_u128(3),
            proposal_id: Uuid::from_u128(2),
            decision_id: Uuid::from_u128(4),
            operation: "package.cache.clean".to_owned(),
            target_resource: "apt:archives".to_owned(),
            report,
            started_at: at(0),
            ended_at: Some(at(10)),
        }
    }

    fn conclude(
        report: AttemptReport,
        after: &[SystemInsight],
        watched: &[WatchedResource],
        now: OffsetDateTime,
    ) -> ActionOutcome {
        observe_outcome(
            &attempt(report),
            &proposal(),
            Some(&insight(30.0)),
            &Reobservation {
                before: &[insight(30.0)],
                after,
                watched_after: watched,
                at: now,
            },
            Uuid::from_u128(9),
        )
    }

    /// A finding with no baseline, as a fresh host produces on its first reading.
    fn categorical() -> SystemInsight {
        SystemInsight {
            insight_id: Uuid::from_u128(1),
            finding: Finding::StorageExhaustion,
            about: None,
            because: vec![InsightEvidence {
                key: MetricKey::host(Subject::RootFilesystemUsed),
                observed: 0.97,
                deviation: None,
            }],
            strength: EvidenceStrength::Strong,
            concluded_at: at(0),
            since: at(-1000),
        }
    }

    #[test]
    fn a_finding_with_no_baseline_is_still_checked_against_what_can_be_read() {
        // The regression this closes had two halves. A categorical finding cited nothing, and
        // `all()` over nothing is true — so a measure that went unreadable after the attempt was
        // read as a condition relieved, by arithmetic rather than by evidence. On a fresh VPS,
        // which is every VPS for its first six hours, this was the ordinary path.
        let unreadable = vec![WatchedResource {
            key: MetricKey::host(Subject::RootFilesystemUsed),
            state: Watching::ReadFailed { since: at(100) },
        }];
        let outcome = observe_outcome(
            &attempt(AttemptReport::Completed),
            &proposal(),
            Some(&categorical()),
            &Reobservation {
                before: &[categorical()],
                after: &[],
                watched_after: &unreadable,
                at: at(1000),
            },
            Uuid::from_u128(9),
        );
        assert_eq!(
            outcome.observed,
            Relief::NotEstablished {
                because: CannotTell::NotReadAfterwards
            },
            "an unread measure behind a baseline-free finding was reported as solved"
        );
    }

    #[test]
    fn a_finding_that_cites_nothing_at_all_establishes_nothing() {
        // Defence in depth. The source no longer produces one, and a vacuous truth that says "it
        // worked" must not be one upstream change away from returning.
        let mut cites_nothing = categorical();
        cites_nothing.because.clear();
        let outcome = observe_outcome(
            &attempt(AttemptReport::Completed),
            &proposal(),
            Some(&cites_nothing),
            &Reobservation {
                before: &[cites_nothing.clone()],
                after: &[],
                watched_after: &readable(),
                at: at(1000),
            },
            Uuid::from_u128(9),
        );
        assert!(
            matches!(outcome.observed, Relief::NotEstablished { .. }),
            "{:?}",
            outcome.observed
        );
    }

    #[test]
    fn a_condition_with_no_baseline_is_not_declared_worse_or_better() {
        // *Worse* is a comparison against what is ordinary here, and a host that has not
        // established one cannot make it. Reading a missing baseline as zero would compare a real
        // deviation against a fabricated one and call the result an improvement.
        let outcome = observe_outcome(
            &attempt(AttemptReport::Completed),
            &proposal(),
            Some(&categorical()),
            &Reobservation {
                before: &[categorical()],
                after: &[categorical()],
                watched_after: &readable(),
                at: at(1000),
            },
            Uuid::from_u128(9),
        );
        assert_eq!(outcome.observed, Relief::StillPresent);
    }

    #[test]
    fn an_operation_that_reported_success_while_nothing_changed_is_a_disagreement() {
        // The single most important thing this path can produce. `apt clean` exits zero on a
        // filesystem that is still full, and anything recording only the exit code records a
        // remedy that worked.
        let outcome = conclude(
            AttemptReport::Completed,
            &[insight(30.0)],
            &readable(),
            at(1000),
        );
        assert_eq!(outcome.observed, Relief::StillPresent);
        assert!(
            matches!(outcome.agreement, Agreement::Disagree { .. }),
            "{:?}",
            outcome.agreement
        );
    }

    #[test]
    fn an_operation_that_worked_and_said_so_agrees() {
        let outcome = conclude(AttemptReport::Completed, &[], &readable(), at(1000));
        assert_eq!(outcome.observed, Relief::Relieved);
        assert_eq!(outcome.agreement, Agreement::Agree);
    }

    #[test]
    fn a_failure_that_cleared_the_condition_is_also_a_disagreement() {
        // Either it did more than it admitted, or something else relieved it. Both are reasons not
        // to take the failure at face value, and both are lost if only success is checked.
        let outcome = conclude(
            AttemptReport::Failed {
                because: "exit 100".to_owned(),
            },
            &[],
            &readable(),
            at(1000),
        );
        assert_eq!(outcome.observed, Relief::Relieved);
        assert!(matches!(outcome.agreement, Agreement::Disagree { .. }));
    }

    #[test]
    fn a_condition_further_from_ordinary_is_not_merely_still_present() {
        // The one shape that argues against trying the same thing again.
        let outcome = conclude(
            AttemptReport::Completed,
            &[insight(48.0)],
            &readable(),
            at(1000),
        );
        assert_eq!(outcome.observed, Relief::Worse);
    }

    #[test]
    fn an_attempt_read_immediately_afterwards_has_not_established_anything() {
        // A restart takes longer than a sample interval. Declaring failure here would condemn every
        // operation that is not instantaneous, which is all of the interesting ones.
        let outcome = conclude(AttemptReport::Completed, &[], &readable(), at(20));
        assert_eq!(
            outcome.observed,
            Relief::NotEstablished {
                because: CannotTell::TooSoon
            }
        );
        assert_eq!(outcome.agreement, Agreement::NotComparable);
    }

    #[test]
    fn a_measure_that_went_unreadable_does_not_count_as_a_condition_relieved() {
        // The failure mode this closes is the worst one available here: a finding disappears
        // because nothing could read the thing it was about, and the absence of the measurement is
        // reported as the absence of the problem.
        let unreadable = vec![WatchedResource {
            key: MetricKey::host(Subject::RootFilesystemUsed),
            state: Watching::ReadFailed { since: at(100) },
        }];
        let outcome = conclude(AttemptReport::Completed, &[], &unreadable, at(1000));
        assert_eq!(
            outcome.observed,
            Relief::NotEstablished {
                because: CannotTell::NotReadAfterwards
            },
            "an unread measure was reported as a solved problem"
        );
    }

    #[test]
    fn an_operation_that_was_declined_has_nothing_to_have_relieved() {
        let outcome = conclude(
            AttemptReport::Refused {
                because: "not authorized".to_owned(),
            },
            &[],
            &readable(),
            at(1000),
        );
        assert_eq!(
            outcome.observed,
            Relief::NotEstablished {
                because: CannotTell::NothingWasAttempted
            }
        );
        assert!(
            !outcome.rollback_available,
            "offered to undo something that never happened"
        );
    }

    #[test]
    fn a_condition_that_cleared_and_returned_is_still_present() {
        // Its identity differs, because identity is derived partly from when the condition began.
        // For this question it is the same condition — matching on identity would report a
        // filesystem that filled again within the minute as a filesystem that was relieved.
        let mut returned = insight(30.0);
        returned.insight_id = Uuid::from_u128(77);
        returned.since = at(500);

        let outcome = conclude(
            AttemptReport::Completed,
            std::slice::from_ref(&returned),
            &readable(),
            at(1000),
        );
        assert_eq!(outcome.observed, Relief::StillPresent);
    }

    #[test]
    fn a_finding_about_one_named_thing_is_not_matched_against_another() {
        // Two certificates, one renewed. Matching on the kind alone would report the renewed one as
        // still expiring, or the untouched one as fixed, depending which came first.
        let about = |name: &str| SystemInsight {
            insight_id: Uuid::from_u128(1),
            finding: Finding::CertificateExpiring,
            about: Some(MetricKey::named(
                Subject::CertificateDaysRemaining,
                name.to_owned(),
            )),
            because: vec![InsightEvidence {
                key: MetricKey::named(Subject::CertificateDaysRemaining, name.to_owned()),
                observed: 0.96,
                deviation: Some(Deviation {
                    ordinary: 60.0,
                    spread: 1.0,
                    observed: 3.0,
                    spreads_away: 57.0,
                }),
            }],
            strength: EvidenceStrength::Strong,
            concluded_at: at(0),
            since: at(-1000),
        };
        let watched = vec![WatchedResource {
            key: MetricKey::named(
                Subject::CertificateDaysRemaining,
                "/etc/ssl/a.pem".to_owned(),
            ),
            state: Watching::Observed {
                value: 90.0,
                at: at(200),
            },
        }];

        // The one that was acted on is gone; the other is not.
        let outcome = observe_outcome(
            &attempt(AttemptReport::Completed),
            &proposal(),
            Some(&about("/etc/ssl/a.pem")),
            &Reobservation {
                before: &[about("/etc/ssl/a.pem"), about("/etc/ssl/b.pem")],
                after: &[about("/etc/ssl/b.pem")],
                watched_after: &watched,
                at: at(1000),
            },
            Uuid::from_u128(9),
        );
        assert_eq!(
            outcome.observed,
            Relief::Relieved,
            "another certificate's finding was read as this one's"
        );
    }

    #[test]
    fn what_was_claimed_is_kept_beside_what_was_observed() {
        // Both, always. Keeping only the observation would lose the disagreement, which is the
        // thing worth knowing; keeping only the claim is what this module exists to stop.
        let outcome = conclude(
            AttemptReport::Completed,
            &[insight(30.0)],
            &readable(),
            at(1000),
        );
        assert_eq!(outcome.reported, AttemptReport::Completed);
        assert_eq!(outcome.observed, Relief::StillPresent);
        assert_eq!(outcome.cause_id, Some(Uuid::from_u128(1)));
    }
}
