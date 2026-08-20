// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Associative and situational context management engine (ADR-0029: association != truth).
//!
//! Maintains active context vectors, associative graphs between entities and concepts,
//! tracking explicit provenance (`why?`, `origin`, `evidence`) and producing inspectable
//! bounded `ContextBundle` projections.

use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::RwLock,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
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

/// Persistent snapshot of context state.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextState {
    /// Active concepts.
    pub nodes: HashMap<String, ConceptNode>,
    /// Associative links.
    pub associations: Vec<Association>,
    /// The erasure epoch these associations were derived under.
    ///
    /// Absent in state written before this field existed, which is state that was never checked
    /// against an erasure and therefore must be rebuilt rather than trusted.
    #[serde(default)]
    pub erasure_epoch: u64,
}

/// Errors occurring in the context organ.
#[derive(Debug, Error)]
pub enum ContextError {
    /// I/O error reading or writing state.
    #[error("context state i/o error: {0}")]
    Io(#[from] std::io::Error),
    /// State file corrupted.
    #[error("context state file corrupted: {0}")]
    CorruptState(String),
    /// Internal lock poisoned.
    #[error("context lock poisoned")]
    LockPoisoned,
}

/// Core domain logic of the associative context organ.
pub struct ContextCore {
    state_path: Option<PathBuf>,
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
            state_path: None,
            nodes: RwLock::new(HashMap::new()),
            associations: RwLock::new(Vec::new()),
            budget: ContextBudget::default(),
            erasure_epoch: RwLock::new(0),
        }
    }

    /// Open `ContextCore` with persistent JSON storage.
    ///
    /// # Errors
    ///
    /// Returns [`ContextError`] on I/O error or state corruption.
    pub fn open(path: &Path) -> Result<Self, ContextError> {
        let (nodes, associations, erasure_epoch) = if path.exists() {
            let mut file = File::open(path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            let state: ContextState = serde_json::from_str(&content)
                .map_err(|e| ContextError::CorruptState(e.to_string()))?;
            (state.nodes, state.associations, state.erasure_epoch)
        } else {
            (HashMap::new(), Vec::new(), 0)
        };

        Ok(Self {
            state_path: Some(path.to_path_buf()),
            nodes: RwLock::new(nodes),
            associations: RwLock::new(associations),
            budget: ContextBudget::default(),
            erasure_epoch: RwLock::new(erasure_epoch),
        })
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
        let _ = self.persist_current();
        true
    }

    /// The erasure epoch this projection was last derived under.
    #[must_use]
    pub fn erasure_epoch(&self) -> u64 {
        self.erasure_epoch.read().map_or(0, |guard| *guard)
    }

    fn persist_current(&self) -> Result<(), ContextError> {
        let nodes = self.nodes.read().map(|g| g.clone()).unwrap_or_default();
        let associations = self
            .associations
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        self.persist_candidate(&nodes, &associations)
    }

    fn persist_candidate(
        &self,
        nodes: &HashMap<String, ConceptNode>,
        associations: &[Association],
    ) -> Result<(), ContextError> {
        if let Some(path) = &self.state_path {
            let state = ContextState {
                nodes: nodes.clone(),
                associations: associations.to_vec(),
                erasure_epoch: self.erasure_epoch(),
            };
            let serialized = serde_json::to_string_pretty(&state)
                .map_err(|e| ContextError::CorruptState(e.to_string()))?;

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let temp_path = path.with_extension("tmp");
            {
                let mut temp_file = File::create(&temp_path)?;
                temp_file.write_all(serialized.as_bytes())?;
                temp_file.sync_all()?;
            }
            fs::rename(&temp_path, path)?;
        }
        Ok(())
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

        if self.persist_candidate(&candidate_nodes, &assocs).is_ok() {
            if let Ok(mut lock) = self.nodes.write() {
                *lock = candidate_nodes;
            }
            if let Ok(mut lock) = self.associations.write() {
                *lock = assocs;
            }
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
        let source_str = source.into();
        let target_str = target.into();

        let nodes = self.nodes.read().map(|g| g.clone()).unwrap_or_default();
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
        } else {
            candidate_assocs.push(Association {
                source: source_str,
                target: target_str,
                strength: strength.clamp(0.0, 1.0),
                origin,
                evidence,
            });
        }

        enforce_edge_budget(&mut candidate_assocs, self.budget.edges);

        if self.persist_candidate(&nodes, &candidate_assocs).is_ok()
            && let Ok(mut lock) = self.associations.write()
        {
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

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn context_bundle_provenance_and_persistence() {
        let dir = tempdir().expect("tempdir");
        let state_path = dir.path().join("context.json");

        let core = ContextCore::open(&state_path).expect("open");
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

        // Reopen from disk: survives restart
        let reopened = ContextCore::open(&state_path).expect("reopen");
        let reopened_bundle = reopened.bundle(0.5);
        assert_eq!(reopened_bundle.items.len(), 1);
        assert_eq!(reopened_bundle.associations.len(), 1);
    }
}
