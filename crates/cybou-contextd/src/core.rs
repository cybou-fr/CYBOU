// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Core domain logic and associative graph operations for `ContextCore`.

use std::{
    collections::HashMap,
    sync::{
        RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

use cybou_protocol::epistemic::EpistemicStatus;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    activation::{ActivationBudget, ActivationSession, activate_from},
    types::{
        Association, AssociationOrigin, ConceptNode, ContextBudget, ContextBundle,
        most_restrictive_privacy, shortest_retention,
    },
};

/// Core domain logic of the associative context organ.
pub struct ContextCore {
    caught_up: AtomicBool,
    nodes: RwLock<HashMap<String, ConceptNode>>,
    associations: RwLock<Vec<Association>>,
    budget: ContextBudget,
    erasure_epoch: RwLock<u64>,
}

impl Default for ContextCore {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextCore {
    /// Record that every contribution the Journal already held has now been delivered.
    pub fn mark_caught_up(&self) {
        self.caught_up.store(true, Ordering::Release);
    }

    /// Whether this projection has seen the whole Journal at least once.
    #[must_use]
    pub fn is_caught_up(&self) -> bool {
        self.caught_up.load(Ordering::Acquire)
    }

    /// Create a new transient `ContextCore` engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            caught_up: AtomicBool::new(false),
            nodes: RwLock::new(HashMap::new()),
            associations: RwLock::new(Vec::new()),
            budget: ContextBudget::default(),
            erasure_epoch: RwLock::new(0),
        }
    }

    /// Begin from the erasure epoch the Journal is currently at.
    #[must_use]
    pub fn resuming_at_epoch(epoch: u64) -> Self {
        Self {
            caught_up: AtomicBool::new(false),
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
        self.activate_classified(label, salience, reason, now, 0);
    }

    /// Activate a concept, carrying the sensitivity of the contribution that activated it.
    pub fn activate_classified(
        &self,
        label: impl Into<String>,
        salience: f64,
        reason: impl Into<String>,
        now: OffsetDateTime,
        sensitivity: u8,
    ) {
        self.activate_with_standing(
            label,
            salience,
            reason,
            now,
            sensitivity,
            EpistemicStatus::Unknown,
        );
    }

    /// Activate a concept, carrying how the epistemic owner stood on what produced it.
    ///
    /// A stated standing replaces whatever was held; `Unknown` never does. A caller that did not
    /// know is not evidence that a dispute went away, and letting silence overwrite `Disputed`
    /// would lose a dispute at the one boundary A4 exists to hold.
    #[allow(
        clippy::too_many_arguments,
        reason = "each argument is a distinct fact about one activation"
    )]
    pub fn activate_with_standing(
        &self,
        label: impl Into<String>,
        salience: f64,
        reason: impl Into<String>,
        now: OffsetDateTime,
        sensitivity: u8,
        epistemic_status: EpistemicStatus,
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
                sensitivity,
                epistemic_status,
            });

        node.salience = (node.salience * 0.5 + salience * 0.5).clamp(0.0, 1.0);
        node.activation_reason = reason_str;
        node.last_activated_at = now;
        node.sensitivity = node.sensitivity.max(sensitivity);
        if epistemic_status != EpistemicStatus::Unknown {
            node.epistemic_status = epistemic_status;
        }

        let dropped = enforce_node_budget(&mut candidate_nodes, self.budget.nodes);
        let assocs = if dropped.is_empty() {
            assocs
        } else {
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
        self.associate_with_class(source, target, strength, origin, evidence, 0, 0, 0);
    }

    /// Link two concepts, inheriting privacy and retention from the contributions behind them.
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
                // By label when salience ties, which it does constantly — concepts activated in one
                // sweep share a score. Without this the bundle came back in whatever order the hash
                // yielded, so the same graph produced a differently ordered answer on every run of
                // the process, and A1 is about the bundle, not only about which concepts are in it.
                .then_with(|| a.label.cmp(&b.label))
        });

        let item_labels: Vec<_> = items.iter().map(|n| n.label.clone()).collect();
        let relevant_assocs: Vec<_> = assocs_list
            .into_iter()
            .filter(|a| item_labels.contains(&a.source) || item_labels.contains(&a.target))
            .collect();

        let excluded_by_salience = self.nodes.read().map_or(0, |nodes| {
            nodes.values().filter(|n| n.salience < min_salience).count()
        });

        ContextBundle {
            items,
            associations: relevant_assocs,
            complete: excluded_by_salience == 0,
        }
    }

    /// Walk the associations from `seeds`, bounded by `budget`.
    ///
    /// The clock is wired here rather than inside the walk, because a walk that reaches for a clock
    /// cannot be tested against one. What it hands over is elapsed time and nothing else: the
    /// instant is never an input to what gets reached, only to when the reaching stops.
    #[must_use]
    pub fn bring_to_mind(&self, seeds: &[String], budget: &ActivationBudget) -> ActivationSession {
        let nodes = self.nodes.read().map(|g| g.clone()).unwrap_or_default();
        let associations = self
            .associations
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        let started = std::time::Instant::now();
        activate_from(&nodes, &associations, seeds, budget, || started.elapsed())
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

/// Hold the concept count within its budget, returning the labels that were dropped.
pub fn enforce_node_budget<S: std::hash::BuildHasher>(
    nodes: &mut HashMap<String, ConceptNode, S>,
    budget: usize,
) -> Vec<String> {
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
            // By label last, because the two keys above tie constantly: concepts activated in one
            // sweep share an instant and often a salience, and without this the survivors were
            // whatever the hash happened to order first — different on every run of the process.
            // A1 asks that one snapshot produce one bundle; this is what makes one *history*
            // produce one snapshot, which is the assumption A1 rests on.
            .then_with(|| a.0.cmp(&b.0))
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
pub fn enforce_edge_budget(associations: &mut Vec<Association>, budget: usize) {
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
