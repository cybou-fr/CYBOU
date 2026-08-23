// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Associative and situational context types, nodes, bundles, and budgets.

use cybou_protocol::{admission::Privacy, epistemic::EpistemicStatus};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

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
    #[serde(default)]
    pub privacy: u8,
    /// Sensitivity inherited from the evidence, the most exposing of them.
    #[serde(default)]
    pub sensitivity: u8,
    /// Retention class inherited from the evidence, shortest-lived of them.
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
    /// The highest sensitivity among the contributions that activated this concept.
    #[serde(default)]
    pub sensitivity: u8,
    /// How `epistemicd` stood on what this concept was derived from.
    ///
    /// ADR-0029 A4: a disputed state is still disputed after retrieval. Carried rather than
    /// recomputed — `contextd` may read the layer above it and may not overrule it, so this field
    /// only ever holds what the epistemic owner decided.
    #[serde(default)]
    pub epistemic_status: EpistemicStatus,
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
    pub complete: bool,
}

/// How much of an associative graph is allowed to exist at once.
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

/// The more restrictive of two privacy classes, on the frozen scale where `Local` is 0.
#[must_use]
pub fn most_restrictive_privacy(left: u8, right: u8) -> u8 {
    match (Privacy::from_u8(left), Privacy::from_u8(right)) {
        (Some(left), Some(right)) => left.most_restrictive(right) as u8,
        _ => Privacy::Local as u8,
    }
}

/// The shorter of two retention classes, where zero means unstated.
#[must_use]
pub fn shortest_retention(left: u8, right: u8) -> u8 {
    match (left, right) {
        (0, other) | (other, 0) => other,
        (left, right) => left.min(right),
    }
}
