// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Turning what a host concluded about itself into something it can say.
//!
//! This is the joint that makes ADR-0041's S0 gate reachable end to end. Telemetry observes, the
//! detector concludes, and until now the conclusion stopped at a struct. What a person asks is
//! *what is going on with this host*, and the answer has to come from the deterministic layer —
//! because the moment it most matters is the moment the network is the thing under investigation.
//!
//! Three things are decided here rather than in whatever writes the sentence, and each is a way the
//! answer is usually lost:
//!
//! **A finding is a hypothesis and the words have to say so.** *The cause is storage exhaustion*
//! and *this is consistent with storage exhaustion* are the same struct and different claims. The
//! second is what the evidence supports; the first is what a fluent renderer produces when nobody
//! decided.
//!
//! **An all-clear is qualified by what was not looked at.** A host whose kernel has no pressure
//! accounting can say *nothing needs attention* about the things it watched, and saying it plainly
//! would be reporting an absence of evidence as evidence of absence — on the one surface a person
//! consults to decide whether to go back to sleep.
//!
//! **Not having watched long enough is its own answer.** For the first minutes after a restart the
//! honest reply is that there is no notion yet of what is ordinary here. A confident all-clear built
//! on four readings is worse than silence, because it will be believed.

use cybou_protocol::meaning::{Qualification, ResponsePlan};
use cybou_protocol::telemetry::{ALL_SUBJECTS, EvidenceStrength, Finding, Subject, SystemInsight};
use uuid::Uuid;

/// The intent an answer about the host's own state carries.
pub const INTENT_SYSTEM_STATE: &str = "inform_system_state";

/// Build a plan describing what the host currently makes of itself.
///
/// `observed` is the subjects that actually have readings — not the subjects that exist. The
/// difference is the whole of the second rule above.
///
/// The identity is supplied rather than generated, for the same reason as everywhere else in this
/// crate: a planner that reached for a random source would be reaching for something.
#[must_use]
pub fn plan_system_state(
    insights: &[SystemInsight],
    observed: &[Subject],
    watched_enough: bool,
    plan_id: Uuid,
) -> ResponsePlan {
    let mut key_points = Vec::new();
    let mut qualifications = Vec::new();

    let unobserved: Vec<Subject> = ALL_SUBJECTS
        .iter()
        .filter(|subject| !observed.contains(subject))
        .copied()
        .collect();

    if !watched_enough {
        // Distinct from "nothing is wrong". One says there is nothing to report; this says there is
        // nobody yet who could tell, and a person reading the first when the second is true has
        // been told something false about their machine.
        key_points.push(
            "I have not been watching this host long enough to know what is ordinary for it."
                .to_owned(),
        );
        qualifications.push(Qualification::NotRead);
        return finish(plan_id, key_points, qualifications, &unobserved);
    }

    if insights.is_empty() {
        key_points.push(format!(
            "Nothing needs attention among the {} thing(s) I watch.",
            observed.len()
        ));
    } else {
        key_points.push(format!("{} thing(s) need attention.", insights.len()));
        let mut ordered: Vec<&SystemInsight> = insights.iter().collect();
        // Strongest evidence first, then by a fixed finding order, so the same state always reads
        // the same way and the most defensible claim is the one at the top.
        ordered.sort_by_key(|insight| (severity(insight.strength), insight.finding.name()));

        for insight in ordered {
            key_points.push(sentence(insight));
            for (subject, deviation) in &insight.because {
                // The readings, so *why do you think that* is answered by looking. An insight that
                // could not show them would be indistinguishable from one a model made up.
                key_points.push(format!(
                    "  {} is {:.2}, where {:.2} is ordinary here (spread {:.3}).",
                    subject.name(),
                    deviation.observed,
                    deviation.ordinary,
                    deviation.spread
                ));
            }
        }
    }

    finish(plan_id, key_points, qualifications, &unobserved)
}

