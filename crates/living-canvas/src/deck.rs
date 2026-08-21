// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Deck presentation composition model for CYBOU Desktop.
//!
//! A Deck is a presentation grouping (tabbed interface) combining multiple cards
//! into a single spatial bounding box without destroying or altering the underlying
//! card identities or state.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::card::{CardGeometry, CardId, CardPresentation};

/// Errors encountered when manipulating Deck structures or violating deck invariants.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum DeckError {
    /// Attempted to create a deck with fewer than two unique cards.
    #[error("deck must contain at least 2 distinct cards")]
    InsufficientCards,
    /// Card is not marked as deckable in its specification.
    #[error("card {0:?} is not deckable")]
    NonDeckableCard(CardId),
    /// Card already belongs to this or another deck.
    #[error("card {0:?} is already in a deck")]
    CardAlreadyInDeck(CardId),
    /// Card does not belong to the target deck.
    #[error("card {0:?} is not in deck {1}")]
    CardNotInDeck(CardId, String),
    /// Targeted deck ID was not found in layout.
    #[error("deck with ID '{0}' not found")]
    DeckNotFound(String),
}

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
    /// Construct a verified Deck instance satisfying all deck invariants.
    ///
    /// # Errors
    ///
    /// Returns `DeckError::NonDeckableCard` if any card cannot be docked, or
    /// `DeckError::InsufficientCards` if fewer than two unique cards were provided.
    pub fn try_new(
        id: impl Into<String>,
        title: impl Into<String>,
        cards: Vec<CardId>,
        x: f64,
        y: f64,
    ) -> Result<Self, DeckError> {
        let mut unique_cards = Vec::new();
        for c in cards {
            if !c.spec().deckable {
                return Err(DeckError::NonDeckableCard(c));
            }
            if !unique_cards.contains(&c) {
                unique_cards.push(c);
            }
        }

        if unique_cards.len() < 2 {
            return Err(DeckError::InsufficientCards);
        }

        let active = unique_cards[0];
        let id_str = id.into();
        let title_str = title.into();
        Ok(Self {
            id: id_str,
            title: title_str,
            card_ids: unique_cards,
            active_card: active,
            geometry: CardGeometry::new(x, y, (420.0, 480.0), 10),
            presentation: CardPresentation::default(),
        })
    }

    /// Create a new Deck containing the specified cards (fallback with sanitization).
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        cards: Vec<CardId>,
        x: f64,
        y: f64,
    ) -> Self {
        Self::try_new(id, title, cards, x, y).unwrap_or_else(|_| {
            let active = CardId::Identity;
            Self {
                id: format!("deck-{}", uuid::Uuid::new_v4()),
                title: "Deck".into(),
                card_ids: vec![CardId::Identity, CardId::Session],
                active_card: active,
                geometry: CardGeometry::new(x, y, (420.0, 480.0), 10),
                presentation: CardPresentation::default(),
            }
        })
    }

    /// Add a card to this deck if deckable and not already present.
    ///
    /// # Errors
    ///
    /// Returns `DeckError::NonDeckableCard` if the card cannot be docked, or
    /// `DeckError::CardAlreadyInDeck` if the card is already a member.
    pub fn add_card(&mut self, card: CardId) -> Result<(), DeckError> {
        if !card.spec().deckable {
            return Err(DeckError::NonDeckableCard(card));
        }
        if self.card_ids.contains(&card) {
            return Err(DeckError::CardAlreadyInDeck(card));
        }
        self.card_ids.push(card);
        Ok(())
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

    /// Validate deck invariants: active card belongs to deck, at least 2 cards, all deckable.
    pub fn validate_and_normalize(&mut self) -> bool {
        self.card_ids.retain(|c| c.spec().deckable);
        // Deduplicate
        let mut seen = std::collections::HashSet::new();
        self.card_ids.retain(|c| seen.insert(*c));

        if self.card_ids.len() < 2 {
            return false;
        }

        if !self.card_ids.contains(&self.active_card) {
            self.active_card = self.card_ids[0];
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_lifecycle_and_tab_switching() {
        let mut deck = DeckInstance::try_new(
            "deck-1",
            "Cognition Deck",
            vec![CardId::Beliefs, CardId::Context],
            100.0,
            100.0,
        )
        .expect("valid deck creation");

        assert_eq!(deck.len(), 2);
        assert_eq!(deck.active_card, CardId::Beliefs);
        assert!(deck.contains(CardId::Context));

        deck.set_active(CardId::Context);
        assert_eq!(deck.active_card, CardId::Context);

        deck.add_card(CardId::Commitments).expect("add commitments");
        assert_eq!(deck.len(), 3);

        assert!(deck.remove_card(CardId::Context));
        assert_eq!(deck.len(), 2);
        assert_eq!(deck.active_card, CardId::Beliefs);
    }

    #[test]
    fn deck_rejects_single_card_or_duplicates() {
        let single =
            DeckInstance::try_new("deck-single", "Single", vec![CardId::Identity], 0.0, 0.0);
        assert_eq!(single, Err(DeckError::InsufficientCards));

        let dups = DeckInstance::try_new(
            "deck-dups",
            "Dups",
            vec![CardId::Identity, CardId::Identity],
            0.0,
            0.0,
        );
        assert_eq!(dups, Err(DeckError::InsufficientCards));
    }
}
