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
    /// Empirical observation updating propositions (`Perception` -> `Beliefs`).
    ///
    /// The direction was `Beliefs` -> `Perception` until 2026-08-22, which contradicted this line
    /// and the wiring it describes: `perceptiond` observes the host and submits to Event1, and
    /// `epistemicd` forms beliefs keyed by the subject of each observation. Nothing flows the other
    /// way. It mattered little while these edges only drew a line; they now decide where a card is
    /// placed, so a reversed edge became a reversed desktop.
    Updates,
    /// Associative priming of focal workspace (`Context` -> `Attention`).
    Primes,
    /// Supplying a projection across a boundary, and recording what was withheld.
    ///
    /// The organs named here are the ones the gateway's disclosure bookkeeping actually counts:
    /// `Intention1` withholds obligations entirely, and `Epistemic1` and `Context1` each supply or
    /// withhold per item. No other organ contributes to a `ContextDisclosed`, so no other organ
    /// has this edge.
    Discloses,
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
                from: CardId::Perception,
                to: CardId::Beliefs,
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
            Relationship {
                from: CardId::Commitments,
                to: CardId::Disclosure,
                kind: RelationshipKind::Discloses,
                label: "discloses",
                amber: false,
            },
            Relationship {
                from: CardId::Beliefs,
                to: CardId::Disclosure,
                kind: RelationshipKind::Discloses,
                label: "discloses",
                amber: false,
            },
            Relationship {
                from: CardId::Context,
                to: CardId::Disclosure,
                kind: RelationshipKind::Discloses,
                label: "discloses",
                amber: false,
            },
        ]
    }

    /// How far a card sits from the organs nothing feeds.
    ///
    /// Computed from [`Self::canonical`] rather than listed. Until 2026-08-22 this was a hand-kept
    /// table, and it had drifted away from the edges it was supposed to summarise: `Identity`
    /// proved `Session` while both sat in layer 0, `Capabilities` audited `Journal` from inside
    /// layer 1, and `Beliefs` reached `Perception` backwards through two of them. The graph drew
    /// one story and the desktop arranged another.
    ///
    /// A card's layer is the longest path into it, so an edge always points forward by at least
    /// one column and the arrangement can be read as the causality it came from. Cards the graph
    /// does not mention — the tool cards — sit after every organ, because they consume what the
    /// organs produce and feed none of it back.
    #[must_use]
    pub fn layer_for_card(card: CardId) -> usize {
        if card.is_system() {
            Self::longest_path_into(card, Self::canonical().len())
        } else {
            Self::layer_count() - 1
        }
    }

    /// How many layers the canonical graph produces, including the one the tool cards occupy.
    #[must_use]
    pub fn layer_count() -> usize {
        let deepest = CardId::ALL_SYSTEM_CARDS
            .iter()
            .map(|card| Self::longest_path_into(*card, Self::canonical().len()))
            .max()
            .unwrap_or(0);
        deepest + 2
    }

    /// The longest chain of edges ending at this card.
    ///
    /// `budget` bounds the recursion at the number of edges. A path longer than that has reused
    /// one, which means a cycle — and a cycle is not a topology. Stopping is how this function
    /// stays total; `the_canonical_graph_is_acyclic` is what keeps the case from arising.
    fn longest_path_into(card: CardId, budget: usize) -> usize {
        if budget == 0 {
            return 0;
        }
        Self::canonical()
            .iter()
            .filter(|edge| edge.to == card)
            .map(|edge| 1 + Self::longest_path_into(edge.from, budget - 1))
            .max()
            .unwrap_or(0)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every card the canonical graph mentions, on either end of an edge.
    fn mentioned() -> Vec<CardId> {
        let mut seen: Vec<CardId> = Vec::new();
        for edge in DesktopRelationshipGraph::canonical() {
            for card in [edge.from, edge.to] {
                if !seen.contains(&card) {
                    seen.push(card);
                }
            }
        }
        seen
    }

    #[test]
    fn the_canonical_graph_is_acyclic() {
        // The layer computation is bounded by the edge count so it terminates whatever the graph
        // says. This is what keeps the bound from ever being what stops it: a cycle would make
        // "how far from the start" meaningless, and the arrangement would be showing an order that
        // does not exist.
        let edges = DesktopRelationshipGraph::canonical();
        let mut remaining: Vec<&Relationship> = edges.iter().collect();
        let mut settled: Vec<CardId> = Vec::new();

        loop {
            let ready: Vec<CardId> = mentioned()
                .into_iter()
                .filter(|card| !settled.contains(card))
                .filter(|card| !remaining.iter().any(|edge| edge.to == *card))
                .collect();
            if ready.is_empty() {
                break;
            }
            for card in &ready {
                remaining.retain(|edge| edge.from != *card);
            }
            settled.extend(ready);
        }

        assert!(
            remaining.is_empty(),
            "the graph has a cycle; {} edges never became reachable",
            remaining.len()
        );
    }

    #[test]
    fn every_edge_points_at_least_one_layer_forward() {
        // This is the property the arrangement reads. An edge inside a layer draws a line between
        // two cards in the same column, which shows a dependency as though it were none; an edge
        // pointing backwards draws the causality reversed.
        for edge in DesktopRelationshipGraph::canonical() {
            let from = DesktopRelationshipGraph::layer_for_card(edge.from);
            let to = DesktopRelationshipGraph::layer_for_card(edge.to);
            assert!(
                to > from,
                "{} -> {} runs from layer {from} to layer {to}",
                edge.from.title(),
                edge.to.title()
            );
        }
    }

    #[test]
    fn an_organ_nothing_feeds_starts_at_the_beginning() {
        for card in mentioned() {
            let fed = DesktopRelationshipGraph::canonical()
                .iter()
                .any(|edge| edge.to == card);
            if !fed {
                assert_eq!(
                    DesktopRelationshipGraph::layer_for_card(card),
                    0,
                    "{} is fed by nothing and is not in the first layer",
                    card.title()
                );
            }
        }
    }

    #[test]
    fn perception_feeds_beliefs_and_not_the_other_way() {
        // `perceptiond` observes the host and submits to Event1; `epistemicd` forms beliefs keyed
        // by the subject of each observation. Nothing flows back. The edge ran the other way until
        // 2026-08-22, contradicting its own documentation.
        let edges = DesktopRelationshipGraph::canonical();
        assert!(
            edges
                .iter()
                .any(|edge| edge.from == CardId::Perception && edge.to == CardId::Beliefs)
        );
        assert!(
            !edges
                .iter()
                .any(|edge| edge.from == CardId::Beliefs && edge.to == CardId::Perception)
        );
        assert!(
            DesktopRelationshipGraph::layer_for_card(CardId::Beliefs)
                > DesktopRelationshipGraph::layer_for_card(CardId::Perception)
        );
    }

    #[test]
    fn what_leaves_is_placed_after_everything_that_produced_it() {
        let disclosure = DesktopRelationshipGraph::layer_for_card(CardId::Disclosure);
        for organ in [CardId::Commitments, CardId::Beliefs, CardId::Context] {
            assert!(
                disclosure > DesktopRelationshipGraph::layer_for_card(organ),
                "{} is not placed before Disclosure",
                organ.title()
            );
        }
    }

    #[test]
    fn a_tool_card_sits_after_every_organ() {
        // Tool cards consume what the organs produce and feed none of it back, so the graph does
        // not mention them and the layout puts them last rather than first.
        let tool = DesktopRelationshipGraph::layer_for_card(CardId::Shell(0));
        for organ in CardId::ALL_SYSTEM_CARDS {
            assert!(
                tool >= DesktopRelationshipGraph::layer_for_card(organ),
                "{} is placed after a tool card",
                organ.title()
            );
        }
        assert_eq!(tool, DesktopRelationshipGraph::layer_count() - 1);
    }

    #[test]
    fn the_layer_count_leaves_room_for_every_layer_it_reports() {
        let layers = DesktopRelationshipGraph::layer_count();
        for card in CardId::ALL_SYSTEM_CARDS {
            assert!(DesktopRelationshipGraph::layer_for_card(card) < layers);
        }
        assert!(DesktopRelationshipGraph::layer_for_card(CardId::Shell(0)) < layers);
    }
}
