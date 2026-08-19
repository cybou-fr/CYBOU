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
    /// Whether the context search was complete within its query budget.
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
}

impl Default for ContextCore {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextCore {
    /// Create a new transient ContextCore engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state_path: None,
            nodes: RwLock::new(HashMap::new()),
            associations: RwLock::new(Vec::new()),
        }
    }

    /// Open ContextCore with persistent JSON storage.
    ///
    /// # Errors
    ///
    /// Returns [`ContextError`] on I/O error or state corruption.
    pub fn open(path: &Path) -> Result<Self, ContextError> {
        let (nodes, associations) = if path.exists() {
            let mut file = File::open(path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            let state: ContextState = serde_json::from_str(&content)
                .map_err(|e| ContextError::CorruptState(e.to_string()))?;
            (state.nodes, state.associations)
        } else {
            (HashMap::new(), Vec::new())
        };

        Ok(Self {
            state_path: Some(path.to_path_buf()),
            nodes: RwLock::new(nodes),
            associations: RwLock::new(associations),
        })
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
        let assocs = self.associations.read().map(|g| g.clone()).unwrap_or_default();

        let node = candidate_nodes.entry(label_str.clone()).or_insert_with(|| ConceptNode {
            label: label_str,
            salience,
            activation_reason: reason_str.clone(),
            last_activated_at: now,
        });

        node.salience = (node.salience * 0.5 + salience * 0.5).clamp(0.0, 1.0);
        node.activation_reason = reason_str;
        node.last_activated_at = now;

        if self.persist_candidate(&candidate_nodes, &assocs).is_ok() {
            if let Ok(mut lock) = self.nodes.write() {
                *lock = candidate_nodes;
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
        let mut candidate_assocs = self.associations.read().map(|g| g.clone()).unwrap_or_default();

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

        if self.persist_candidate(&nodes, &candidate_assocs).is_ok() {
            if let Ok(mut lock) = self.associations.write() {
                *lock = candidate_assocs;
            }
        }
    }

    /// Retrieve active concept bundle bounded by salience threshold.
    #[must_use]
    pub fn bundle(&self, min_salience: f64) -> ContextBundle {
        let nodes_map = self.nodes.read().map(|g| g.clone()).unwrap_or_default();
        let assocs_list = self.associations.read().map(|g| g.clone()).unwrap_or_default();

        let mut items: Vec<_> = nodes_map
            .into_values()
            .filter(|n| n.salience >= min_salience)
            .collect();
        items.sort_by(|a, b| b.salience.partial_cmp(&a.salience).unwrap_or(std::cmp::Ordering::Equal));

        let item_labels: Vec<_> = items.iter().map(|n| n.label.clone()).collect();
        let relevant_assocs: Vec<_> = assocs_list
            .into_iter()
            .filter(|a| item_labels.contains(&a.source) || item_labels.contains(&a.target))
            .collect();

        ContextBundle {
            items,
            associations: relevant_assocs,
            complete: true,
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
        let assocs = self.associations.read().map(|g| g.clone()).unwrap_or_default();
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

#[cfg(test)]
mod tests {
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
