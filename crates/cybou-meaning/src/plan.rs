// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Turning what Mind holds into a plan, before anything turns a plan into words.
//!
//! ADR-0031 puts a `ResponsePlan` between typed state and prose, and until now nothing built one.
//! `Realize` was reachable and unused, and C5 — *a plan expresses claims, evidence references and
//! qualifications before language realization* — had no assertion behind it, because there was no
//! plan for an assertion to be about.
//!
//! What matters here is not the wording. It is that **the hedges are decided in the typed layer**.
//! A capability nobody could read, a projection outside its freshness, a listing cut short by a
//! bound: each becomes a [`Qualification`] on the plan, where the realizer cannot lose it. If the
//! hedging were left to whoever writes the sentence, "eleven of eleven capabilities are available"
//! and "eleven of eleven, as far as I could read" would be the same function of the same state,
//! chosen by tone.
//!
//! This planner reaches nothing. It is a function of the facts it is handed, so a plan cannot
//! acquire a claim from a Journal, a clock, or an environment variable — the same discipline the
//! realizer has, one layer earlier.

use cybou_protocol::meaning::{Qualification, ResponsePlan};
use cybou_protocol::{CapabilityState, KnowledgeState};
use uuid::Uuid;

/// One capability, as its owner reported it.
#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityFact {
    /// The capability's own name.
    pub name: String,
    /// What its owner said about it.
    pub state: CapabilityState,
}

/// What is known about the system's health at one instant.
///
/// Every field is something an owner established or explicitly did not. There is no field for "how
/// it feels" and no default that stands in for a missing reading: `knowledge` is what says whether
/// there was a reading at all.
#[derive(Clone, Debug, PartialEq)]
// Four flags, and each answers a question the others cannot: whether the reading is current,
// whether the record behind it was verified, whether anything was kept from this reader, and
// whether the list was cut short. Folding any pair into one value would report two different
// standings as the same one, which is the distinction this whole type exists to carry.
#[allow(clippy::struct_excessive_bools)]
pub struct StatusFacts {
    /// Whether the projection these facts came from was produced at all.
    pub knowledge: KnowledgeState,
    /// The capabilities, as reported.
    pub capabilities: Vec<CapabilityFact>,
    /// Whether the projection is inside the freshness its owner declared.
    pub fresh: bool,
    /// Whether the Journal behind it was verified through its head.
    pub verified: bool,
    /// Whether anything was held back from the reader this answer is for.
    pub withheld_anything: bool,
    /// Whether the capability list was cut short by a bound.
    pub truncated: bool,
    /// The contributions this answer rests on.
    pub evidence: Vec<Uuid>,
}

/// The intent a status answer carries.
pub const INTENT_STATUS: &str = "inform_status";

/// Build a plan describing the system's health.
///
/// The identity is supplied rather than generated, because a planner that reached for a random
/// source would be reaching for something, and the point of this layer is that it reaches for
/// nothing.
#[must_use]
pub fn plan_status(facts: &StatusFacts, plan_id: Uuid) -> ResponsePlan {
    let mut key_points = Vec::new();
    let mut qualifications = Vec::new();

    if facts.knowledge == KnowledgeState::Unknown {
        // Nothing was read, so there is nothing to summarise and no number to give. A planner that
        // answered "0 of 0 available" here would be reporting an empty reading as a healthy one.
        key_points.push("The capability projection has not been read.".to_owned());
        qualifications.push(Qualification::NotRead);
        return finish(plan_id, key_points, qualifications, facts);
    }

    let total = facts.capabilities.len();
    let available = facts
        .capabilities
        .iter()
        .filter(|capability| capability.state == CapabilityState::Available)
        .count();
    key_points.push(format!(
        "{available} of {total} capabilities are available."
    ));

    // Named individually, because "not all of them" is not an answer a person can act on. Only the
    // ones that are not available: listing the others would be restating the count.
    for capability in &facts.capabilities {
        if capability.state != CapabilityState::Available {
            key_points.push(format!(
                "{} is not available; its owner reports {}.",
                capability.name,
                state_name(capability.state)
            ));
        }
    }

    // A capability whose own state could not be determined is why `unknown` is a qualification and
    // not a value: the count above cannot be read as complete while one of its terms is missing.
    if facts
        .capabilities
        .iter()
        .any(|capability| state_name(capability.state) == "unknown")
    {
        qualifications.push(Qualification::NotRead);
    }
    if !facts.fresh {
        qualifications.push(Qualification::Stale);
    }
    if !facts.verified {
        qualifications.push(Qualification::Unverified);
    }
    if facts.withheld_anything {
        qualifications.push(Qualification::Withheld);
    }
    if facts.truncated {
        qualifications.push(Qualification::Partial);
    }

    finish(plan_id, key_points, qualifications, facts)
}

