// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Decks: cards grouped into one tabbed surface, and the rules that keep the grouping honest.
//!
//! A deck owns the cards docked in it (Invariant L8), so every operation here has to leave both
//! sides of that ownership true — a card in two places, or in none, is the failure these methods
//! exist to prevent.

use crate::card::CardId;
use crate::deck::{DeckError, DeckInstance};
use crate::layout::{
    model::{DesktopItemId, Rect, UsableViewport},
    placement::PlacementResolver,
};

use super::DesktopLayout;

impl DesktopLayout {
    /// Create a new deck grouping given cards and position it on desktop.
    ///
    /// # Errors
    ///
    /// Returns `DeckError` if cards cannot be grouped into a valid deck.
    pub fn create_deck(
        &mut self,
        title: impl Into<String>,
        cards: Vec<CardId>,
        x: f64,
        y: f64,
    ) -> Result<String, DeckError> {
        let id = format!("deck-{}", uuid::Uuid::new_v4());

        let mut min_w: f64 = 340.0;
        let mut min_h: f64 = 240.0;
        for c in &cards {
            let spec = c.spec();
            min_w = min_w.max(spec.min_size.0);
            min_h = min_h.max(spec.min_size.1 + 36.0);
        }

        let mut deck = DeckInstance::try_new(&id, title, cards, x, y)?;
        deck.geometry.width = deck.geometry.width.max(min_w);
        deck.geometry.height = deck.geometry.height.max(min_h);
        self.decks.push(deck);
        self.bring_item_forward(&DesktopItemId::Deck(id.clone()));
        Ok(id)
    }

    /// Add a card into an existing deck.
    ///
    /// # Errors
    ///
    /// Returns `DeckError::CardAlreadyInDeck` if card is already docked, or
    /// `DeckError::DeckNotFound` if the deck ID does not exist in layout.
    pub fn add_to_deck(&mut self, deck_id: &str, card: CardId) -> Result<(), DeckError> {
        if self.is_in_deck(card) {
            return Err(DeckError::CardAlreadyInDeck(card));
        }
        if let Some(deck) = self.decks.iter_mut().find(|d| d.id == deck_id) {
            deck.add_card(card)
        } else {
            Err(DeckError::DeckNotFound(deck_id.to_string()))
        }
    }

    /// Detach a card from a deck, safely positioning it adjacent using `PlacementResolver` (Invariant L13).
    pub fn detach_from_deck(
        &mut self,
        deck_id: &str,
        card: CardId,
        viewport: Option<UsableViewport>,
    ) {
        let mut should_dissolve = false;
        let mut deck_geom = None;

        if let Some(deck) = self.deck_mut(deck_id) {
            deck_geom = Some(deck.geometry);
            deck.remove_card(card);
            if deck.len() <= 1 {
                should_dissolve = true;
            }
        }

        if let Some(geom) = deck_geom {
            let spec = card.spec();
            let vp = viewport.unwrap_or_default();
            let items = self.desktop_items();
            let pref_rect = Rect::new(geom.x, geom.y, geom.width, geom.height);
            let (det_x, det_y) = PlacementResolver::find_placement(
                &items,
                spec.default_size.0,
                spec.default_size.1,
                Some(pref_rect),
                vp,
            );
            self.set_position(card, det_x, det_y);
        }

        self.bring_forward(card);

        if should_dissolve {
            self.dissolve_deck(deck_id);
        }
    }

    /// Dissolve a deck and restore its cards as independent spatial cards.
    pub fn dissolve_deck(&mut self, deck_id: &str) {
        if let Some(pos) = self.decks.iter().position(|d| d.id == deck_id) {
            let deck = self.decks.remove(pos);
            let mut offset = 0.0;
            for c in deck.card_ids {
                self.set_position(c, deck.geometry.x + offset, deck.geometry.y + offset);
                self.bring_forward(c);
                offset += 40.0;
            }
        }
    }

    /// Find deck containing a given card, if any.
    #[must_use]
    pub fn deck_for_card(&self, card: CardId) -> Option<&DeckInstance> {
        self.decks.iter().find(|d| d.contains(card))
    }

    /// Find mutable deck containing a given card, if any.
    pub fn deck_for_card_mut(&mut self, card: CardId) -> Option<&mut DeckInstance> {
        self.decks.iter_mut().find(|d| d.contains(card))
    }

    /// Check if a card is currently docked in any deck.
    #[must_use]
    pub fn is_in_deck(&self, card: CardId) -> bool {
        self.decks.iter().any(|d| d.contains(card))
    }

    /// Get deck by ID.
    #[must_use]
    pub fn deck(&self, id: &str) -> Option<&DeckInstance> {
        self.decks.iter().find(|d| d.id == id)
    }

    /// Get mutable deck by ID.
    pub fn deck_mut(&mut self, id: &str) -> Option<&mut DeckInstance> {
        self.decks.iter_mut().find(|d| d.id == id)
    }

    /// Update position for a deck.
    pub fn set_deck_position(&mut self, id: &str, x: f64, y: f64) {
        if let Some(deck) = self.deck_mut(id) {
            deck.geometry.x = x.clamp(0.0, 10000.0);
            deck.geometry.y = y.clamp(0.0, 10000.0);
        }
    }

    /// Update size for a deck.
    pub fn set_deck_size(&mut self, id: &str, width: f64, height: f64) {
        if let Some(deck) = self.deck_mut(id) {
            deck.geometry.width = width.clamp(280.0, 2000.0);
            deck.geometry.height = height.clamp(160.0, 1600.0);
        }
    }

    /// Toggle collapsed state for a deck.
    pub fn toggle_deck_collapse(&mut self, id: &str) {
        if let Some(deck) = self.deck_mut(id) {
            deck.presentation.collapsed = !deck.presentation.collapsed;
        }
    }

    /// Toggle pinned state for a deck.
    pub fn toggle_deck_pinned(&mut self, id: &str) {
        if let Some(deck) = self.deck_mut(id) {
            deck.presentation.pinned = !deck.presentation.pinned;
        }
    }
}
