// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What a single card is, and what can be done to one.
//!
//! Everything here reads or moves one card. Grouping, arrangement and persistence live beside this
//! module because they are about the desktop rather than about a card, and keeping them apart is
//! what makes each of these methods short enough to read at once.

use crate::card::{CardGeometry, CardId, CardInstance, CardPresentation};
use crate::layout::model::{DesktopItem, DesktopItemId, Rect};

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

    /// Somewhere inside `view` that no open item covers, for a card of `size`.
    ///
    /// `view` is in canvas coordinates: what the window currently shows, which is the only part of
    /// an unbounded plane a person can be surprised by. A card placed outside it is a card that
    /// opened somewhere the person is not looking, which is the same as not opening.
    ///
    /// Searched in reading order, coarse steps, first fit. Not a packing algorithm: the desktop is
    /// arranged by a person and this only has to answer "where is there room right now" fast enough
    /// to run on a click, for a canvas holding tens of cards rather than thousands.
    #[must_use]
    pub fn free_spot_in(&self, size: (f64, f64), view: Rect) -> (f64, f64) {
        const STEP: f64 = 32.0;
        const MARGIN: f64 = 24.0;

        let taken: Vec<Rect> = self
            .desktop_items()
            .iter()
            .map(DesktopItem::effective_rect)
            .collect();

        let left = view.x + MARGIN;
        let top = view.y + MARGIN;
        let right = view.x + view.width - MARGIN;
        let bottom = view.y + view.height - MARGIN;

        let mut y = top;
        while y + size.1 <= bottom {
            let mut x = left;
            while x + size.0 <= right {
                let candidate = Rect::new(x, y, size.0, size.1);
                if !taken.iter().any(|item| candidate.intersects(item)) {
                    return (x, y);
                }
                x += STEP;
            }
            y += STEP;
        }

        // Nothing is free: cascade from the top-left of the view by however many cards are already
        // out, so a fourth card does not land exactly under the third. Overlapping deliberately,
        // in view, beats a tidy coordinate nobody can see.
        let overlap = (taken.len() % 8) as f64 * STEP;
        (left + overlap, top + overlap)
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

#[cfg(test)]
mod free_spot_tests {
    use crate::card::CardId;
    use crate::layout::engine::DesktopLayout;
    use crate::layout::model::Rect;

    #[test]
    fn a_card_opens_where_nothing_else_is() {
        // Every card used to open at a coordinate written into the call site, so the third one
        // opened under the second and the fourth under both.
        let mut layout = DesktopLayout::new();
        let view = Rect::new(0.0, 0.0, 1440.0, 800.0);
        let size = (380.0, 300.0);

        let first = layout.free_spot_in(size, view);
        layout.open_card(CardId::Services(0), first.0, first.1);
        let second = layout.free_spot_in(size, view);

        assert_ne!(first, second, "the second card opened on top of the first");
        let a = Rect::new(first.0, first.1, size.0, size.1);
        let b = Rect::new(second.0, second.1, size.0, size.1);
        assert!(!a.intersects(&b), "the two cards overlap");
    }

    #[test]
    fn a_spot_is_inside_the_part_of_the_canvas_being_looked_at() {
        // The view travels with the camera. A card opened at the canvas origin while somebody is
        // three thousand pixels away has not opened as far as they are concerned.
        let layout = DesktopLayout::new();
        let view = Rect::new(3000.0, 1200.0, 1440.0, 800.0);
        let (x, y) = layout.free_spot_in((380.0, 300.0), view);

        assert!(
            x >= view.x && x + 380.0 <= view.x + view.width,
            "x {x} is outside the view"
        );
        assert!(
            y >= view.y && y + 300.0 <= view.y + view.height,
            "y {y} is outside the view"
        );
    }

    #[test]
    fn a_full_view_still_answers_with_somewhere_visible() {
        // When there is genuinely no room, overlapping deliberately and in view beats a tidy
        // coordinate nobody is looking at. What it must never do is fail to answer.
        let mut layout = DesktopLayout::new();
        let view = Rect::new(0.0, 0.0, 600.0, 400.0);
        layout.open_card(CardId::Services(0), 0.0, 0.0);
        layout.open_card(CardId::Processes(0), 0.0, 0.0);

        let (x, y) = layout.free_spot_in((560.0, 360.0), view);
        assert!(x >= view.x && y >= view.y);
        assert!(x < view.x + view.width && y < view.y + view.height);
    }
}
