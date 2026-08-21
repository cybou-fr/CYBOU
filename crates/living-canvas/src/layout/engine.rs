// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Core Spatial Layout Engine v2 managing top-level desktop surfaces, persistence, and arrangements.

use serde::{Deserialize, Serialize};

use crate::card::{CardGeometry, CardId, CardInstance, CardPresentation};
use crate::deck::{DeckError, DeckInstance};
use crate::layout::{
    migration::{CanvasLayoutV8, from_v8},
    model::{ArrangementMode, DesktopItem, DesktopItemId, Rect, UsableViewport},
    placement::PlacementResolver,
    snap::{SnapResult, compute_snap},
};

#[cfg(target_arch = "wasm32")]
use crate::layout::migration::{LAYOUT_KEY_V8, LAYOUT_KEY_V9};

/// Persistent Desktop layout (schema version 9).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct DesktopLayout {
    /// Schema version, currently 9.
    pub schema_version: u32,
    /// Ordered list of card instances.
    pub cards: Vec<CardInstance>,
    /// Optional decks grouping cards into tabbed presentation containers.
    #[serde(default)]
    pub decks: Vec<DeckInstance>,
}

impl Default for DesktopLayout {
    fn default() -> Self {
        from_v8(&CanvasLayoutV8::default())
    }
}

