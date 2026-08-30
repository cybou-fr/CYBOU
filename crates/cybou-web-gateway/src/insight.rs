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
use cybou_protocol::telemetry::{
    ALL_SUBJECTS, EvidenceStrength, MetricKey, SystemInsight, WatchedResource, Watching,
};
use cybou_remediation::{StandingPolicy, authorize, criticise, propose};
use cybou_telemetryd::trend::{Projection, Reaching, Trend};
use cybou_web_contracts::{
    FindingProjection, InsightProjection, OfferProjection, ProjectionProjection, ReadingProjection,
    WEB_SCHEMA_V1, WatchedProjection,
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
    watched: &[WatchedResource],
    watched_enough: bool,
    projections: &[(MetricKey, Projection)],
    now: OffsetDateTime,
) -> InsightProjection {
    let plan = cybou_meaning::plan_system_state(
        insights,
        watched,
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
            .filter(|subject| {
                // Not observed, rather than not watched. A declared thing that could not be read
                // must not count towards "I looked at this".
                !watched.iter().any(|resource| {
                    resource.key.subject == **subject && resource.state.is_observed()
                })
            })
            .map(|subject| subject.name().to_owned())
            .collect(),
        watched: watched.iter().map(watched_state).collect(),
        projections: projections
            .iter()
            .map(|(key, projection)| heading(key, projection))
            .collect(),
        said: cybou_meaning::realize(&plan, cybou_meaning::Language::English),
    }
}

/// One watched thing, as a reader receives it.
fn watched_state(resource: &WatchedResource) -> WatchedProjection {
    let (at, value) = match &resource.state {
        Watching::Observed { value, at } => (Some(*at), Some(*value)),
        Watching::NeverRead => (None, None),
        Watching::ReadFailed { since } => (Some(*since), None),
        Watching::Stale { last_read } => (Some(*last_read), None),
    };
    WatchedProjection {
        subject: resource.key.label(),
        state: resource.state.name().to_owned(),
        at: at.and_then(|instant| instant.format(&Rfc3339).ok()),
        value,
    }
}

/// One subject's direction, as a reader receives it.
fn heading(key: &MetricKey, projection: &Projection) -> ProjectionProjection {
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
        // Also no number. A window nobody is feeding has no rate, and the last one it had is not
        // the current one however recently it was true.
        Reaching::ReadingsStopped { .. } => ("readings-stopped", None, false),
        Reaching::NotEnoughHistory { .. } => ("not-enough-history", None, false),
    };
    ProjectionProjection {
        // The thing it is about, not just the kind. A page of rows all called
        // `certificate.days.remaining` is a page nobody can act on.
        subject: key.label(),
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
        watched: Vec::new(),
        projections: Vec::new(),
        said: String::new(),
    }
}

