// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Associative and situational context management engine (ADR-0029: association != truth).
//!
//! Maintains active context vectors, associative graphs between entities and concepts,
//! tracking explicit provenance (`why?`, `origin`, `evidence`) and producing inspectable
//! bounded `ContextBundle` projections.

pub mod core;
pub mod types;

#[cfg(target_os = "linux")]
pub mod service;

pub use core::{ContextCore, enforce_edge_budget, enforce_node_budget};
pub use types::{
    Association, AssociationOrigin, ConceptNode, ContextBundle, ContextBudget,
    most_restrictive_privacy, shortest_retention,
};

#[cfg(test)]
mod tests {
    use cybou_protocol::admission::Privacy;
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;

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
