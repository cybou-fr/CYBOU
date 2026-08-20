// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Associative and situational context management engine (ADR-0029: association != truth).
//!
//! Maintains active context vectors, associative graphs between entities and concepts,
//! tracking explicit provenance (`why?`, `origin`, `evidence`) and producing inspectable
//! bounded `ContextBundle` projections.

use std::{collections::HashMap, sync::RwLock};

use cybou_protocol::admission::Privacy;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[cfg(target_os = "linux")]
pub mod service;

/// Origin/derivation source of an associative relation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssociationOrigin {
    /// Derived from an observed cognitive episode or perception.
    Episodic,
    /// Derived from structured epistemic propositions.
    Epistemic,
    /// Temporal co-occurrence in the conscious workspace.
    TemporalCooccurrence,
    /// User explicit instruction.
    UserExplicit,
    /// Static knowledge graph.
    StaticKnowledge,
}

/// An associative link between two concepts.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Association {
    /// Source concept label.
    pub source: String,
    /// Target concept label.
    pub target: String,
    /// Associative strength in [0.0, 1.0].
    pub strength: f64,
    /// Provenance origin.
    pub origin: AssociationOrigin,
    /// Contributing evidence / causal message IDs.
    pub evidence: Vec<Uuid>,
    /// Privacy class inherited from the evidence, most restrictive of them.
    ///
    /// ADR-0029 A9. An association is a claim derived from contributions, and a derived claim that
    /// is less private than what it was derived from is a way to launder a private fact into a
    /// public one by observing it twice.
    ///
    /// On the frozen scale `Local` is 0 and `Public` is 3, so more restrictive is *lower*. Reading
    /// this as an ordinary number and taking the larger of two is how a `Local` fact became
    /// `Public` by being seen beside one.
    #[serde(default)]
    pub privacy: u8,
    /// Sensitivity inherited from the evidence, the most exposing of them.
    #[serde(default)]
    pub sensitivity: u8,
    /// Retention class inherited from the evidence, shortest-lived of them.
    ///
    /// An association that outlives its evidence would keep asserting a link whose contributions
    /// are gone.
    #[serde(default)]
    pub retention_class: u8,
}

/// An active situational context element.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptNode {
    /// Concept tag or label.
    pub label: String,
    /// Salience / activation weight in [0.0, 1.0].
    pub salience: f64,
    /// Why this concept was activated (answers "why was I retrieved?").
    pub activation_reason: String,
    /// When this concept was last activated.
    #[serde(with = "time::serde::rfc3339")]
    pub last_activated_at: OffsetDateTime,
}

/// Bounded context bundle returned for cognitive queries.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextBundle {
    /// Active concept items.
    pub items: Vec<ConceptNode>,
    /// Relevant associative links between items.
    pub associations: Vec<Association>,
    /// Whether the search covered everything the query asked for within its budget.
    ///
    /// Answering `true` unconditionally would claim a completeness nobody established. ADR-0029
    /// gives this a node, edge, depth, time and token budget; today the only bound is a salience
    /// floor, so a bundle is complete exactly when nothing was left out by that floor.
    pub complete: bool,
}

/// Core domain logic of the associative context organ.
pub struct ContextCore {
    nodes: RwLock<HashMap<String, ConceptNode>>,
    associations: RwLock<Vec<Association>>,
    /// The budget this projection is held within.
    budget: ContextBudget,
    /// The erasure epoch this projection was derived under.
    ///
    /// ADR-0029 A7: an erasure epoch invalidates the associative projection. A derived index that
    /// outlives an erasure keeps associations whose evidence has been destroyed, which is the one
    /// way an index can resurrect what a person asked to be gone.
    erasure_epoch: RwLock<u64>,
}

/// How much of an associative graph is allowed to exist at once.
///
/// ADR-0029 A2 and A11: activation is bounded by explicit budgets, and the workspace stays bounded
/// even when activation would return thousands of associations. Only the node and edge dimensions
/// are enforced here; depth, time and token budgets belong to the activation session this version
/// does not have, and claiming them would be worse than admitting their absence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextBudget {
    /// Most concepts retained.
    pub nodes: usize,
    /// Most associations retained.
    pub edges: usize,
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self {
            nodes: 512,
            edges: 2048,
        }
    }
}

