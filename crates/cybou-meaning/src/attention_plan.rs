// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Turning what reached attention into a plan, before anything turns a plan into words.
//!
//! This is the last joint in the path a word travels: an utterance becomes an act, the act seeds an
//! activation, the activation proposes, attention admits, and what was admitted has to become
//! something sayable. Every joint before this one refuses to lose something — a budget that cut the
//! walk, a quota that turned proposals away, a standing the epistemic owner set. This is where all
//! of those either reach the sentence or quietly do not.
//!
//! They are converted here rather than in the renderer for the reason ADR-0031 gives for having a
//! plan at all: *the hedges are decided in the typed layer.* "Four things came to mind" and "four
//! things came to mind, of two thousand, and one of them is contested" are the same function of the
//! same admission, chosen by tone, if the choice is left to whoever writes the sentence.
//!
//! Like the rest of this crate, the function reaches nothing. It is a function of the admission it
//! is handed, so a plan cannot acquire a claim from a graph, a clock, or a workspace.

use cybou_protocol::attention::Admission;
use cybou_protocol::epistemic::EpistemicStatus;
use cybou_protocol::meaning::{Qualification, ResponsePlan};
use uuid::Uuid;

/// The intent an answer about what came to mind carries.
pub const INTENT_ASSOCIATION: &str = "inform_association";

/// Build a plan describing what a word brought to mind and what became of it.
///
/// The identity is supplied rather than generated, for the same reason as elsewhere in this crate:
/// a planner that reached for a random source would be reaching for something.
#[must_use]
pub fn plan_attention(seed: &str, admission: &Admission, plan_id: Uuid) -> ResponsePlan {
    let mut key_points = Vec::new();
    let mut qualifications = Vec::new();

    if admission.considered == 0 {
        if admission.upstream_complete {
            // Nothing was proposed, by a retrieval that finished. This is the one case where the
            // graph genuinely holds nothing, and it is the only case where saying so is true.
            key_points.push(format!("Nothing is associated with {seed}."));
        } else {
            // Nothing was proposed by a retrieval that never finished, which is not the same fact
            // at all — and the difference is the substrate's oldest rule: partial is not empty
            // truth. A `Partial` hedge beside "nothing is associated" does not repair it, because a
            // hedge qualifies a claim and does not withdraw one. The claim itself has to change.
            key_points.push(format!(
                "The search for what is associated with {seed} did not finish, so nothing came back."
            ));
        }
    } else if admission.admitted.is_empty() {
        key_points.push(format!(
            "{} things came to mind for {seed}, and none could be attended to.",
            admission.considered
        ));
    } else {
        key_points.push(format!(
            "{} of {} things that came to mind for {seed} are being attended to.",
            admission.admitted.len(),
            admission.considered
        ));
        for item in &admission.admitted {
            // The reason travels all the way from the graph. A key point that said only the label
            // would make the answer unarguable at exactly the point A12 exists to keep arguable.
            key_points.push(format!("{}: {}", item.label, item.reason));
        }
    }

    // Either kind of shortfall qualifies the answer, and one sentence covers both because a reader
    // acts on the same thing: this is not all of it. Which one it was stays on the admission for
    // anyone who needs to know whether attention was busy or the retrieval never finished.
    if !admission.complete || !admission.upstream_complete {
        qualifications.push(Qualification::Partial);
    }
    for item in &admission.admitted {
        // Each concept's own standing, in the order the concepts are in, so two runs compare.
        if let Some(qualification) = standing_qualification(item.epistemic_status)
            && !qualifications.contains(&qualification)
        {
            qualifications.push(qualification);
        }
    }

    ResponsePlan {
        plan_id,
        intent: INTENT_ASSOCIATION.to_owned(),
        key_points,
        // The plan cites the concepts by name in its points rather than by contribution id: what
        // the walk rests on is the graph, and the graph's own evidence belongs to the activation
        // that produced it. Claiming contributions here would be citing something not read.
        referenced_evidence: Vec::new(),
        qualifications,
    }
}

/// What a standing obliges the answer to say, if anything.
///
/// `Observed` alone obliges nothing. Every other standing maps to a distinct qualification rather
/// than to a shared "uncertain": a reader told something weaker than "contested" would not know to
/// go and look.
const fn standing_qualification(status: EpistemicStatus) -> Option<Qualification> {
    match status {
        EpistemicStatus::Observed => None,
        EpistemicStatus::Stale => Some(Qualification::Stale),
        EpistemicStatus::Disputed => Some(Qualification::Disputed),
        EpistemicStatus::Superseded => Some(Qualification::Superseded),
        EpistemicStatus::Unknown => Some(Qualification::NotRead),
    }
}

#[cfg(test)]
mod tests {
    use cybou_protocol::attention::AttentionProposal;

    use super::*;

    fn id() -> Uuid {
        Uuid::from_u128(5)
    }

    fn proposal(label: &str, status: EpistemicStatus) -> AttentionProposal {
        AttentionProposal {
            label: label.to_owned(),
            relevance: 0.8,
            reason: format!("lemon → {label} (Episodic, strength 0.80) at depth 1"),
            epistemic_status: status,
        }
    }

    fn admission(admitted: Vec<AttentionProposal>, considered: usize, complete: bool) -> Admission {
        Admission {
            refused_for_quota: considered - admitted.len(),
            considered,
            refused_unreached: 0,
            refused_duplicate: 0,
            complete,
            upstream_complete: true,
            admitted,
        }
    }

