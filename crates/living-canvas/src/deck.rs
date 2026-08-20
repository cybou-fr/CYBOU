// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Deck presentation composition model for CYBOU Desktop.
//!
//! A Deck is a presentation grouping (tabbed interface) combining multiple cards
//! into a single spatial bounding box without destroying or altering the underlying
//! card identities or state.

use serde::{Deserialize, Serialize};

use crate::card::{CardGeometry, CardId, CardPresentation};

/// Persistent Deck instance grouping multiple cards into one spatial container.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct DeckInstance {
    /// Unique identifier for this deck instance.
    pub id: String,
    /// User-visible title (or composite title of cards).
    pub title: String,
    /// Ordered list of card IDs contained in this deck.
    pub card_ids: Vec<CardId>,
    /// Currently active/visible card ID in the tab bar.
    pub active_card: CardId,
    /// Deck bounding geometry on the canvas.
    pub geometry: CardGeometry,
    /// Deck presentation flags (collapsed, pinned).
    pub presentation: CardPresentation,
}

impl DeckInstance {
    /// Create a new Deck containing the specified cards.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        cards: Vec<CardId>,
        x: f64,
        y: f64,
    ) -> Self {
        let active = cards.first().copied().unwrap_or(CardId::Identity);
        let id_str = id.into();
        let title_str = title.into();
        Self {
            id: id_str,
            title: title_str,
            card_ids: cards,
            active_card: active,
            geometry: CardGeometry::new(x, y, (420.0, 480.0), 10),
            presentation: CardPresentation::default(),
        }
    }

    /// Add a card to this deck if not already present.
    pub fn add_card(&mut self, card: CardId) {
        if !self.card_ids.contains(&card) {
            self.card_ids.push(card);
        }
    }

    /// Remove a card from this deck, returning true if removed.
    pub fn remove_card(&mut self, card: CardId) -> bool {
        if let Some(pos) = self.card_ids.iter().position(|&c| c == card) {
            self.card_ids.remove(pos);
            if self.active_card == card {
                self.active_card = self.card_ids.first().copied().unwrap_or(CardId::Identity);
            }
            true
        } else {
            false
        }
    }

    /// Set active card in this deck if it belongs to it.
    pub fn set_active(&mut self, card: CardId) {
        if self.card_ids.contains(&card) {
            self.active_card = card;
        }
    }

    /// Check if this deck contains a specific card.
    #[must_use]
    pub fn contains(&self, card: CardId) -> bool {
        self.card_ids.contains(&card)
    }

    /// Number of cards in this deck.
    #[must_use]
    pub fn len(&self) -> usize {
        self.card_ids.len()
    }

    /// Whether this deck is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.card_ids.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_lifecycle_and_tab_switching() {
        let mut deck = DeckInstance::new(
            "deck-1",
            "Cognition Deck",
            vec![CardId::Beliefs, CardId::Context],
            100.0,
            100.0,
        );

        assert_eq!(deck.len(), 2);
        assert_eq!(deck.active_card, CardId::Beliefs);
        assert!(deck.contains(CardId::Context));

        deck.set_active(CardId::Context);
        assert_eq!(deck.active_card, CardId::Context);

        deck.add_card(CardId::Commitments);
        assert_eq!(deck.len(), 3);

        assert!(deck.remove_card(CardId::Context));
        assert_eq!(deck.len(), 2);
        assert_eq!(deck.active_card, CardId::Beliefs);
    }
}