/// One finding, worded as the hypothesis it is.
fn sentence(insight: &SystemInsight) -> String {
    let hedge = match insight.strength {
        // Never "the cause is". The strongest thing the evidence supports is that the observations
        // and the explanation agree, and a renderer that upgraded that to a cause would be making
        // the claim the plan boundary exists to prevent.
        EvidenceStrength::Strong => "This is what the readings show",
        EvidenceStrength::Moderate => "This is consistent with what I observed",
        EvidenceStrength::Weak => "This is one reading out of range, and nothing corroborates it",
    };
    format!(
        "{}: {}. {hedge}. Since {}.",
        insight.finding.name(),
        describe(insight.finding),
        insight.since
    )
}

/// What a finding means, in the words a person would use.
const fn describe(finding: Finding) -> &'static str {
    match finding {
        Finding::MemoryPressure => "the machine is spending time waiting for memory",
        Finding::StorageExhaustion => "storage is nearly full",
        Finding::IoSaturation => "the machine is spending time waiting for disk",
        Finding::CpuSaturation => "the machine is spending time waiting for CPU",
        Finding::ServiceFailure => "one or more services are in a failed state",
        Finding::UnexplainedDeviation => {
            "something is out of its ordinary range and I cannot say why"
        }
    }
}

/// Strongest first.
const fn severity(strength: EvidenceStrength) -> u8 {
    match strength {
        EvidenceStrength::Strong => 0,
        EvidenceStrength::Moderate => 1,
        EvidenceStrength::Weak => 2,
    }
}

/// Assemble the plan, saying what was not looked at.
fn finish(
    plan_id: Uuid,
    mut key_points: Vec<String>,
    mut qualifications: Vec<Qualification>,
    unobserved: &[Subject],
) -> ResponsePlan {
    if !unobserved.is_empty() {
        let names: Vec<&str> = unobserved.iter().map(|subject| subject.name()).collect();
        key_points.push(format!("I have no readings for: {}.", names.join(", ")));
        // The qualification, not just the sentence. A hedge that lives only in prose is a hedge a
        // caller composing this with another answer will drop, which is the failure `compose`
        // exists to prevent one layer over.
        if !qualifications.contains(&Qualification::NotRead) {
            qualifications.push(Qualification::NotRead);
        }
    }
    ResponsePlan {
        plan_id,
        intent: INTENT_SYSTEM_STATE.to_owned(),
        key_points,
        // The readings are in the points and are not contributions. A `Reading` never enters the
        // Journal, so there is nothing here to cite — claiming evidence ids would be citing
        // something that does not exist.
        referenced_evidence: Vec::new(),
        qualifications,
    }
}

#[cfg(test)]
mod tests {
    use cybou_protocol::telemetry::Deviation;
    use time::OffsetDateTime;

    use super::*;

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    fn id() -> Uuid {
        Uuid::from_u128(11)
    }

