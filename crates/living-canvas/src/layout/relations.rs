// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Unified semantic relationship graph for mind organ connections and causal layout.

use serde::{Deserialize, Serialize};

use crate::{CardId, DesktopItemId, DesktopLayout, layout::model::DesktopItem};

/// Semantic relationship classification between mind capabilities.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RelationshipKind {
    /// Cryptographic or provenance validation (`Identity` -> `Session`).
    Proves,
    /// Auditing and admission tracking (`Capabilities` -> `Journal`).
    Audits,
    /// Execution control and lifecycle suspension (`Lifecycle` -> `Commitments`).
    Suspends,
    /// Introspective guidance and attention biasing (`SelfModel` -> `Attention`).
    Guides,
    /// Empirical observation updating propositions (`Beliefs` -> `Perception`).
    Updates,
    /// Associative priming of focal workspace (`Context` -> `Attention`).
    Primes,
}

/// A directional semantic dependency edge between two cards.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Relationship {
    /// Source card.
    pub from: CardId,
    /// Target dependent card.
    pub to: CardId,
    /// Semantic classification.
    pub kind: RelationshipKind,
    /// Human-readable relationship label.
    pub label: &'static str,
    /// Amber highlight flag for degraded or warning connections.
    pub amber: bool,
}

/// Canonical relationship graph provider.
pub struct DesktopRelationshipGraph;

impl DesktopRelationshipGraph {
    /// Canonical list of all semantic connections between Mind organs.
    #[must_use]
    pub const fn canonical() -> &'static [Relationship] {
        &[
            Relationship {
                from: CardId::Identity,
                to: CardId::Session,
                kind: RelationshipKind::Proves,
                label: "proves",
                amber: false,
            },
            Relationship {
                from: CardId::Capabilities,
                to: CardId::Journal,
                kind: RelationshipKind::Audits,
                label: "audits",
                amber: false,
            },
            Relationship {
                from: CardId::Lifecycle,
                to: CardId::Commitments,
                kind: RelationshipKind::Suspends,
                label: "suspends",
                amber: false,
            },
            Relationship {
                from: CardId::SelfModel,
                to: CardId::Attention,
                kind: RelationshipKind::Guides,
                label: "guides",
                amber: false,
            },
            Relationship {
                from: CardId::Beliefs,
                to: CardId::Perception,
                kind: RelationshipKind::Updates,
                label: "updates",
                amber: false,
            },
            Relationship {
                from: CardId::Context,
                to: CardId::Attention,
                kind: RelationshipKind::Primes,
                label: "primes",
                amber: false,
            },
        ]
    }

    /// Determine the topological layer index (0 to 4) for a given card based on the relationship graph.
    #[must_use]
    pub const fn layer_for_card(card: CardId) -> usize {
        match card {
            CardId::Identity | CardId::Session | CardId::Perception | CardId::Lifecycle => 0,
            CardId::Capabilities | CardId::Journal => 1,
            CardId::Context | CardId::Beliefs => 2,
            CardId::Commitments | CardId::Attention | CardId::SelfModel => 3,
            // Tool cards (Shell, Files, Feed), and Disclosure, which is a system card but not an
            // organ: it describes what leaves, so it sits after everything that produces it.
            _ => 4,
        }
    }

    /// Determine the layer index for any desktop item (Card or Deck).
    #[must_use]
    pub fn layer_for_item(item: &DesktopItem, layout: &DesktopLayout) -> usize {
        match &item.id {
            DesktopItemId::Card(id) => Self::layer_for_card(*id),
            DesktopItemId::Deck(d_id) => {
                // Deck inherits the minimum layer among its contained cards
                layout
                    .deck(d_id)
                    .and_then(|d| d.card_ids.iter().map(|c| Self::layer_for_card(*c)).min())
                    .unwrap_or(1)
            }
        }
    }
}
