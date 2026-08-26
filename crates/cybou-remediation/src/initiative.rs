// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! When a host may act on a finding, and when it must wait or stop.
//!
//! This crate's header says the decision to execute has no natural place to live, so it gets made by
//! whoever wires a proposer to an executor, on a working system, under pressure to make it work. That
//! is still true, and the wiring does not exist yet. This is the decision, written before the place
//! that would otherwise make it.
//!
//! Nothing here proposes, authorizes or executes. It answers one question about one finding:
//!
//! ```text
//! act        nothing has been tried, or what was tried was seen not to work and may be tried again
//! wait       something was tried and it is too soon to know
//! leave it   it worked, or trying again is not what a person would want
//! ```
//!
//! ## Why not simply act whenever a finding is present
//!
//! Because a finding is present *while the remedy is taking effect*. A restart takes longer than a
//! sample interval, so a host that acted on presence alone would restart a service, look, still see
//! it down, and restart it again — the loop that turns a self-maintaining host into an outage. The
//! whole reason [`crate::TOO_SOON_AFTER`] exists is that an answer taken too early is not an answer,
//! and acting on one is worse than not looking at all.
//!
//! ## Why a host stops rather than trying harder
//!
//! An action that was carried out, observed, and did not relieve the finding is evidence about the
//! remedy, not about the effort. Repeating it is the behaviour of something that cannot tell the
//! difference between *not yet* and *not this way*. So a remedy that was seen to fail is not offered
//! again for the same finding, and the host says so instead — which is the point at which a person is
//! genuinely needed, and the point a retry loop would hide.
//!
//! An attempt whose result nobody knows is treated as neither. `AttemptReport::DidNotFinish` means
//! something may well have happened, and doing it again is exactly the wrong response to *may*.

use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use cybou_protocol::action::{ActionOutcome, AttemptReport, ExecutionAttempt, Relief};
use cybou_protocol::telemetry::SystemInsight;

use crate::outcome::TOO_SOON_AFTER;

/// What is known about one earlier attempt on a finding.
#[derive(Clone, Debug, PartialEq)]
pub struct Tried {
    /// What was carried out.
    pub attempt: ExecutionAttempt,
    /// What was concluded about it, once anybody has looked.
    pub outcome: Option<ActionOutcome>,
}

/// Whether a host may act on a finding now.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Initiative {
    /// Nothing stands in the way.
    Act,
    /// Something was tried and it is too soon to know whether it worked.
    ///
    /// Carries when it will be time, so a caller waits rather than polls a decision it could have
    /// been given once.
    Wait {
        /// When re-observation would mean anything.
        until: OffsetDateTime,
    },
    /// Acting again is not the right thing, and this is why.
    Leave {
        /// What to tell a person, in words that say what they would have to decide.
        because: Settled,
    },
}

/// Why a host is not going to act on this finding again.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Settled {
    /// It was tried, it was seen, and it worked.
    Relieved,
    /// It was tried, it was seen, and it did not work.
    ///
    /// The one that needs a person. Evidence about the remedy rather than about the effort, and a
    /// host that tried again would be unable to tell *not yet* from *not this way*.
    RemedyDidNotWork,
    /// It was tried and made things worse.
    MadeItWorse,
    /// Something ran and nobody knows how it ended.
    ///
    /// Doing it again is exactly the wrong response to *may have happened*.
    OutcomeUnknown,
    /// Nothing was carried out, and whatever declined it will decline it again.
    Refused,
}

impl Settled {
    /// How this reads to a person.
    #[must_use]
    pub const fn describe(&self) -> &'static str {
        match self {
            Self::Relieved => "the remedy worked",
            Self::RemedyDidNotWork => "the remedy ran and the finding is still here",
            Self::MadeItWorse => "the remedy ran and made it worse",
            Self::OutcomeUnknown => "something ran and nobody knows how it ended",
            Self::Refused => "nothing was carried out",
        }
    }

    /// Whether this is the kind of ending a person should be asked about.
    ///
    /// Success is not. The other four are: each is a host saying it has done what it knows how to do
    /// and the problem is still somebody's.
    #[must_use]
    pub const fn needs_a_person(&self) -> bool {
        !matches!(self, Self::Relieved)
    }
}