impl DesktopLayout {
    /// Construct a new empty layout with schema version 9.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: 9,
            cards: Vec::new(),
            decks: Vec::new(),
        }
    }

    /// Return all active top-level spatial items on the desktop:
    /// standalone cards (cards NOT in any deck) plus active decks.
    /// Cards docked inside decks are excluded (Invariant L8).
    #[must_use]
    pub fn desktop_items(&self) -> Vec<DesktopItem> {
        let mut items = Vec::new();
        for card in &self.cards {
            if !self.is_in_deck(card.id) {
                items.push(DesktopItem {
                    id: DesktopItemId::Card(card.id),
                    geometry: card.geometry,
                    presentation: card.presentation,
                });
            }
        }
        for deck in &self.decks {
            items.push(DesktopItem {
                id: DesktopItemId::Deck(deck.id.clone()),
                geometry: deck.geometry,
                presentation: deck.presentation,
            });
        }
        items
    }

    /// Compute snap alignment against other desktop items during drag/resize operations.
    #[must_use]
    pub fn compute_snap(
        &self,
        dragged_id: &DesktopItemId,
        candidate_x: f64,
        candidate_y: f64,
        width: f64,
        height: f64,
        snap_threshold: f64,
    ) -> SnapResult {
        compute_snap(
            &self.desktop_items(),
            dragged_id,
            candidate_x,
            candidate_y,
            width,
            height,
            snap_threshold,
        )
    }

    /// Compute the overall bounding rectangle enclosing all standalone cards and decks.
    #[must_use]
    pub fn bounding_rect(&self) -> Option<Rect> {
        let items = self.desktop_items();
        if items.is_empty() {
            return None;
        }
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for item in items {
            let r = item.effective_rect();
            min_x = min_x.min(r.x);
            min_y = min_y.min(r.y);
            max_x = max_x.max(r.right());
            max_y = max_y.max(r.bottom());
        }

        Some(Rect::new(
            min_x,
            min_y,
            (max_x - min_x).max(10.0),
            (max_y - min_y).max(10.0),
        ))
    }

    /// Compute zoom and pan parameters to fit a bounding rectangle within viewport dimensions.
    #[must_use]
    pub fn fit_to_viewport(
        bounding: Rect,
        viewport_w: f64,
        viewport_h: f64,
        padding: f64,
    ) -> (f64, (f64, f64)) {
        let avail_w = (viewport_w - padding * 2.0).max(100.0);
        let avail_h = (viewport_h - padding * 2.0).max(100.0);

        let zoom = (avail_w / bounding.width)
            .min(avail_h / bounding.height)
            .clamp(0.4, 1.2);

        let pan_x = (viewport_w - bounding.width * zoom) / 2.0 - bounding.x * zoom;
        let pan_y = (viewport_h - bounding.height * zoom) / 2.0 - bounding.y * zoom;

        (zoom, (pan_x, pan_y))
    }

    /// Migrate a legacy v8 layout into v9 format.
    #[must_use]
    pub fn from_v8(v8: &CanvasLayoutV8) -> Self {
        from_v8(v8)
    }

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
            self.cards.retain(|c| c.id != id);
            for deck in &mut self.decks {
                deck.remove_card(id);
            }
            self.decks.retain(|d| !d.is_empty());
        }
    }

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

    /// Validate all desktop layout invariants and normalize state:
    /// 1. Ensures all 11 Mind organ system cards are present (instantiating defaults if missing).
    /// 2. Clamps all card and deck dimensions to spec min/max bounds and reachable positions.
    /// 3. Normalizes z-order monotonically to prevent gaps and overflow (Invariant L14).
    /// 4. Validates decks: dissolves invalid/empty/<2 card decks, removes duplicate cards, ensures `active_card` is in deck (Invariants L1–L4).
    /// 5. Ensures no card is in multiple decks simultaneously (Invariant L1).
    pub fn validate_and_normalize(&mut self) {
        // 1. Ensure all system cards exist
        for sys_id in CardId::ALL_SYSTEM_CARDS {
            if !self.cards.iter().any(|c| c.id == sys_id) {
                let spec = sys_id.spec();
                let max_z = self.cards.iter().map(|c| c.geometry.z).max().unwrap_or(0);
                self.cards.push(CardInstance {
                    id: sys_id,
                    geometry: CardGeometry::new(60.0, 60.0, spec.default_size, max_z + 1),
                    presentation: CardPresentation::default(),
                });
            }
        }

        // 2. Clamp and normalize all card geometries
        for card in &mut self.cards {
            let spec = card.id.spec();
            card.geometry.width = card.geometry.width.clamp(spec.min_size.0, spec.max_size.0);
            card.geometry.height = card.geometry.height.clamp(spec.min_size.1, spec.max_size.1);
            card.geometry.x = card.geometry.x.clamp(0.0, 10000.0);
            card.geometry.y = card.geometry.y.clamp(0.0, 10000.0);
        }

        // 3. Validate decks and resolve multi-deck card conflicts
        let mut assigned_cards = std::collections::HashSet::new();
        let mut valid_decks = Vec::new();

        for mut deck in self.decks.drain(..) {
            // Remove cards already claimed by another deck
            deck.card_ids.retain(|c| assigned_cards.insert(*c));

            if deck.validate_and_normalize() {
                deck.geometry.x = deck.geometry.x.clamp(0.0, 10000.0);
                deck.geometry.y = deck.geometry.y.clamp(0.0, 10000.0);
                deck.geometry.width = deck.geometry.width.clamp(280.0, 2000.0);
                deck.geometry.height = deck.geometry.height.clamp(160.0, 1600.0);
                valid_decks.push(deck);
            }
        }
        self.decks = valid_decks;

        // 4. Normalize z-indices monotonically starting at 1
        self.normalize_z_indices();
    }

    /// Normalize all z-indices monotonically starting from 1 (Invariant L14).
    pub fn normalize_z_indices(&mut self) {
        let mut z_items: Vec<(u32, bool, usize)> = Vec::new();
        for (i, c) in self.cards.iter().enumerate() {
            z_items.push((c.geometry.z, false, i));
        }
        for (i, d) in self.decks.iter().enumerate() {
            z_items.push((d.geometry.z, true, i));
        }
        z_items.sort_by_key(|item| item.0);

        for (new_z, (_, is_deck, idx)) in z_items.into_iter().enumerate() {
            let assigned_z = u32::try_from(new_z + 1).unwrap_or(u32::MAX);
            if is_deck {
                self.decks[idx].geometry.z = assigned_z;
            } else {
                self.cards[idx].geometry.z = assigned_z;
            }
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

    /// Parse layout from raw JSON string, supporting both v9 and v8 formats.
    #[must_use]
    pub fn parse_json(json: &str) -> Option<Self> {
        if let Ok(v9) = serde_json::from_str::<Self>(json)
            && v9.schema_version == 9
        {
            return Some(v9);
        }
        if let Ok(v8) = serde_json::from_str::<CanvasLayoutV8>(json) {
            return Some(Self::from_v8(&v8));
        }
        None
    }

    /// Apply an automated spatial arrangement algorithm with active viewport awareness.
    ///
    /// Enforces Invariants:
    /// - L5: Arrangement never moves pinned items.
    /// - L6: Arrangement never overlaps unpinned items with pinned obstacles.
    /// - L7: Arrangement never overlaps output items.
    /// - L8: Cards inside decks are excluded from arrangement.
    /// - L11: Automatic arrangement is deterministic.
    pub fn apply_arrangement(&mut self, mode: ArrangementMode, viewport: Option<UsableViewport>) {
        let vp = viewport.unwrap_or_default();
        match mode {
            ArrangementMode::Free => {}
            ArrangementMode::Grid => self.arrange_grid(vp),
            ArrangementMode::Compact => self.arrange_compact(vp),
            ArrangementMode::Relations => self.arrange_relations(vp),
            ArrangementMode::Home => self.arrange_home(vp),
        }
    }

    /// Helper to apply item positioning to either `self.cards` or `self.decks`.
    fn update_item_position(&mut self, id: &DesktopItemId, x: f64, y: f64) {
        match id {
            DesktopItemId::Card(c_id) => {
                if let Some(card) = self.card_mut(*c_id) {
                    card.geometry.x = x;
                    card.geometry.y = y;
                }
            }
            DesktopItemId::Deck(d_id) => {
                if let Some(deck) = self.deck_mut(d_id) {
                    deck.geometry.x = x;
                    deck.geometry.y = y;
                }
            }
        }
    }

    /// Arrange top-level items in an adaptive multi-track Grid layout.
    #[allow(clippy::cast_precision_loss)]
    fn arrange_grid(&mut self, viewport: UsableViewport) {
        let items = self.desktop_items();
        let mut pinned_rects: Vec<Rect> = items
            .iter()
            .filter(|it| it.is_pinned())
            .map(DesktopItem::effective_rect)
            .collect();

        let track_width = 360.0;
        let col_gap = 20.0;
        let row_gap = 20.0;
        let start_x = 40.0;
        let start_y = 40.0;

        let available_w = (viewport.width - start_x * 2.0).max(800.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let num_tracks = (((available_w + col_gap) / (track_width + col_gap)).floor() as usize)
            .clamp(2, 6);

        let mut track_heights = vec![start_y; num_tracks];
        let mut placed_rects: Vec<Rect> = Vec::new();

        for item in items {
            if item.is_pinned() {
                continue;
            }

            let eff_w = item.geometry.width;
            let eff_h = item.effective_height();

            let span = if eff_w <= track_width + 40.0 {
                1
            } else if eff_w <= track_width * 2.0 + col_gap + 40.0 {
                2.min(num_tracks)
            } else {
                3.min(num_tracks)
            };

            // Find best starting track
            let mut best_track = 0;
            let mut min_height = f64::MAX;

            for t in 0..=(num_tracks - span) {
                let max_h = (t..t + span)
                    .map(|idx| track_heights[idx])
                    .fold(0.0_f64, f64::max);
                if max_h < min_height {
                    min_height = max_h;
                    best_track = t;
                }
            }

            let mut place_y = min_height;
            let place_x = start_x + (best_track as f64) * (track_width + col_gap);

            // Obstacle resolution: shift downwards until no collision with pinned or placed items
            loop {
                let candidate = Rect::new(place_x, place_y, eff_w, eff_h);
                let mut collides = false;

                for obst in pinned_rects.iter().chain(placed_rects.iter()) {
                    if candidate.intersects(obst) {
                        place_y = obst.bottom() + row_gap;
                        collides = true;
                        break;
                    }
                }

                if !collides {
                    break;
                }
            }

            self.update_item_position(&item.id, place_x, place_y);
            let final_rect = Rect::new(place_x, place_y, eff_w, eff_h);
            placed_rects.push(final_rect);

            for h in track_heights.iter_mut().skip(best_track).take(span) {
                *h = place_y + eff_h + row_gap;
            }
        }

        // Maintain pinned rects reference to satisfy compiler
        pinned_rects.clear();
    }

    /// Arrange top-level items using 2D Skyline / bin-packing (Compact mode).
    fn arrange_compact(&mut self, _viewport: UsableViewport) {
        let items = self.desktop_items();
        let pinned_rects: Vec<Rect> = items
            .iter()
            .filter(|it| it.is_pinned())
            .map(DesktopItem::effective_rect)
            .collect();

        let mut placed_rects: Vec<Rect> = pinned_rects.clone();
        let step_x = 20.0;
        let step_y = 20.0;
        let start_x = 30.0;
        let start_y = 30.0;

        for item in items {
            if item.is_pinned() {
                continue;
            }

            let eff_w = item.geometry.width;
            let eff_h = item.effective_height();

            // Search for lowest (x, y) with minimum Manhattan penalty
            let mut best_x = start_x;
            let mut best_y = start_y;
            let mut found = false;

            let mut y = start_y;
            while y < 3000.0 && !found {
                let mut x = start_x;
                while x < 1800.0 {
                    let candidate = Rect::new(x, y, eff_w, eff_h);
                    let collides = placed_rects.iter().any(|r| r.intersects(&candidate));
                    if !collides {
                        best_x = x;
                        best_y = y;
                        found = true;
                        break;
                    }
                    x += step_x;
                }
                y += step_y;
            }

            self.update_item_position(&item.id, best_x, best_y);
            placed_rects.push(Rect::new(best_x, best_y, eff_w, eff_h));
        }
    }

    /// Arrange items in a deterministic layered causal graph.
    #[allow(clippy::cast_precision_loss)]
    fn arrange_relations(&mut self, _viewport: UsableViewport) {
        let items = self.desktop_items();
        let pinned_rects: Vec<Rect> = items
            .iter()
            .filter(|it| it.is_pinned())
            .map(DesktopItem::effective_rect)
            .collect();

        let mut placed_rects: Vec<Rect> = pinned_rects;

        let col_width = 360.0;
        let col_gap = 30.0;
        let row_gap = 20.0;
        let start_x = 40.0;
        let start_y = 40.0;

        // Categorize into 5 causal layers
        let item_layer = |item: &DesktopItem| -> usize {
            match &item.id {
                DesktopItemId::Card(id) => match id {
                    CardId::Identity | CardId::Session | CardId::Perception | CardId::Lifecycle => {
                        0
                    }
                    CardId::Capabilities | CardId::Journal => 1,
                    CardId::Context | CardId::Beliefs => 2,
                    CardId::Commitments | CardId::Attention | CardId::SelfModel => 3,
                    _ => 4, // Tool cards
                },
                DesktopItemId::Deck(_) => 1,
            }
        };

        let mut layer_items: [Vec<DesktopItem>; 5] = [
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ];

        for item in items {
            if !item.is_pinned() {
                let l = item_layer(&item);
                layer_items[l].push(item);
            }
        }

        for (l_idx, layer) in layer_items.into_iter().enumerate() {
            let col_x = start_x + (l_idx as f64) * (col_width + col_gap);
            let mut cur_y = start_y;

            for item in layer {
                let eff_w = item.geometry.width;
                let eff_h = item.effective_height();

                loop {
                    let candidate = Rect::new(col_x, cur_y, eff_w, eff_h);
                    let mut collides = false;

                    for obst in &placed_rects {
                        if candidate.intersects(obst) {
                            cur_y = obst.bottom() + row_gap;
                            collides = true;
                            break;
                        }
                    }

                    if !collides {
                        break;
                    }
                }

                self.update_item_position(&item.id, col_x, cur_y);
                placed_rects.push(Rect::new(col_x, cur_y, eff_w, eff_h));
                cur_y += eff_h + row_gap;
            }
        }
    }

    /// Arrange items in canonical Home layout adapted to viewport.
    #[allow(clippy::cast_precision_loss)]
    fn arrange_home(&mut self, viewport: UsableViewport) {
        let items = self.desktop_items();
        let pinned_rects: Vec<Rect> = items
            .iter()
            .filter(|it| it.is_pinned())
            .map(DesktopItem::effective_rect)
            .collect();

        let mut placed_rects: Vec<Rect> = pinned_rects;

        let col_width = 380.0;
        let col_gap = 24.0;
        let row_gap = 20.0;
        let start_x = 40.0;
        let start_y = 40.0;

        let num_cols = if viewport.width >= 1600.0 {
            4
        } else if viewport.width >= 1200.0 {
            3
        } else {
            2
        };

        // Canonical Home columns
        let home_col = |item: &DesktopItem| -> usize {
            match &item.id {
                DesktopItemId::Card(id) => match id {
                    CardId::Session | CardId::Identity | CardId::Perception | CardId::Lifecycle => {
                        0
                    }
                    CardId::Capabilities | CardId::Journal | CardId::Attention => 1 % num_cols,
                    CardId::Commitments | CardId::Context | CardId::Beliefs | CardId::SelfModel => {
                        2 % num_cols
                    }
                    _ => num_cols - 1,
                },
                DesktopItemId::Deck(_) => 1 % num_cols,
            }
        };

        let mut cols: Vec<Vec<DesktopItem>> = vec![Vec::new(); num_cols];
        for item in items {
            if !item.is_pinned() {
                let c = home_col(&item);
                cols[c].push(item);
            }
        }

        for (c_idx, col) in cols.into_iter().enumerate() {
            let col_x = start_x + (c_idx as f64) * (col_width + col_gap);
            let mut cur_y = start_y;

            for item in col {
                let eff_w = item.geometry.width;
                let eff_h = item.effective_height();

                loop {
                    let candidate = Rect::new(col_x, cur_y, eff_w, eff_h);
                    let mut collides = false;

                    for obst in &placed_rects {
                        if candidate.intersects(obst) {
                            cur_y = obst.bottom() + row_gap;
                            collides = true;
                            break;
                        }
                    }

                    if !collides {
                        break;
                    }
                }

                self.update_item_position(&item.id, col_x, cur_y);
                placed_rects.push(Rect::new(col_x, cur_y, eff_w, eff_h));
                cur_y += eff_h + row_gap;
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl DesktopLayout {
    /// Load layout from browser `localStorage`, seamlessly migrating from v8 if necessary.
    #[must_use]
    pub fn load() -> Self {
        let storage = web_sys::window().and_then(|w| w.local_storage().ok().flatten());
        let Some(storage) = storage else {
            let mut def = Self::default();
            def.validate_and_normalize();
            return def;
        };

        // 1. Try v9 key first
        if let Ok(Some(v9_str)) = storage.get_item(LAYOUT_KEY_V9)
            && let Ok(mut v9) = serde_json::from_str::<Self>(&v9_str)
            && v9.schema_version == 9
        {
            v9.validate_and_normalize();
            return v9;
        }

        // 2. Try legacy v8 key
        if let Ok(Some(v8_str)) = storage.get_item(LAYOUT_KEY_V8)
            && let Ok(v8) = serde_json::from_str::<CanvasLayoutV8>(&v8_str)
        {
            let mut migrated = Self::from_v8(&v8);
            migrated.validate_and_normalize();
            migrated.save();
            return migrated;
        }

        let mut default_layout = Self::default();
        default_layout.validate_and_normalize();
        default_layout.save();
        default_layout
    }

    /// Save current layout to browser `localStorage` under `cybou.desktop.layout.v9`.
    pub fn save(&self) {
        let storage = web_sys::window().and_then(|w| w.local_storage().ok().flatten());
        if let Some(storage) = storage
            && let Ok(serialized) = serde_json::to_string(self)
        {
            let _ = storage.set_item(LAYOUT_KEY_V9, &serialized);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl DesktopLayout {
    /// Non-WASM loader returning validated default layout.
    #[must_use]
    pub fn load() -> Self {
        let mut def = Self::default();
        def.validate_and_normalize();
        def
    }

    /// Non-WASM save no-op.
    pub fn save(&self) {}
}