/// Assemble the plan, keeping the qualification list free of repeats.
fn finish(
    plan_id: Uuid,
    key_points: Vec<String>,
    mut qualifications: Vec<Qualification>,
    facts: &StatusFacts,
) -> ResponsePlan {
    qualifications.dedup();
    ResponsePlan {
        plan_id,
        intent: INTENT_STATUS.to_owned(),
        key_points,
        referenced_evidence: facts.evidence.clone(),
        qualifications,
    }
}

/// A capability state in the owner's own spelling.
const fn state_name(state: CapabilityState) -> &'static str {
    match state {
        CapabilityState::Available => "available",
        CapabilityState::Unavailable => "unavailable",
        CapabilityState::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> Uuid {
        Uuid::from_u128(7)
    }

    fn healthy() -> StatusFacts {
        StatusFacts {
            knowledge: KnowledgeState::Known,
            capabilities: vec![
                CapabilityFact {
                    name: "identity-continuity".into(),
                    state: CapabilityState::Available,
                },
                CapabilityFact {
                    name: "accepted-biography".into(),
                    state: CapabilityState::Available,
                },
            ],
            fresh: true,
            verified: true,
            withheld_anything: false,
            truncated: false,
            evidence: vec![Uuid::from_u128(1)],
        }
    }

    #[test]
    fn a_reading_that_never_happened_is_not_a_healthy_reading() {
        // The failure this planner exists to prevent: no capabilities read, and a count of zero out
        // of zero rendered as though everything were fine.
        let facts = StatusFacts {
            knowledge: KnowledgeState::Unknown,
            capabilities: Vec::new(),
            ..healthy()
        };
        let plan = plan_status(&facts, id());

        assert!(plan.qualifications.contains(&Qualification::NotRead));
        assert!(
            !plan.key_points.iter().any(|point| point.contains("0 of 0")),
            "an unread projection was summarised as a count: {:?}",
            plan.key_points
        );
    }

    #[test]
    fn everything_available_carries_no_hedge() {
        let plan = plan_status(&healthy(), id());
        assert!(plan.qualifications.is_empty(), "{:?}", plan.qualifications);
        assert_eq!(plan.key_points[0], "2 of 2 capabilities are available.");
    }

    #[test]
    fn what_is_not_available_is_named_rather_than_counted() {
        // "One is not available" is not something a person can act on.
        let mut facts = healthy();
        facts.capabilities[1].state = CapabilityState::Unavailable;
        let plan = plan_status(&facts, id());

        assert!(
            plan.key_points
                .iter()
                .any(|point| point.contains("accepted-biography") && point.contains("unavailable")),
            "{:?}",
            plan.key_points
        );
    }

    #[test]
    fn a_capability_nobody_could_read_hedges_the_whole_count() {
        // The count cannot be read as complete while one of its terms is missing.
        let mut facts = healthy();
        facts.capabilities[0].state = CapabilityState::Unknown;
        let plan = plan_status(&facts, id());
        assert!(plan.qualifications.contains(&Qualification::NotRead));
    }

    #[test]
    fn every_standing_of_the_answer_becomes_a_qualification() {
        let facts = StatusFacts {
            fresh: false,
            verified: false,
            withheld_anything: true,
            truncated: true,
            ..healthy()
        };
        let plan = plan_status(&facts, id());

        for expected in [
            Qualification::Stale,
            Qualification::Unverified,
            Qualification::Withheld,
            Qualification::Partial,
        ] {
            assert!(
                plan.qualifications.contains(&expected),
                "{expected:?} was decided nowhere: {:?}",
                plan.qualifications
            );
        }
    }

    #[test]
    fn the_plan_carries_what_it_rests_on() {
        let plan = plan_status(&healthy(), id());
        assert_eq!(plan.referenced_evidence, vec![Uuid::from_u128(1)]);
        assert_eq!(plan.intent, INTENT_STATUS);
        assert_eq!(plan.plan_id, id());
    }

    #[test]
    fn the_same_facts_produce_the_same_plan() {
        // The planner is a function of what it was handed. Anything it reached for would make this
        // fail, which is why the test is here rather than a comment saying it reaches for nothing.
        assert_eq!(plan_status(&healthy(), id()), plan_status(&healthy(), id()));
    }
}