impl Default for ContextCore {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextCore {
    /// Create a new transient `ContextCore` engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            associations: RwLock::new(Vec::new()),
            budget: ContextBudget::default(),
            erasure_epoch: RwLock::new(0),
        }
    }

    /// Begin from the erasure epoch the Journal is currently at.
    ///
    /// The graph itself is not carried across a restart, because it is not remembered: every node
    /// and every link is derived from contributions the Journal still holds, and the organ rebuilds
    /// it by replaying them. A saved copy would have been the one thing in the system able to
    /// outlive its own evidence — the associations it holds are exactly what ADR-0029 A7 says must
    /// not survive an erasure, and reloading it put them back for as long as it took to notice.
    ///
    /// The epoch is not derived, so it is given: starting at zero would make the first check after
    /// a start look like a fresh erasure of a graph that was built after it.
    #[must_use]
    pub fn resuming_at_epoch(epoch: u64) -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            associations: RwLock::new(Vec::new()),
            budget: ContextBudget::default(),
            erasure_epoch: RwLock::new(epoch),
        }
    }

    /// The budget this projection is held within.
    #[must_use]
    pub const fn budget(&self) -> ContextBudget {
        self.budget
    }

    /// Discard the projection when the Journal reports an erasure epoch this one predates.
    ///
    /// ADR-0029 A7. An index that survives an erasure keeps associations whose evidence no longer
    /// exists, which is how a derived structure resurrects what a person asked to be gone. It is
    /// rebuilt from the Journal afterwards, so nothing that still exists is lost.
    ///
    /// Returns whether the projection was discarded.
    pub fn invalidate_for_epoch(&self, epoch: u64) -> bool {
        let known = self.erasure_epoch.read().map_or(0, |guard| *guard);
        if epoch <= known {
            return false;
        }
        if let Ok(mut nodes) = self.nodes.write() {
            nodes.clear();
        }
        if let Ok(mut associations) = self.associations.write() {
            associations.clear();
        }
        if let Ok(mut current) = self.erasure_epoch.write() {
            *current = epoch;
        }
        true
    }

    /// The erasure epoch this projection was last derived under.
    #[must_use]
    pub fn erasure_epoch(&self) -> u64 {
        self.erasure_epoch.read().map_or(0, |guard| *guard)
    }

    /// Activate or update a situational concept node.
    pub fn activate(
        &self,
        label: impl Into<String>,
        salience: f64,
        reason: impl Into<String>,
        now: OffsetDateTime,
    ) {
        let label_str = label.into();
        let reason_str = reason.into();

        let mut candidate_nodes = self.nodes.read().map(|g| g.clone()).unwrap_or_default();
        let assocs = self
            .associations
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();

        let node = candidate_nodes
            .entry(label_str.clone())
            .or_insert_with(|| ConceptNode {
                label: label_str,
                salience,
                activation_reason: reason_str.clone(),
                last_activated_at: now,
            });

        node.salience = (node.salience * 0.5 + salience * 0.5).clamp(0.0, 1.0);
        node.activation_reason = reason_str;
        node.last_activated_at = now;

        let dropped = enforce_node_budget(&mut candidate_nodes, self.budget.nodes);
        let assocs = if dropped.is_empty() {
            assocs
        } else {
            // An association whose end no longer exists is not an association. Dropping the edge
            // with the node keeps the graph from holding links into nothing.
            assocs
                .into_iter()
                .filter(|link| !dropped.contains(&link.source) && !dropped.contains(&link.target))
                .collect()
        };

        if let Ok(mut lock) = self.nodes.write() {
            *lock = candidate_nodes;
        }
        if let Ok(mut lock) = self.associations.write() {
            *lock = assocs;
        }
    }

    /// Link two concepts associatively with full provenance.
    pub fn associate(
        &self,
        source: impl Into<String>,
        target: impl Into<String>,
        strength: f64,
        origin: AssociationOrigin,
        evidence: Vec<Uuid>,
    ) {
        // Unclassified evidence is treated as the most restrictive thing it could be: an
        // association that does not know what it came from must not claim it may go anywhere.
        self.associate_with_class(source, target, strength, origin, evidence, 0, 0, 0);
    }

    /// Link two concepts, inheriting privacy and retention from the contributions behind them.
    ///
    /// The caller supplies the classes of the evidence because it is the caller that saw the
    /// envelopes; this decides what to keep. Most restrictive privacy and shortest retention win,
    /// which is the only direction that cannot make a derived claim looser than its sources.
    #[allow(
        clippy::too_many_arguments,
        reason = "each argument is a distinct fact about one link"
    )]
    pub fn associate_with_class(
        &self,
        source: impl Into<String>,
        target: impl Into<String>,
        strength: f64,
        origin: AssociationOrigin,
        evidence: Vec<Uuid>,
        privacy: u8,
        sensitivity: u8,
        retention_class: u8,
    ) {
        let source_str = source.into();
        let target_str = target.into();

        let mut candidate_assocs = self
            .associations
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();

        // Update or insert
        if let Some(existing) = candidate_assocs
            .iter_mut()
            .find(|a| a.source == source_str && a.target == target_str)
        {
            existing.strength = strength.clamp(0.0, 1.0);
            for ev in evidence {
                if !existing.evidence.contains(&ev) {
                    existing.evidence.push(ev);
                }
            }
            // Corroborating a link with more evidence can only tighten what it inherits: new
            // evidence adds an obligation, it never relaxes one already carried.
            existing.privacy = most_restrictive_privacy(existing.privacy, privacy);
            existing.sensitivity = existing.sensitivity.max(sensitivity);
            existing.retention_class =
                shortest_retention(existing.retention_class, retention_class);
        } else {
            candidate_assocs.push(Association {
                source: source_str,
                target: target_str,
                strength: strength.clamp(0.0, 1.0),
                origin,
                evidence,
                privacy,
                sensitivity,
                retention_class,
            });
        }

        enforce_edge_budget(&mut candidate_assocs, self.budget.edges);

        if let Ok(mut lock) = self.associations.write() {
            *lock = candidate_assocs;
        }
    }

    /// Retrieve active concept bundle bounded by salience threshold.
    #[must_use]
    pub fn bundle(&self, min_salience: f64) -> ContextBundle {
        let nodes_map = self.nodes.read().map(|g| g.clone()).unwrap_or_default();
        let assocs_list = self
            .associations
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();

        let mut items: Vec<_> = nodes_map
            .into_values()
            .filter(|n| n.salience >= min_salience)
            .collect();
        items.sort_by(|a, b| {
            b.salience
                .partial_cmp(&a.salience)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let item_labels: Vec<_> = items.iter().map(|n| n.label.clone()).collect();
        let relevant_assocs: Vec<_> = assocs_list
            .into_iter()
            .filter(|a| item_labels.contains(&a.source) || item_labels.contains(&a.target))
            .collect();

        // Complete against the only budget this version applies. When the bounded activation of
        // ADR-0029 arrives, this is where its budgets decide the same field.
        let excluded_by_salience = self.nodes.read().map_or(0, |nodes| {
            nodes.values().filter(|n| n.salience < min_salience).count()
        });

        ContextBundle {
            items,
            associations: relevant_assocs,
            complete: excluded_by_salience == 0,
        }
    }

    /// Return active concept nodes ordered by salience descending.
    #[must_use]
    pub fn active_context(&self) -> Vec<ConceptNode> {
        self.bundle(0.0).items
    }

    /// Return related concept labels for a given tag.
    #[must_use]
    pub fn related_tags(&self, tag: &str) -> Vec<String> {
        let assocs = self
            .associations
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        let mut list = Vec::new();
        for a in assocs {
            if a.source == tag && !list.contains(&a.target) {
                list.push(a.target);
            } else if a.target == tag && !list.contains(&a.source) {
                list.push(a.source);
            }
        }
        list.sort();
        list
    }
}

/// The more restrictive of two privacy classes, on the frozen scale where `Local` is 0.
///
/// Delegating to the protocol rather than comparing numbers: the scale runs from most restrictive
/// to least, so the arithmetic that looks right is the one that leaks. A value the protocol does
/// not recognise is treated as the most restrictive there is, because an unknown classification is
/// not permission.
fn most_restrictive_privacy(left: u8, right: u8) -> u8 {
    match (Privacy::from_u8(left), Privacy::from_u8(right)) {
        (Some(left), Some(right)) => left.most_restrictive(right) as u8,
        _ => Privacy::Local as u8,
    }
}

/// The shorter of two retention classes, where zero means unstated.
fn shortest_retention(left: u8, right: u8) -> u8 {
    match (left, right) {
        (0, other) | (other, 0) => other,
        (left, right) => left.min(right),
    }
}

/// Hold the concept count within its budget, returning the labels that were dropped.
///
/// Least salient first, and oldest first among equals. A budget that dropped by insertion order
/// would forget what the system is attending to in favour of what it happened to see first.
fn enforce_node_budget(nodes: &mut HashMap<String, ConceptNode>, budget: usize) -> Vec<String> {
    if nodes.len() <= budget {
        return Vec::new();
    }
    let mut ranked: Vec<_> = nodes
        .values()
        .map(|node| (node.label.clone(), node.salience, node.last_activated_at))
        .collect();
    ranked.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.2.cmp(&b.2))
    });

    let dropped: Vec<String> = ranked
        .into_iter()
        .take(nodes.len() - budget)
        .map(|(label, _, _)| label)
        .collect();
    for label in &dropped {
        nodes.remove(label);
    }
    dropped
}