    fn everything() -> Vec<Subject> {
        ALL_SUBJECTS.to_vec()
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
            concluded_at: at(100),
            since: at(0),
        }
    }

    #[test]
    fn a_finding_is_worded_as_a_hypothesis_and_never_as_a_cause() {
        // "The cause is storage exhaustion" and "this is what the readings show" are the same
        // struct and different claims. The second is what the evidence supports.
        let plan = plan_system_state(
            &[insight(
                Finding::StorageExhaustion,
                EvidenceStrength::Strong,
            )],
            &everything(),
            true,
            id(),
        );
        let said = plan.key_points.join(" ");
        assert!(said.contains("readings show"), "{said}");
        assert!(!said.contains("the cause"), "{said}");
        assert!(!said.contains("caused by"), "{said}");
    }

    #[test]
    fn not_having_watched_long_enough_is_its_own_answer() {
        // A confident all-clear built on four readings is worse than silence, because it will be
        // believed.
        let plan = plan_system_state(&[], &everything(), false, id());
        assert!(plan.key_points[0].contains("not been watching"), "{plan:?}");
        assert!(
            !plan
                .key_points
                .iter()
                .any(|point| point.contains("Nothing needs attention")),
            "a host that has not watched reported an all-clear: {plan:?}"
        );
        assert!(plan.qualifications.contains(&Qualification::NotRead));
    }

    #[test]
    fn an_all_clear_is_qualified_by_what_was_never_looked_at() {
        // A host whose kernel has no pressure accounting can say nothing needs attention about what
        // it watched. Saying it plainly reports an absence of evidence as evidence of absence, on
        // the one surface a person consults to decide whether to go back to sleep.
        let partial: Vec<Subject> = everything()
            .into_iter()
            .filter(|subject| *subject != Subject::MemoryPressure)
            .collect();
        let plan = plan_system_state(&[], &partial, true, id());

        assert!(plan.qualifications.contains(&Qualification::NotRead));
        assert!(
            plan.key_points
                .iter()
                .any(|point| point.contains("memory.pressure")),
            "the unwatched subject is not named: {plan:?}"
        );
    }

    #[test]
    fn a_host_watching_everything_and_finding_nothing_says_so_without_hedging() {
        // The control. Every test above passes on a planner that hedges unconditionally, and a
        // hedge that is always there is the same as no hedge.
        let plan = plan_system_state(&[], &everything(), true, id());
        assert!(plan.qualifications.is_empty(), "{:?}", plan.qualifications);
        assert!(plan.key_points[0].contains("Nothing needs attention"));
    }

    #[test]
    fn the_most_defensible_finding_is_the_one_at_the_top() {
        let plan = plan_system_state(
            &[
                insight(Finding::UnexplainedDeviation, EvidenceStrength::Weak),
                insight(Finding::StorageExhaustion, EvidenceStrength::Strong),
                insight(Finding::MemoryPressure, EvidenceStrength::Moderate),
            ],
            &everything(),
            true,
            id(),
        );
        let storage = plan
            .key_points
            .iter()
            .position(|point| point.starts_with("storage-exhaustion"))
            .expect("present");
        let memory = plan
            .key_points
            .iter()
            .position(|point| point.starts_with("memory-pressure"))
            .expect("present");
        let unexplained = plan
            .key_points
            .iter()
            .position(|point| point.starts_with("unexplained-deviation"))
            .expect("present");
        assert!(storage < memory && memory < unexplained, "{plan:?}");
    }

    #[test]
    fn the_readings_behind_a_finding_reach_the_answer() {
        // Why do you think that, answered by looking. A plan that named only the finding would be
        // indistinguishable from one a model made up.
        let plan = plan_system_state(
            &[insight(
                Finding::StorageExhaustion,
                EvidenceStrength::Strong,
            )],
            &everything(),
            true,
            id(),
        );
        let said = plan.key_points.join(" ");
        assert!(said.contains("filesystem.root.used"), "{said}");
        assert!(said.contains("0.96"), "{said}");
        assert!(said.contains("0.62"), "{said}");
    }

    #[test]
    fn a_weak_finding_says_that_nothing_corroborates_it() {
        let plan = plan_system_state(
            &[insight(Finding::MemoryPressure, EvidenceStrength::Weak)],
            &everything(),
            true,
            id(),
        );
        assert!(
            plan.key_points
                .iter()
                .any(|point| point.contains("nothing corroborates")),
            "{plan:?}"
        );
    }

    #[test]
    fn the_plan_cites_no_contributions_because_a_reading_is_not_one() {
        // A `Reading` never enters the Journal, so there is nothing to cite. Claiming evidence ids
        // here would be citing something that does not exist.
        let plan = plan_system_state(
            &[insight(
                Finding::StorageExhaustion,
                EvidenceStrength::Strong,
            )],
            &everything(),
            true,
            id(),
        );
        assert!(plan.referenced_evidence.is_empty());
    }

    #[test]
    fn the_same_state_always_reads_the_same_way() {
        let insights = [
            insight(Finding::MemoryPressure, EvidenceStrength::Moderate),
            insight(Finding::StorageExhaustion, EvidenceStrength::Strong),
        ];
        let first = plan_system_state(&insights, &everything(), true, id());
        for _ in 0..8 {
            assert_eq!(
                plan_system_state(&insights, &everything(), true, id()),
                first
            );
        }
    }
}
