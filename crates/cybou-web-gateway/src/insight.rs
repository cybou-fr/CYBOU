// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Assembling what the host makes of itself into something a browser can draw.
//!
//! Three layers meet here and none of them knows about the others: telemetry concluded, meaning
//! worded it, and the gate decided what could be done. Putting the join in the gateway rather than
//! in any of them is what keeps that true — an organ that assembled the whole answer would have to
//! read the two below it.
//!
//! Both the structure and the prose are carried. The structure is what the desktop draws; the prose
//! is what the host would say if asked. Sending only the structure would put the wording in the
//! browser, where it is one refactor from a fluent sentence nobody planned; sending only the prose
//! would make the readings undrawable. Having both means they can be compared, which is the point
//! of a deterministic path.
//!
//! **Nothing here can carry anything out.** The verdicts are shown so a person can see what the gate
//! would say. There is no executor behind any of them, and this module has no way to reach one.

use cybou_protocol::action::{AuthorizationVerdict, RiskLevel};
use cybou_protocol::telemetry::{ALL_SUBJECTS, EvidenceStrength, Subject, SystemInsight};
use cybou_remediation::{StandingPolicy, authorize, criticise, propose};
use cybou_telemetryd::trend::{Projection, Reaching, Trend};
use cybou_web_contracts::{
    FindingProjection, InsightProjection, OfferProjection, ProjectionProjection, ReadingProjection,
    WEB_SCHEMA_V1,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

/// Build the projection a reader receives from what the host holds.
///
/// `observed` is the subjects that actually have readings, which is not the same as the subjects
/// that exist — the difference is what lets an all-clear be read against what was looked at.
#[must_use]
pub fn project(
    insights: &[SystemInsight],
    observed: &[Subject],
    watched_enough: bool,
    projections: &[(Subject, Projection)],
    now: OffsetDateTime,
) -> InsightProjection {
    let plan = cybou_meaning::plan_system_state(
        insights,
        observed,
        watched_enough,
        // Derived from the instant rather than random, so the same host state projects identically
        // and a reader comparing two responses is comparing the answers and not the identifiers.
        Uuid::from_u128(u128::from(now.unix_timestamp().unsigned_abs())),
    );

    InsightProjection {
        schema_version: WEB_SCHEMA_V1,
        knowledge: cybou_protocol::KnowledgeState::Known,
        watched_enough,
        findings: insights
            .iter()
            .map(|insight| finding(insight, now))
            .collect(),
        unobserved: ALL_SUBJECTS
            .iter()
            .filter(|subject| !observed.contains(subject))
            .map(|subject| subject.name().to_owned())
            .collect(),
        projections: projections
            .iter()
            .map(|(subject, projection)| heading(*subject, projection))
            .collect(),
        said: cybou_meaning::realize(&plan, cybou_meaning::Language::English),
    }
}

/// One subject's direction, as a reader receives it.
fn heading(subject: Subject, projection: &Projection) -> ProjectionProjection {
    let (reaching, after_seconds, beyond) = match projection.reaching {
        Reaching::Already => ("already", None, false),
        Reaching::AtThisRate {
            after,
            beyond_what_was_watched,
        } => (
            "at-this-rate",
            Some(after.whole_seconds()),
            beyond_what_was_watched,
        ),
        // No number, deliberately. Zero would be a time, and *not at this rate* is not a time.
        Reaching::NotAtThisRate => ("not-at-this-rate", None, false),
        Reaching::NotEnoughHistory { .. } => ("not-enough-history", None, false),
    };
    ProjectionProjection {
        subject: subject.name().to_owned(),
        trend: match projection.trend {
            Trend::Rising(_) => "rising",
            Trend::Falling(_) => "falling",
            Trend::Flat => "flat",
        }
        .to_owned(),
        current: projection.current,
        threshold: projection.threshold,
        reaching: reaching.to_owned(),
        after_seconds,
        beyond_what_was_watched: beyond,
        watched_seconds: projection.watched.whole_seconds(),
    }
}

/// The projection for a host that could not be read at all.
///
/// Distinct from a host with nothing to report. One says the organ did not answer; the other says
/// it answered and found nothing, and a surface that showed the same thing for both would tell a
/// person their machine is fine when nobody looked.
#[must_use]
pub fn unread() -> InsightProjection {
    InsightProjection {
        schema_version: WEB_SCHEMA_V1,
        knowledge: cybou_protocol::KnowledgeState::Unknown,
        watched_enough: false,
        findings: Vec::new(),
        unobserved: Vec::new(),
        projections: Vec::new(),
        said: String::new(),
    }
}

/// One finding, with its readings and what could be offered about it.
fn finding(insight: &SystemInsight, now: OffsetDateTime) -> FindingProjection {
    FindingProjection {
        finding: insight.finding.name().to_owned(),
        means: means(insight),
        strength: match insight.strength {
            EvidenceStrength::Weak => "weak",
            EvidenceStrength::Moderate => "moderate",
            EvidenceStrength::Strong => "strong",
        }
        .to_owned(),
        since: insight
            .since
            .format(&Rfc3339)
            .unwrap_or_else(|_| String::new()),
        readings: insight
            .because
            .iter()
            .map(|(subject, deviation)| ReadingProjection {
                subject: subject.name().to_owned(),
                observed: deviation.observed,
                ordinary: deviation.ordinary,
                spread: deviation.spread,
            })
            .collect(),
        offers: offers(insight, now),
    }
}

/// What the host would say this finding means.
///
/// Taken from the same planner that writes the prose, so the phrase on a card and the phrase in the
/// sentence cannot drift apart into two descriptions of one thing.
fn means(insight: &SystemInsight) -> String {
    let plan = cybou_meaning::plan_system_state(
        std::slice::from_ref(insight),
        ALL_SUBJECTS,
        true,
        Uuid::nil(),
    );
    plan.key_points
        .iter()
        .find(|point| point.starts_with(insight.finding.name()))
        .and_then(|point| point.split_once(": "))
        .map(|(_, rest)| rest.split(". ").next().unwrap_or(rest).to_owned())
        .unwrap_or_default()
}

/// What could be offered about a finding, and what the gate decided.
fn offers(insight: &SystemInsight, now: OffsetDateTime) -> Vec<OfferProjection> {
    propose(insight, now, |operation| {
        Uuid::from_u128(operation.verb().len() as u128)
    })
    .into_iter()
    .map(|proposal| {
        let checks = criticise(&proposal, insight);
        let decision = authorize(
            &proposal,
            &checks,
            insight.strength == EvidenceStrength::Weak,
            // The gate is asked with an unconfigured policy, deliberately. This surface shows what
            // would be decided on a machine nobody has granted anything on, which is every machine
            // today, and a projection that read a policy would show a different answer per reader.
            &StandingPolicy::nothing_pre_authorized(),
            now,
        );
        let (verdict, reason) = match decision.verdict {
            AuthorizationVerdict::Granted => ("granted".to_owned(), String::new()),
            AuthorizationVerdict::RequiresUserConfirmation { prompt } => {
                ("requires-confirmation".to_owned(), prompt)
            }
            AuthorizationVerdict::Denied { reason } => ("denied".to_owned(), reason),
        };
        OfferProjection {
            operation: proposal.operation,
            target: proposal.target_resource,
            risk: match proposal.risk_level {
                RiskLevel::Low => "low",
                RiskLevel::Medium => "medium",
                RiskLevel::High => "high",
                RiskLevel::Critical => "critical",
            }
            .to_owned(),
            reversible: proposal.reversible,
            verdict,
            reason,
        }
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use cybou_protocol::telemetry::{Deviation, Finding};

    use super::*;

    fn at() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000).expect("a fixed instant")
    }

    fn insight(finding: Finding, strength: EvidenceStrength) -> SystemInsight {
        SystemInsight {
            insight_id: Uuid::from_u128(1),
            finding,
            because: vec![(
                Subject::RootFilesystemUsed,
                Deviation {
                    ordinary: 0.62,
                    spread: 0.01,
                    observed: 0.96,
                    spreads_away: 22.9,
                },
            )],
            strength,
            concluded_at: at(),
            since: at(),
        }
    }

    #[test]
    fn a_host_that_could_not_be_read_is_not_a_host_with_nothing_to_report() {
        // A surface showing the same thing for both would tell a person their machine is fine when
        // nobody looked.
        let silent = unread();
        let calm = project(&[], ALL_SUBJECTS, true, &[], at());

        assert_eq!(silent.knowledge, cybou_protocol::KnowledgeState::Unknown);
        assert_eq!(calm.knowledge, cybou_protocol::KnowledgeState::Known);
        assert!(silent.said.is_empty());
        assert!(calm.said.contains("Nothing needs attention"));
    }

    #[test]
    fn every_offer_carries_what_the_gate_decided_and_none_of_them_is_granted() {
        // The reason the verdict is on the wire at all: a person can see what would happen before
        // anything can make it happen, and on a machine nobody configured the answer is always that
        // it would be asked.
        let projected = project(
            &[insight(
                Finding::StorageExhaustion,
                EvidenceStrength::Strong,
            )],
            ALL_SUBJECTS,
            true,
            &[],
            at(),
        );
        let offers = &projected.findings[0].offers;
        assert!(!offers.is_empty());
        for offer in offers {
            assert_ne!(offer.verdict, "granted", "{offer:?}");
            assert_eq!(offer.verdict, "requires-confirmation");
            assert!(!offer.reason.is_empty(), "a confirmation with no prompt");
        }
    }

    #[test]
    fn a_guess_offers_to_look_and_is_refused_everything_else() {
        let projected = project(
            &[insight(Finding::ServiceFailure, EvidenceStrength::Weak)],
            ALL_SUBJECTS,
            true,
            &[],
            at(),
        );
        let offers = &projected.findings[0].offers;
        let denied = offers
            .iter()
            .filter(|offer| offer.verdict == "denied")
            .count();
        assert!(denied > 0, "{offers:?}");
        assert!(
            offers
                .iter()
                .any(|offer| offer.operation == "service.status"
                    && offer.verdict == "requires-confirmation"),
            "looking was refused: {offers:?}"
        );
    }

    #[test]
    fn the_readings_reach_the_wire() {
        let projected = project(
            &[insight(
                Finding::StorageExhaustion,
                EvidenceStrength::Strong,
            )],
            ALL_SUBJECTS,
            true,
            &[],
            at(),
        );
        let reading = &projected.findings[0].readings[0];
        assert_eq!(reading.subject, "filesystem.root.used");
        assert!((reading.observed - 0.96).abs() < f64::EPSILON);
        assert!((reading.ordinary - 0.62).abs() < f64::EPSILON);
    }

    #[test]
    fn what_was_never_looked_at_is_on_the_wire_too() {
        // So an all-clear can be read against what was actually watched rather than as a statement
        // about everything.
        let partial: Vec<Subject> = ALL_SUBJECTS
            .iter()
            .filter(|subject| **subject != Subject::MemoryPressure)
            .copied()
            .collect();
        let projected = project(&[], &partial, true, &[], at());
        assert_eq!(projected.unobserved, vec!["memory.pressure".to_owned()]);
        assert!(projected.said.contains("no readings for"));
    }

    #[test]
    fn a_host_that_has_not_watched_long_enough_says_so_on_the_wire() {
        let projected = project(&[], ALL_SUBJECTS, false, &[], at());
        assert!(!projected.watched_enough);
        assert!(projected.said.contains("not been watching"));
    }

    #[test]
    fn the_phrase_on_a_card_is_the_phrase_in_the_sentence() {
        // Taken from the same planner, so the card and the prose cannot drift apart into two
        // descriptions of one thing.
        let projected = project(
            &[insight(
                Finding::StorageExhaustion,
                EvidenceStrength::Strong,
            )],
            ALL_SUBJECTS,
            true,
            &[],
            at(),
        );
        let means = &projected.findings[0].means;
        assert!(!means.is_empty());
        assert!(
            projected.said.contains(means.as_str()),
            "{means} / {}",
            projected.said
        );
    }

    #[test]
    fn the_same_host_state_projects_identically() {
        let insights = [insight(
            Finding::StorageExhaustion,
            EvidenceStrength::Strong,
        )];
        let first = project(&insights, ALL_SUBJECTS, true, &[], at());
        for _ in 0..8 {
            assert_eq!(project(&insights, ALL_SUBJECTS, true, &[], at()), first);
        }
    }
}
