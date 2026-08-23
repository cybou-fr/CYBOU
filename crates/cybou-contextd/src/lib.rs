// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Associative and situational context management engine (ADR-0029: association != truth).
//!
//! Maintains active context vectors, associative graphs between entities and concepts,
//! tracking explicit provenance (`why?`, `origin`, `evidence`) and producing inspectable
//! bounded `ContextBundle` projections.

pub mod activation;
pub mod core;
pub mod types;

#[cfg(target_os = "linux")]
pub mod service;

pub use activation::{
    ActivatedConcept, ActivationBudget, ActivationSession, Exhausted, activate_from,
};
pub use core::{ContextCore, enforce_edge_budget, enforce_node_budget};
pub use types::{
    Association, AssociationOrigin, ConceptNode, ContextBudget, ContextBundle,
    most_restrictive_privacy, shortest_retention,
};

#[cfg(test)]
mod tests {
    use cybou_protocol::{admission::Privacy, epistemic::EpistemicStatus};
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn a_later_activation_that_knew_nothing_does_not_erase_a_dispute() {
        // A caller that did not know is not evidence that a dispute went away. Letting silence
        // overwrite `Disputed` would lose it at the one boundary A4 exists to hold — and the loss
        // would look exactly like there having been nothing to lose.
        let context_engine = ContextCore::new();
        let now = OffsetDateTime::now_utc();
        context_engine.activate_with_standing(
            "kernel-version",
            1.0,
            "observed",
            now,
            0,
            EpistemicStatus::Disputed,
        );
        context_engine.activate("kernel-version", 1.0, "observed again", now);

        let session = context_engine
            .bring_to_mind(&["kernel-version".to_owned()], &ActivationBudget::default());
        assert_eq!(session.items[0].epistemic_status, EpistemicStatus::Disputed);
    }

    #[test]
    fn the_epistemic_owner_settling_a_dispute_is_carried_through() {
        // The other direction, so the rule is not "disputes are permanent". A stated standing
        // replaces a stated standing; only silence does not.
        let context_engine = ContextCore::new();
        let now = OffsetDateTime::now_utc();
        context_engine.activate_with_standing(
            "kernel-version",
            1.0,
            "observed",
            now,
            0,
            EpistemicStatus::Disputed,
        );
        context_engine.activate_with_standing(
            "kernel-version",
            1.0,
            "corroborated",
            now,
            0,
            EpistemicStatus::Observed,
        );

        let session = context_engine
            .bring_to_mind(&["kernel-version".to_owned()], &ActivationBudget::default());
        assert_eq!(session.items[0].epistemic_status, EpistemicStatus::Observed);
        assert!(!session.carries_qualified());
    }

    #[test]
    fn the_same_history_of_activations_always_leaves_the_same_concepts_behind() {
        // Found by a flaky end-to-end test. Overflowing the concept budget with equally salient
        // concepts activated in one sweep left the survivors to hash order, so the same sequence of
        // activations produced a different graph on every run of the process — including runs where
        // the concept being asked about was the one evicted. A1 asks that one snapshot produce one
        // bundle; this is what makes one history produce one snapshot.
        fn overflowed() -> Vec<String> {
            let context_engine = ContextCore::new();
            let now = OffsetDateTime::now_utc();
            for index in 0..(context_engine.budget().nodes + 200) {
                context_engine.activate(format!("concept-{index:04}"), 1.0, "swept", now);
            }
            context_engine
                .active_context()
                .into_iter()
                .map(|node| node.label)
                .collect()
        }

        let survivors = overflowed();
        assert_eq!(survivors.len(), ContextCore::new().budget().nodes);
        for _ in 0..4 {
            assert_eq!(overflowed(), survivors);
        }
    }

    #[test]
    fn the_organ_can_be_asked_what_a_word_brings_to_mind() {
        // The walk is wired to the graph the organ actually holds, and to a real clock. Without
        // this the module would be correct and unreachable, which is how `Realize` sat unused.
        let context_engine = ContextCore::new();
        let now = OffsetDateTime::now_utc();
        context_engine.activate("lemon", 1.0, "observed", now);
        context_engine.activate("honey", 1.0, "observed", now);
        context_engine.associate(
            "lemon",
            "honey",
            0.84,
            AssociationOrigin::Episodic,
            vec![Uuid::from_u128(1)],
        );

        let session =
            context_engine.bring_to_mind(&["lemon".to_owned()], &ActivationBudget::default());
        let honey = session
            .items
            .iter()
            .find(|item| item.label == "honey")
            .expect("honey is one link from lemon");
        assert!(honey.reason.contains("lemon → honey"), "{}", honey.reason);
        assert!(session.complete);
    }