/// Whether this host may act on `finding` now, given what it already tried.
///
/// `tried` is what is known about the most recent attempt on this same finding, and `None` means
/// nothing has been. `finding` may itself be `None`, for an episode whose finding is no longer
/// reported — which is what success looks like. Same finding rather than same operation: a finding is identified by what it is
/// about and when it began, so a problem that cleared and came back is a new one and may be acted on
/// afresh.
#[must_use]
pub fn initiative(finding: Option<&SystemInsight>, tried: Option<&Tried>) -> Initiative {
    // The finding is optional because the two states a caller can be in are not the same: it may be
    // asking about something the host currently concludes, or about an episode it began before it
    // restarted, whose finding may since have gone — including because the remedy worked. Refusing
    // the second would leave an unfinished episode unfinishable.
    //
    // Nothing about the decision turns on the clock either: it is a function of what happened, and
    // when somebody asks is `wait_for`'s business.
    let _ = finding;
    let Some(tried) = tried else {
        return Initiative::Act;
    };

    match &tried.attempt.report {
        // Nothing ran. Whatever declined it — a missing adapter, a refused permit — will decline it
        // again, and trying repeatedly would be arguing with a policy rather than telling somebody.
        AttemptReport::Refused { .. } => {
            return Initiative::Leave {
                because: Settled::Refused,
            };
        }
        AttemptReport::DidNotFinish => {
            return Initiative::Leave {
                because: Settled::OutcomeUnknown,
            };
        }
        AttemptReport::Completed | AttemptReport::Failed { .. } => {}
    }

    let Some(outcome) = &tried.outcome else {
        // It ran and nobody has looked yet, so the answer is the same either way: wait for the
        // outcome. When the delay has passed, `wait_for` returns nothing left to wait — which reads
        // as *look now*, and looking is not this module's business to do.
        let since = tried.attempt.ended_at.unwrap_or(tried.attempt.started_at);
        return Initiative::Wait {
            until: since + TOO_SOON_AFTER,
        };
    };

    match &outcome.observed {
        Relief::Relieved => Initiative::Leave {
            because: Settled::Relieved,
        },
        Relief::Worse => Initiative::Leave {
            because: Settled::MadeItWorse,
        },
        Relief::StillPresent => Initiative::Leave {
            because: Settled::RemedyDidNotWork,
        },
        // Nobody could tell. Acting again on an answer that was never an answer would be treating
        // *we could not see* as *it did not work*, and those call for different things: one needs a
        // remedy, the other needs somebody to fix the looking.
        Relief::NotEstablished { .. } => Initiative::Leave {
            because: Settled::OutcomeUnknown,
        },
    }
}

/// How long a caller should wait before asking again, if it should.
#[must_use]
pub fn wait_for(initiative: &Initiative, now: OffsetDateTime) -> Option<Duration> {
    match initiative {
        Initiative::Wait { until } => Some((*until - now).max(Duration::ZERO)),
        Initiative::Act | Initiative::Leave { .. } => None,
    }
}

/// The identity of the finding an attempt was made about, for keeping the two together.
#[must_use]
pub const fn about(outcome: &ActionOutcome) -> Option<Uuid> {
    outcome.cause_id
}

#[cfg(test)]
mod tests {
    use cybou_protocol::action::Agreement;
    use cybou_protocol::telemetry::{
        EvidenceStrength, Finding, InsightEvidence, MetricKey, Subject,
    };

    use super::*;

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    fn finding() -> SystemInsight {
        SystemInsight {
            insight_id: Uuid::from_u128(0x0e01),
            finding: Finding::ServiceFailure,
            about: Some(MetricKey::named(
                Subject::ServiceActive,
                "nginx.service".to_owned(),
            )),
            because: vec![InsightEvidence {
                key: MetricKey::named(Subject::ServiceActive, "nginx.service".to_owned()),
                observed: 0.0,
                deviation: None,
            }],
            strength: EvidenceStrength::Strong,
            concluded_at: at(0),
            since: at(0),
        }
    }

    fn tried(report: &AttemptReport, observed: Option<Relief>) -> Tried {
        Tried {
            attempt: ExecutionAttempt {
                attempt_id: Uuid::from_u128(0x0e02),
                proposal_id: Uuid::from_u128(0x0e03),
                decision_id: Uuid::from_u128(0x0e04),
                operation: "service.restart".to_owned(),
                target_resource: "systemd:nginx.service".to_owned(),
                report: report.clone(),
                body_readings: Vec::new(),
                started_at: at(10),
                ended_at: Some(at(12)),
            },
            outcome: observed.map(|observed| ActionOutcome {
                outcome_id: Uuid::from_u128(0x0e05),
                attempt_id: Uuid::from_u128(0x0e02),
                proposal_id: Uuid::from_u128(0x0e03),
                cause_id: Some(Uuid::from_u128(0x0e01)),
                reported: report.clone(),
                observed,
                agreement: Agreement::NotComparable,
                rollback_available: true,
                concluded_at: at(120),
            }),
        }
    }

    #[test]
    fn a_finding_nobody_has_acted_on_may_be_acted_on() {
        assert_eq!(initiative(Some(&finding()), None), Initiative::Act);
    }