    #[test]
    fn a_contested_concept_reaching_attention_qualifies_the_answer() {
        // The end of the chain A4 runs along. Everything before this carried the standing; this is
        // where carrying it either becomes a sentence or stops mattering.
        let plan = plan_attention(
            "lemon",
            &admission(vec![proposal("honey", EpistemicStatus::Disputed)], 1, true),
            id(),
        );
        assert!(plan.qualifications.contains(&Qualification::Disputed));
    }

    #[test]
    fn a_quota_that_turned_things_away_qualifies_the_answer() {
        let plan = plan_attention(
            "lemon",
            &admission(
                vec![proposal("honey", EpistemicStatus::Observed)],
                2000,
                false,
            ),
            id(),
        );
        assert!(plan.qualifications.contains(&Qualification::Partial));
        assert!(
            plan.key_points[0].contains("2000"),
            "the size of what was turned away is not in the answer: {}",
            plan.key_points[0]
        );
    }

    #[test]
    fn a_retrieval_that_never_finished_qualifies_even_when_nothing_was_refused() {
        // The defect the walkthrough found, held at the layer that turns it into a sentence. The
        // admission refused nothing and is itself complete; the answer is still not the whole of it.
        let mut short = admission(vec![proposal("honey", EpistemicStatus::Observed)], 1, true);
        short.upstream_complete = false;
        let plan = plan_attention("lemon", &short, id());
        assert!(plan.qualifications.contains(&Qualification::Partial));
    }

    #[test]
    fn everything_that_fit_and_is_settled_carries_no_hedge() {
        let plan = plan_attention(
            "lemon",
            &admission(vec![proposal("honey", EpistemicStatus::Observed)], 1, true),
            id(),
        );
        assert!(plan.qualifications.is_empty(), "{:?}", plan.qualifications);
    }

    #[test]
    fn nothing_associated_and_nothing_admitted_are_different_answers() {
        // One says the graph had nothing; the other says attention had no room, and a person can
        // act on the second.
        let empty_graph = plan_attention("bergamot", &admission(Vec::new(), 0, true), id());
        let no_room = plan_attention("lemon", &admission(Vec::new(), 40, false), id());
        assert_ne!(empty_graph.key_points, no_room.key_points);
        assert!(empty_graph.key_points[0].contains("Nothing is associated"));
        assert!(no_room.key_points[0].contains("none could be attended to"));
        assert!(no_room.qualifications.contains(&Qualification::Partial));
    }

    #[test]
    fn a_search_that_did_not_finish_never_reports_that_nothing_is_associated() {
        // Found by a flaky end-to-end run, which is the only place it could have been found: a
        // wall clock cut a two-thousand-edge walk before its first step, the activation honestly
        // returned nothing, and the plan turned an unfinished search into the claim that the graph
        // was empty. Partial is not empty truth, and a hedge qualifies a claim without withdrawing
        // one — so the claim itself changes.
        let mut unfinished = admission(Vec::new(), 0, true);
        unfinished.upstream_complete = false;
        let plan = plan_attention("lemon", &unfinished, id());

        assert!(
            !plan.key_points[0].contains("Nothing is associated"),
            "an unfinished search reported an empty graph: {}",
            plan.key_points[0]
        );
        assert!(plan.key_points[0].contains("did not finish"));
        assert!(plan.qualifications.contains(&Qualification::Partial));
    }

    #[test]
    fn a_search_that_did_finish_and_found_nothing_says_so_plainly() {
        // The control. Without it the fix above could be a renderer that never commits to anything.
        let plan = plan_attention("bergamot", &admission(Vec::new(), 0, true), id());
        assert!(plan.key_points[0].contains("Nothing is associated"));
        assert!(plan.qualifications.is_empty());
    }

    #[test]
    fn each_standing_becomes_its_own_qualification_rather_than_a_shared_uncertainty() {
        // A reader told something weaker than "contested" would not know to go and look.
        for (status, expected) in [
            (EpistemicStatus::Stale, Qualification::Stale),
            (EpistemicStatus::Disputed, Qualification::Disputed),
            (EpistemicStatus::Superseded, Qualification::Superseded),
            (EpistemicStatus::Unknown, Qualification::NotRead),
        ] {
            let plan = plan_attention(
                "lemon",
                &admission(vec![proposal("honey", status)], 1, true),
                id(),
            );
            assert_eq!(plan.qualifications, vec![expected], "{status:?}");
        }
    }

    #[test]
    fn why_each_concept_came_to_mind_survives_into_the_plan() {
        // A12 at the far end. A key point naming only the label would make the answer unarguable
        // at exactly the point the path exists to keep arguable.
        let plan = plan_attention(
            "lemon",
            &admission(vec![proposal("honey", EpistemicStatus::Observed)], 1, true),
            id(),
        );
        assert!(
            plan.key_points
                .iter()
                .any(|point| point.contains("lemon → honey")),
            "{:?}",
            plan.key_points
        );
    }

    #[test]
    fn one_hedge_is_stated_once_however_many_concepts_raise_it() {
        let plan = plan_attention(
            "lemon",
            &admission(
                vec![
                    proposal("honey", EpistemicStatus::Disputed),
                    proposal("tea", EpistemicStatus::Disputed),
                ],
                2,
                true,
            ),
            id(),
        );
        assert_eq!(plan.qualifications, vec![Qualification::Disputed]);
    }

    #[test]
    fn the_same_admission_always_plans_the_same_way() {
        let given = admission(
            vec![
                proposal("honey", EpistemicStatus::Disputed),
                proposal("tea", EpistemicStatus::Stale),
            ],
            9,
            false,
        );
        let first = plan_attention("lemon", &given, id());
        for _ in 0..8 {
            assert_eq!(plan_attention("lemon", &given, id()), first);
        }
    }
}