/// Hold the association count within its budget, weakest first.
fn enforce_edge_budget(associations: &mut Vec<Association>, budget: usize) {
    if associations.len() <= budget {
        return;
    }
    associations.sort_by(|a, b| {
        b.strength
            .partial_cmp(&a.strength)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    associations.truncate(budget);
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_association_cannot_come_out_looser_than_its_evidence() {
        // Local is 0 and Public is 3: on this scale more restrictive is lower, and the version of
        // this test that used privacy 4 as "stricter" was asserting a scale that does not exist.
        let local = super::Privacy::Local as u8;
        let public = super::Privacy::Public as u8;

        for (first, second) in [(local, public), (public, local)] {
            let core = super::ContextCore::new();
            let now = time::OffsetDateTime::now_utc();
            core.activate("operating-system", 1.0, "observed", now);
            core.activate("kernel-version", 1.0, "observed", now);

            core.associate_with_class(
                "operating-system",
                "kernel-version",
                0.5,
                super::AssociationOrigin::TemporalCooccurrence,
                vec![uuid::Uuid::from_u128(1)],
                first,
                0,
                3,
            );
            core.associate_with_class(
                "operating-system",
                "kernel-version",
                0.9,
                super::AssociationOrigin::TemporalCooccurrence,
                vec![uuid::Uuid::from_u128(2)],
                second,
                1,
                1,
            );

            let bundle = core.bundle(0.0);
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
        let core = super::ContextCore::new();
        let now = time::OffsetDateTime::now_utc();
        let budget = core.budget().nodes;

        // One more concept than the budget allows, with salience rising as they are added.
        for index in 0..=budget {
            let salience = f64::from(u32::try_from(index).expect("test budget fits"))
                / f64::from(u32::try_from(budget + 1).expect("test budget fits"));
            core.activate(format!("concept-{index}"), salience, "test", now);
        }

        let held = core.active_context();
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
        let core = super::ContextCore::new();
        let now = time::OffsetDateTime::now_utc();
        core.activate("operating-system", 1.0, "observed by perceptiond", now);
        core.associate(
            "operating-system",
            "kernel-version",
            1.0,
            super::AssociationOrigin::TemporalCooccurrence,
            vec![uuid::Uuid::new_v4()],
        );
        assert!(!core.active_context().is_empty());

        // An index that survives an erasure keeps associations whose evidence is gone, which is
        // how a derived structure resurrects what a person asked to be destroyed.
        assert!(core.invalidate_for_epoch(1));
        assert!(core.active_context().is_empty());
        assert_eq!(core.erasure_epoch(), 1);

        // The same epoch twice is not a second erasure.
        assert!(!core.invalidate_for_epoch(1));
    }

    use super::*;

    #[test]
    fn context_bundle_carries_the_provenance_of_what_activated_it() {
        let core = ContextCore::new();
        let now = OffsetDateTime::now_utc();
        let ev1 = Uuid::new_v4();

        core.activate("system-maintenance", 0.9, "scheduled cron trigger", now);
        core.associate(
            "system-maintenance",
            "backup",
            0.85,
            AssociationOrigin::Episodic,
            vec![ev1],
        );

        let bundle = core.bundle(0.5);
        assert_eq!(bundle.items.len(), 1);
        assert_eq!(bundle.items[0].label, "system-maintenance");
        assert_eq!(bundle.items[0].activation_reason, "scheduled cron trigger");
        assert_eq!(bundle.associations.len(), 1);
        assert_eq!(bundle.associations[0].origin, AssociationOrigin::Episodic);
    }

    #[test]
    fn a_restarted_projection_starts_empty_at_the_epoch_it_was_given() {
        // Nothing is carried across a restart: the graph is rebuilt from the Journal, and a saved
        // copy would keep associations whose evidence an erasure has already destroyed.
        let core = ContextCore::resuming_at_epoch(7);
        assert!(core.active_context().is_empty());

        // The epoch it starts at is one it already knows about, so the next check does not read a
        // completed erasure as a fresh one and discard a graph built after it.
        assert!(!core.invalidate_for_epoch(7));
        assert!(core.invalidate_for_epoch(8));
    }
}