    #[test]
    fn an_erasure_leaves_nothing_for_a_word_to_bring_to_mind() {
        // A7 reaching the surface a person actually queries: the projection is invalidated, so the
        // walk over it finds the graph gone rather than finding an erased concept still reachable.
        let context_engine = ContextCore::new();
        let now = OffsetDateTime::now_utc();
        context_engine.activate("lemon", 1.0, "observed", now);
        context_engine.activate("honey", 1.0, "observed", now);
        context_engine.associate(
            "lemon",
            "honey",
            0.84,
            AssociationOrigin::Episodic,
            vec![Uuid::from_u128(1)],
        );
        assert!(context_engine.invalidate_for_epoch(1));

        let session =
            context_engine.bring_to_mind(&["lemon".to_owned()], &ActivationBudget::default());
        assert!(session.items.is_empty());
        // And it says the seed was not found rather than reporting an empty association set.
        assert!(session.exhausted.contains(&Exhausted::UnknownSeed));
        assert!(!session.complete);
    }

    #[test]
    fn an_association_cannot_come_out_looser_than_its_evidence() {
        let local = Privacy::Local as u8;
        let public = Privacy::Public as u8;

        for (first, second) in [(local, public), (public, local)] {
            let context_engine = ContextCore::new();
            let now = OffsetDateTime::now_utc();
            context_engine.activate("operating-system", 1.0, "observed", now);
            context_engine.activate("kernel-version", 1.0, "observed", now);

            context_engine.associate_with_class(
                "operating-system",
                "kernel-version",
                0.5,
                AssociationOrigin::TemporalCooccurrence,
                vec![Uuid::from_u128(1)],
                first,
                0,
                3,
            );
            context_engine.associate_with_class(
                "operating-system",
                "kernel-version",
                0.9,
                AssociationOrigin::TemporalCooccurrence,
                vec![Uuid::from_u128(2)],
                second,
                1,
                1,
            );

            let bundle = context_engine.bundle(0.0);
            let link = bundle
                .associations
                .iter()
                .find(|a| a.target == "kernel-version")
                .expect("the association exists");
            assert_eq!(
                link.privacy, local,
                "a link derived from something Local must not become Public by meeting one"
            );
            assert_eq!(link.sensitivity, 1, "sensitivity takes the most exposing");
            assert_eq!(link.retention_class, 1, "retention takes the shortest");
            assert_eq!(link.evidence.len(), 2);
        }
    }

    #[test]
    fn the_graph_is_held_within_its_budget_and_keeps_what_is_salient() {
        let context_engine = ContextCore::new();
        let now = OffsetDateTime::now_utc();
        let budget = context_engine.budget().nodes;

        for index in 0..=budget {
            let salience = f64::from(u32::try_from(index).expect("test budget fits"))
                / f64::from(u32::try_from(budget + 1).expect("test budget fits"));
            context_engine.activate(format!("concept-{index}"), salience, "test", now);
        }

        let held = context_engine.active_context();
        assert_eq!(held.len(), budget, "the graph must stay within its budget");
        assert!(
            !held.iter().any(|node| node.label == "concept-0"),
            "the least salient concept is the one that goes"
        );
        assert!(
            held.iter()
                .any(|node| node.label == format!("concept-{budget}")),
            "the most salient concept must survive"
        );
    }

    #[test]
    fn an_erasure_epoch_discards_the_projection_rather_than_outliving_it() {
        let context_engine = ContextCore::new();
        let now = OffsetDateTime::now_utc();
        context_engine.activate("operating-system", 1.0, "observed by perceptiond", now);
        context_engine.associate(
            "operating-system",
            "kernel-version",
            1.0,
            AssociationOrigin::TemporalCooccurrence,
            vec![Uuid::new_v4()],
        );
        assert!(!context_engine.active_context().is_empty());

        assert!(context_engine.invalidate_for_epoch(1));
        assert!(context_engine.active_context().is_empty());
        assert_eq!(context_engine.erasure_epoch(), 1);

        assert!(!context_engine.invalidate_for_epoch(1));
    }

    #[test]
    fn context_bundle_carries_the_provenance_of_what_activated_it() {
        let context_engine = ContextCore::new();
        let now = OffsetDateTime::now_utc();
        let ev1 = Uuid::new_v4();

        context_engine.activate("system-maintenance", 0.9, "scheduled cron trigger", now);
        context_engine.associate(
            "system-maintenance",
            "backup",
            0.85,
            AssociationOrigin::Episodic,
            vec![ev1],
        );

        let bundle = context_engine.bundle(0.5);
        assert_eq!(bundle.items.len(), 1);
        assert_eq!(bundle.items[0].label, "system-maintenance");
        assert_eq!(bundle.items[0].activation_reason, "scheduled cron trigger");
        assert_eq!(bundle.associations.len(), 1);
        assert_eq!(bundle.associations[0].origin, AssociationOrigin::Episodic);
    }

    #[test]
    fn a_restarted_projection_starts_empty_at_the_epoch_it_was_given() {
        let context_engine = ContextCore::resuming_at_epoch(7);
        assert!(context_engine.active_context().is_empty());

        assert!(!context_engine.invalidate_for_epoch(7));
        assert!(context_engine.invalidate_for_epoch(8));
    }
}