    #[test]
    fn a_host_waits_out_the_delay_rather_than_acting_on_a_finding_that_is_still_present() {
        // The loop that turns a self-maintaining host into an outage: a restart takes longer than a
        // sample interval, so acting on presence alone restarts, looks, sees it still down, and
        // restarts again.
        let recent = tried(&AttemptReport::Completed, None);
        let waiting = initiative(Some(&finding()), Some(&recent));

        match waiting {
            Initiative::Wait { until } => assert_eq!(until, at(12) + TOO_SOON_AFTER),
            other => panic!("acted while the remedy was still taking effect: {other:?}"),
        }
        assert_eq!(
            wait_for(&waiting, at(20)),
            Some(TOO_SOON_AFTER - Duration::seconds(8))
        );
    }

    #[test]
    fn a_remedy_that_was_seen_to_work_is_not_repeated() {
        let done = tried(&AttemptReport::Completed, Some(Relief::Relieved));
        assert_eq!(
            initiative(Some(&finding()), Some(&done)),
            Initiative::Leave {
                because: Settled::Relieved
            }
        );
        assert!(!Settled::Relieved.needs_a_person());
    }

    #[test]
    fn a_remedy_that_was_seen_not_to_work_is_not_tried_again() {
        // Evidence about the remedy, not about the effort. A host that tried again would be unable
        // to tell "not yet" from "not this way", and this is exactly the point where a person is
        // genuinely needed — the point a retry loop would hide.
        let failed = tried(&AttemptReport::Completed, Some(Relief::StillPresent));
        let settled = initiative(Some(&finding()), Some(&failed));

        assert_eq!(
            settled,
            Initiative::Leave {
                because: Settled::RemedyDidNotWork
            }
        );
        assert!(matches!(settled, Initiative::Leave { because } if because.needs_a_person()));
    }

    #[test]
    fn a_remedy_that_made_things_worse_is_told_apart_from_one_that_did_nothing() {
        // Different things to tell somebody, and a summary that called both "it did not work" would
        // lose the one that matters more.
        let worse = tried(&AttemptReport::Completed, Some(Relief::Worse));
        assert_eq!(
            initiative(Some(&finding()), Some(&worse)),
            Initiative::Leave {
                because: Settled::MadeItWorse
            }
        );
        assert_ne!(
            Settled::MadeItWorse.describe(),
            Settled::RemedyDidNotWork.describe()
        );
    }

    #[test]
    fn an_attempt_nobody_knows_the_end_of_is_not_repeated() {
        // "Something may well have happened" is exactly the wrong thing to answer by doing it again.
        let unknown = tried(&AttemptReport::DidNotFinish, None);
        assert_eq!(
            initiative(Some(&finding()), Some(&unknown)),
            Initiative::Leave {
                because: Settled::OutcomeUnknown
            }
        );
    }

    #[test]
    fn a_result_nobody_could_read_is_not_a_failed_remedy() {
        // "We could not see" and "it did not work" call for different things: one needs a remedy,
        // the other needs somebody to fix the looking.
        let blind = tried(
            &AttemptReport::Completed,
            Some(Relief::NotEstablished {
                because: cybou_protocol::action::CannotTell::NotReadAfterwards,
            }),
        );
        assert_eq!(
            initiative(Some(&finding()), Some(&blind)),
            Initiative::Leave {
                because: Settled::OutcomeUnknown
            }
        );
    }

    #[test]
    fn nothing_carried_out_is_not_tried_again_either() {
        // Whatever declined it will decline it again, and repeating would be arguing with a policy
        // instead of telling somebody about it.
        let refused = tried(
            &AttemptReport::Refused {
                because: "no adapter".to_owned(),
            },
            None,
        );
        assert_eq!(
            initiative(Some(&finding()), Some(&refused)),
            Initiative::Leave {
                because: Settled::Refused
            }
        );
    }

    #[test]
    fn an_episode_can_be_asked_about_after_its_finding_is_gone() {
        // What a driver has when it restarted mid-episode, and what success looks like: the remedy
        // worked, so the telemetry organ stopped reporting the finding. Refusing to answer would
        // leave the episode unfinishable, and the successful one is the one that would be lost.
        let recent = tried(&AttemptReport::Completed, None);
        assert!(matches!(
            initiative(None, Some(&recent)),
            Initiative::Wait { .. }
        ));
        assert_eq!(initiative(None, None), Initiative::Act);
    }

    #[test]
    fn nothing_here_asks_a_caller_to_poll() {
        // A waiting decision carries when the waiting is over, so a caller sleeps once rather than
        // asking a question it could have been answered fully the first time.
        let recent = tried(&AttemptReport::Completed, None);
        let waiting = initiative(Some(&finding()), Some(&recent));

        assert!(wait_for(&waiting, at(20)).is_some());
        assert_eq!(wait_for(&waiting, at(10_000)), Some(Duration::ZERO));
        assert_eq!(wait_for(&Initiative::Act, at(0)), None);
    }
}