/// One finding, with its readings and what could be offered about it.
fn finding(insight: &SystemInsight, now: OffsetDateTime) -> FindingProjection {
    FindingProjection {
        id: Some(insight.insight_id),
        finding: insight.finding.name().to_owned(),
        // The thing itself, not the kind of thing. Without it two findings about two certificates
        // are two identical rows.
        about: insight.about.as_ref().and_then(|key| key.instance.clone()),
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
            .map(|evidence| ReadingProjection {
                subject: evidence.key.label(),
                observed: evidence.observed,
                // Both absent together, or both present. A baseline is one measurement of this
                // host and splitting it would let half of it reach a reader alone.
                ordinary: evidence.deviation.map(|deviation| deviation.ordinary),
                spread: evidence.deviation.map(|deviation| deviation.spread),
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
    let everything: Vec<WatchedResource> = ALL_SUBJECTS
        .iter()
        .copied()
        .map(|subject| WatchedResource {
            key: MetricKey::host(subject),
            state: Watching::Observed {
                value: 0.0,
                at: OffsetDateTime::UNIX_EPOCH,
            },
        })
        .collect();
    let plan = cybou_meaning::plan_system_state(
        std::slice::from_ref(insight),
        &everything,
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
    // A projection is not a lifecycle owner and must not mint identities. `nil` is an explicit
    // preview sentinel; the proposal and decision that can produce a permit are created and
    // retained by Action1 with real identities.
    propose(insight, now, |_| Uuid::nil())
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
                AuthorizationVerdict::GrantedOnConfirmation { confirmed_by } => {
                    ("granted-on-confirmation".to_owned(), confirmed_by)
                }
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
    use cybou_protocol::telemetry::{Deviation, Finding, InsightEvidence, Subject};

    use super::*;

    fn at() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000).expect("a fixed instant")
    }

    /// Every universal subject, read.
    fn everything() -> Vec<WatchedResource> {
        ALL_SUBJECTS
            .iter()
            .copied()
            .map(|subject| observed_now(MetricKey::host(subject)))
            .collect()
    }

    /// One watched thing with a reading, for a fixture that only cares that it was read.
    fn observed_now(key: MetricKey) -> WatchedResource {
        WatchedResource {
            key,
            state: Watching::Observed {
                value: 0.0,
                at: OffsetDateTime::UNIX_EPOCH,
            },
        }
    }

    fn insight(finding: Finding, strength: EvidenceStrength) -> SystemInsight {
        SystemInsight {
            insight_id: Uuid::from_u128(1),
            finding,
            about: None,
            because: vec![InsightEvidence {
                key: MetricKey::host(Subject::RootFilesystemUsed),
                observed: 0.96,
                deviation: Some(Deviation {
                    ordinary: 0.62,
                    spread: 0.01,
                    observed: 0.96,
                    spreads_away: 22.9,
                }),
            }],
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
        let calm = project(&[], &everything(), true, &[], at());

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
            &everything(),
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
            &everything(),
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
            &everything(),
            true,
            &[],
            at(),
        );
        let reading = &projected.findings[0].readings[0];
        assert_eq!(reading.subject, "filesystem.root.used");
        assert!((reading.observed - 0.96).abs() < f64::EPSILON);
        assert_eq!(reading.ordinary, Some(0.62));
    }

    #[test]
    fn two_findings_about_two_certificates_reach_the_wire_as_two_things() {
        // Everything under this was rewritten so a measurement keeps track of which one it is
        // about. It used to stop one inch short of the reader: the instance reached this function
        // and was dropped before the wire.
        let expiring = |name: &str| SystemInsight {
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
            concluded_at: at(),
            since: at(),
        };

        let projected = project(
            &[expiring("/etc/ssl/a.pem"), expiring("/etc/ssl/b.pem")],
            &everything(),
            true,
            &[],
            at(),
        );

        let about: Vec<Option<String>> = projected
            .findings
            .iter()
            .map(|finding| finding.about.clone())
            .collect();
        assert_eq!(
            about,
            vec![
                Some("/etc/ssl/a.pem".to_owned()),
                Some("/etc/ssl/b.pem".to_owned())
            ],
            "the reader cannot tell which certificate either row is about"
        );
        // And each row's readings are its own, not the other one's.
        for (finding, name) in projected
            .findings
            .iter()
            .zip(["/etc/ssl/a.pem", "/etc/ssl/b.pem"])
        {
            assert!(
                finding
                    .readings
                    .iter()
                    .all(|reading| reading.subject.contains(name)),
                "a row about {name} carries {:?}",
                finding.readings
            );
        }
    }

    #[test]
    fn a_finding_about_the_host_carries_no_name_on_the_wire() {
        let projected = project(
            &[insight(
                Finding::StorageExhaustion,
                EvidenceStrength::Strong,
            )],
            &everything(),
            true,
            &[],
            at(),
        );
        assert_eq!(projected.findings[0].about, None);
    }

    #[test]
    fn what_was_never_looked_at_is_on_the_wire_too() {
        // So an all-clear can be read against what was actually watched rather than as a statement
        // about everything.
        let partial: Vec<WatchedResource> = ALL_SUBJECTS
            .iter()
            .filter(|subject| **subject != Subject::MemoryPressure)
            .copied()
            .map(|subject| observed_now(MetricKey::host(subject)))
            .collect();
        let projected = project(&[], &partial, true, &[], at());
        assert_eq!(projected.unobserved, vec!["memory.pressure".to_owned()]);
        assert!(projected.said.contains("no readings for"));
    }

    #[test]
    fn a_host_that_has_not_watched_long_enough_says_so_on_the_wire() {
        let projected = project(&[], &everything(), false, &[], at());
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
            &everything(),
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
        let first = project(&insights, &everything(), true, &[], at());
        for _ in 0..8 {
            assert_eq!(project(&insights, &everything(), true, &[], at()), first);
        }
    }
}
