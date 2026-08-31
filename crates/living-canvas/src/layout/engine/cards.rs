// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What a single card is, and what can be done to one.
//!
//! Everything here reads or moves one card. Grouping, arrangement and persistence live beside this
//! module because they are about the desktop rather than about a card, and keeping them apart is
//! what makes each of these methods short enough to read at once.

use crate::card::{CardGeometry, CardId, CardInstance, CardPresentation};
use crate::layout::model::DesktopItemId;

use super::DesktopLayout;

impl DesktopLayout {
    /// Look up a card instance by ID.
    #[must_use]
    pub fn card(&self, id: CardId) -> Option<&CardInstance> {
        self.cards.iter().find(|c| c.id == id)
    }

    /// Look up a mutable card instance by ID.
    pub fn card_mut(&mut self, id: CardId) -> Option<&mut CardInstance> {
        self.cards.iter_mut().find(|c| c.id == id)
    }

    /// Get geometry for a given card ID.
    #[must_use]
    pub fn geometry(&self, id: CardId) -> CardGeometry {
        self.card(id).map_or_else(
            || CardGeometry::new(50.0, 50.0, id.spec().default_size, 1),
            |c| c.geometry,
        )
    }

    /// Get presentation state for a given card ID.
    #[must_use]
    pub fn presentation(&self, id: CardId) -> CardPresentation {
        self.card(id)
            .map_or_else(CardPresentation::default, |c| c.presentation)
    }

    /// Update coordinates of a card.
    pub fn set_position(&mut self, id: CardId, x: f64, y: f64) {
        if let Some(card) = self.card_mut(id) {
            card.geometry.x = x;
            card.geometry.y = y;
        }
    }

    /// Update size of a card, respecting min and max boundaries.
    pub fn set_size(&mut self, id: CardId, width: f64, height: f64) {
        let spec = id.spec();
        if let Some(card) = self.card_mut(id) {
            card.geometry.width = width.clamp(spec.min_size.0, spec.max_size.0);
            card.geometry.height = height.clamp(spec.min_size.1, spec.max_size.1);
        }
    }

    /// Toggle or set collapsed state for a card.
    pub fn set_collapsed(&mut self, id: CardId, collapsed: bool) {
        if let Some(card) = self.card_mut(id) {
            card.presentation.collapsed = collapsed;
        }
    }

    /// Toggle or set pinned state for a card.
    pub fn set_pinned(&mut self, id: CardId, pinned: bool) {
        if let Some(card) = self.card_mut(id) {
            card.presentation.pinned = pinned;
        }
    }

    /// Set panel representation tier (Standard, Glance, Expanded) for a card.
    pub fn set_representation(
        &mut self,
        id: CardId,
        representation: crate::card::PanelRepresentation,
    ) {
        if let Some(card) = self.card_mut(id) {
            card.presentation.representation = representation;
        }
    }

    /// Bring any desktop item (Card or Deck) forward in stacking order (Invariant L14).
    pub fn bring_item_forward(&mut self, item_id: &DesktopItemId) {
        let max_z = self
            .cards
            .iter()
            .map(|c| c.geometry.z)
            .chain(self.decks.iter().map(|d| d.geometry.z))
            .max()
            .unwrap_or(0);

        let new_z = max_z + 1;
        match item_id {
            DesktopItemId::Card(id) => {
                if let Some(c) = self.card_mut(*id) {
                    c.geometry.z = new_z;
                }
            }
            DesktopItemId::Deck(id) => {
                if let Some(d) = self.deck_mut(id) {
                    d.geometry.z = new_z;
                }
            }
        }
    }

    /// Bring a card to the front by setting its z-index above all others.
    pub fn bring_forward(&mut self, id: CardId) {
        self.bring_item_forward(&DesktopItemId::Card(id));
    }

    /// Bring a deck to the front by setting its z-index above all others.
    pub fn bring_deck_forward(&mut self, id: &str) {
        self.bring_item_forward(&DesktopItemId::Deck(id.to_string()));
    }

    /// Check if a card is currently present in the layout.
    #[must_use]
    pub fn contains_card(&self, id: CardId) -> bool {
        self.cards.iter().any(|c| c.id == id)
    }

    /// Open or focus a dynamic card instance.
    pub fn open_card(&mut self, id: CardId, x: f64, y: f64) {
        if self.contains_card(id) {
            self.bring_forward(id);
            return;
        }
        // Opening it is the answer to having closed it.
        self.closed.retain(|closed| *closed != id);
        let spec = id.spec();
        let max_z = self.cards.iter().map(|c| c.geometry.z).max().unwrap_or(0);
        self.cards.push(CardInstance {
            id,
            geometry: CardGeometry::new(x, y, spec.default_size, max_z + 1),
            presentation: CardPresentation::default(),
        });
    }

    /// Close and remove a card from the layout if closable.
    pub fn close_card(&mut self, id: CardId) {
        if id.spec().closable {
            // Remembered, so that the next load knows this was a decision rather than a gap.
            if CardId::ALL_SYSTEM_CARDS.contains(&id) && !self.closed.contains(&id) {
                self.closed.push(id);
            }
            self.cards.retain(|c| c.id != id);
            for deck in &mut self.decks {
                deck.remove_card(id);
            }
            self.decks.retain(|d| !d.is_empty());
        }
    }
}
