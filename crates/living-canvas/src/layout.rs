// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Desktop layout persistence and migration engine for CYBOU Desktop.
//!
//! Owns the `DesktopLayout` v9 schema and provides transparent, loss-less migration
//! from legacy `cybou.living-canvas.layout.v8`.

use serde::{Deserialize, Serialize};

use crate::card::{CardGeometry, CardId, CardInstance, CardPresentation};
use crate::deck::DeckInstance;

/// Desktop layout schema version 9 storage key in browser `localStorage`.
pub const LAYOUT_KEY_V9: &str = "cybou.desktop.layout.v9";

/// Legacy layout schema version 8 storage key.
pub const LAYOUT_KEY_V8: &str = "cybou.living-canvas.layout.v8";

/// Legacy 2D point from v8 layout schema.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct PointV8 {
    /// X offset in pixels.
    pub x: f64,
    /// Y offset in pixels.
    pub y: f64,
    /// Stacking order.
    pub z: u32,
}

/// Legacy `CanvasLayout` schema v8.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct CanvasLayoutV8 {
    /// Identity card position.
    pub identity: PointV8,
    /// Session card position.
    pub session: PointV8,
    /// Capabilities card position.
    pub capabilities: PointV8,
    /// Journal card position.
    pub journal: PointV8,
    /// Lifecycle card position.
    pub lifecycle: PointV8,
    /// Commitments card position.
    pub commitments: PointV8,
    /// Self-model card position.
    pub self_model: PointV8,
    /// Attention card position.
    pub attention: PointV8,
    /// Beliefs card position.
    pub beliefs: PointV8,
    /// Perception card position.
    pub perception: PointV8,
    /// Context card position.
    pub context: PointV8,
}

impl Default for CanvasLayoutV8 {
    fn default() -> Self {
        Self {
            identity: PointV8 {
                x: 70.0,
                y: 50.0,
                z: 1,
            },
            session: PointV8 {
                x: 55.0,
                y: 300.0,
                z: 2,
            },
            capabilities: PointV8 {
                x: 445.0,
                y: 70.0,
                z: 6,
            },
            journal: PointV8 {
                x: 880.0,
                y: 50.0,
                z: 3,
            },
            lifecycle: PointV8 {
                x: 900.0,
                y: 340.0,
                z: 5,
            },
            commitments: PointV8 {
                x: 470.0,
                y: 410.0,
                z: 4,
            },
            self_model: PointV8 {
                x: 55.0,
                y: 600.0,
                z: 7,
            },
            attention: PointV8 {
                x: 470.0,
                y: 620.0,
                z: 8,
            },
            beliefs: PointV8 {
                x: 880.0,
                y: 620.0,
                z: 9,
            },
            perception: PointV8 {
                x: 55.0,
                y: 840.0,
                z: 10,
            },
            context: PointV8 {
                x: 470.0,
                y: 840.0,
                z: 11,
            },
        }
    }
}

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
        Self::from_v8(&CanvasLayoutV8::default())
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

    /// Migrate a legacy v8 layout into v9 format.
    ///
    /// Preserves exact (x, y, z) coordinates while populating default dimensions from `CardSpec`
    /// and setting initial presentation state to uncollapsed and unpinned.
    #[must_use]
    pub fn from_v8(v8: &CanvasLayoutV8) -> Self {
        let pairs = [
            (CardId::Identity, v8.identity),
            (CardId::Session, v8.session),
            (CardId::Capabilities, v8.capabilities),
            (CardId::Journal, v8.journal),
            (CardId::Lifecycle, v8.lifecycle),
            (CardId::Commitments, v8.commitments),
            (CardId::SelfModel, v8.self_model),
            (CardId::Attention, v8.attention),
            (CardId::Beliefs, v8.beliefs),
            (CardId::Perception, v8.perception),
            (CardId::Context, v8.context),
        ];

        let cards = pairs
            .into_iter()
            .map(|(id, pt)| CardInstance {
                id,
                geometry: CardGeometry::new(pt.x, pt.y, id.spec().default_size, pt.z),
                presentation: CardPresentation::default(),
            })
            .collect();

        Self {
            schema_version: 9,
            cards,
            decks: Vec::new(),
        }
    }

    /// Retrieve card instance by ID if present.
    #[must_use]
    pub fn card(&self, id: CardId) -> Option<&CardInstance> {
        self.cards.iter().find(|c| c.id == id)
    }

    /// Retrieve mutable card instance by ID if present.
    pub fn card_mut(&mut self, id: CardId) -> Option<&mut CardInstance> {
        self.cards.iter_mut().find(|c| c.id == id)
    }

    /// Get geometry for a given card ID (or default if missing).
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

    /// Toggle or set maximized state for a card.
    pub fn set_maximized(&mut self, id: CardId, maximized: bool) {
        if let Some(card) = self.card_mut(id) {
            card.presentation.maximized = maximized;
            if maximized {
                card.presentation.collapsed = false;
            }
        }
    }

    /// Bring a card to the front by setting its z-index above all others.
    pub fn bring_forward(&mut self, id: CardId) {
        let max_z = self.cards.iter().map(|c| c.geometry.z).max().unwrap_or(0);
        if let Some(card) = self.card_mut(id) {
            card.geometry.z = max_z + 1;
        }
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
    pub fn create_deck(
        &mut self,
        title: impl Into<String>,
        cards: Vec<CardId>,
        x: f64,
        y: f64,
    ) -> String {
        let id = format!("deck-{}", uuid::Uuid::new_v4());
        let first_card = cards.first().copied();
        let (w, h) = if let Some(fc) = first_card {
            let spec = fc.spec();
            (spec.default_size.0.max(340.0), (spec.default_size.1 + 36.0).max(220.0))
        } else {
            (360.0, 240.0)
        };
        let mut deck = DeckInstance::new(&id, title, cards, x, y);
        deck.geometry.width = w;
        deck.geometry.height = h;
        self.decks.push(deck);
        id
    }

    /// Add a card into an existing deck.
    pub fn add_to_deck(&mut self, deck_id: &str, card: CardId) {
        if let Some(deck) = self.decks.iter_mut().find(|d| d.id == deck_id) {
            deck.add_card(card);
        }
    }

    /// Detach a card from a deck, restoring it as an independent card next to the deck.
    pub fn detach_from_deck(&mut self, deck_id: &str, card: CardId) {
        let mut should_dissolve = false;
        let mut detach_x = 50.0;
        let mut detach_y = 50.0;

        if let Some(deck) = self.decks.iter_mut().find(|d| d.id == deck_id) {
            detach_x = deck.geometry.x + deck.geometry.width + 24.0;
            detach_y = deck.geometry.y;
            deck.remove_card(card);
            if deck.len() <= 1 {
                should_dissolve = true;
            }
        }

        self.set_position(card, detach_x, detach_y);
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
            deck.geometry.x = x.clamp(12.0, 3800.0);
            deck.geometry.y = y.clamp(12.0, 3800.0);
        }
    }

    /// Update size for a deck.
    pub fn set_deck_size(&mut self, id: &str, width: f64, height: f64) {
        if let Some(deck) = self.deck_mut(id) {
            deck.geometry.width = width.clamp(280.0, 800.0);
            deck.geometry.height = height.clamp(160.0, 700.0);
        }
    }

    /// Bring deck forward in stacking order.
    pub fn bring_deck_forward(&mut self, id: &str) {
        let max_z = self
            .cards
            .iter()
            .map(|c| c.geometry.z)
            .chain(self.decks.iter().map(|d| d.geometry.z))
            .max()
            .unwrap_or(1);
        if let Some(deck) = self.deck_mut(id) {
            deck.geometry.z = max_z + 1;
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

    /// Apply an automated spatial arrangement algorithm.
    ///
    /// Pinned cards are strictly preserved at their current spatial coordinates.
    pub fn apply_arrangement(&mut self, mode: ArrangementMode) {
        match mode {
            ArrangementMode::Free => {}
            ArrangementMode::Grid => self.arrange_grid(),
            ArrangementMode::Compact => self.arrange_compact(),
            ArrangementMode::Relations => self.arrange_relations(),
            ArrangementMode::Focus(target) => self.arrange_focus(target),
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn arrange_grid(&mut self) {
        let col_width = 380.0;
        let col_gap = 24.0;
        let row_gap = 20.0;
        let start_x = 40.0;
        let start_y = 40.0;
        let num_cols = 3;

        let mut col_heights = [start_y; 3];

        for card in &mut self.cards {
            if card.presentation.pinned {
                continue;
            }
            let mut min_col = 0;
            for (col_idx, &h) in col_heights.iter().enumerate().take(num_cols) {
                if h < col_heights[min_col] {
                    min_col = col_idx;
                }
            }

            let x = start_x + (min_col as f64) * (col_width + col_gap);
            let y = col_heights[min_col];
            card.geometry.x = x;
            card.geometry.y = y;

            let h = if card.presentation.collapsed {
                44.0
            } else {
                card.geometry.height
            };
            col_heights[min_col] += h + row_gap;
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn arrange_compact(&mut self) {
        let col_width = 350.0;
        let col_gap = 16.0;
        let row_gap = 14.0;
        let start_x = 30.0;
        let start_y = 30.0;
        let num_cols = 3;

        let mut col_heights = [start_y; 3];

        for card in &mut self.cards {
            if card.presentation.pinned {
                continue;
            }
            let mut min_col = 0;
            for (col_idx, &h) in col_heights.iter().enumerate().take(num_cols) {
                if h < col_heights[min_col] {
                    min_col = col_idx;
                }
            }

            let x = start_x + (min_col as f64) * (col_width + col_gap);
            let y = col_heights[min_col];
            card.geometry.x = x;
            card.geometry.y = y;

            let h = if card.presentation.collapsed {
                44.0
            } else {
                card.geometry.height
            };
            col_heights[min_col] += h + row_gap;
        }
    }

    fn arrange_relations(&mut self) {
        let topology = [
            (CardId::Session, 50.0, 50.0),
            (CardId::Identity, 50.0, 320.0),
            (CardId::Perception, 50.0, 580.0),
            (CardId::SelfModel, 50.0, 800.0),
            (CardId::Capabilities, 440.0, 50.0),
            (CardId::Commitments, 440.0, 420.0),
            (CardId::Attention, 440.0, 680.0),
            (CardId::Context, 440.0, 900.0),
            (CardId::Journal, 880.0, 50.0),
            (CardId::Lifecycle, 880.0, 400.0),
            (CardId::Beliefs, 880.0, 680.0),
        ];

        for (id, tx, ty) in topology {
            if let Some(card) = self.card_mut(id)
                && !card.presentation.pinned
            {
                card.geometry.x = tx;
                card.geometry.y = ty;
            }
        }
    }

    fn arrange_focus(&mut self, target_id: CardId) {
        let center_x = 420.0;
        let center_y = 60.0;

        if let Some(target) = self.card_mut(target_id)
            && !target.presentation.pinned
        {
            target.geometry.x = center_x;
            target.geometry.y = center_y;
            target.presentation.collapsed = false;
        }

        let mut left_y = 40.0;
        let mut right_y = 40.0;
        let mut toggle = false;

        for card in &mut self.cards {
            if card.id == target_id || card.presentation.pinned {
                continue;
            }
            card.presentation.collapsed = true;

            if toggle {
                card.geometry.x = 40.0;
                card.geometry.y = left_y;
                left_y += 52.0;
            } else {
                card.geometry.x = 920.0;
                card.geometry.y = right_y;
                right_y += 52.0;
            }
            toggle = !toggle;
        }
    }
}

/// Spatial arrangement algorithm mode.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ArrangementMode {
    /// Free-form unconstrained positioning.
    Free,
    /// Compact packing with top-left gravity.
    Compact,
    /// Structured multi-column grid alignment.
    Grid,
    /// Force-directed / clustered layout around canonical causal relationships.
    Relations,
    /// Focused presentation centering target card enlarged, others placed around borders.
    Focus(CardId),
}

#[cfg(target_arch = "wasm32")]
impl DesktopLayout {
    /// Load layout from browser `localStorage`, seamlessly migrating from v8 if necessary.
    #[must_use]
    pub fn load() -> Self {
        let storage = web_sys::window().and_then(|w| w.local_storage().ok().flatten());
        let Some(storage) = storage else {
            return Self::default();
        };

        // 1. Try v9 key first
        if let Ok(Some(v9_str)) = storage.get_item(LAYOUT_KEY_V9)
            && let Ok(v9) = serde_json::from_str::<Self>(&v9_str)
            && v9.schema_version == 9
        {
            return v9;
        }

        // 2. Try legacy v8 key
        if let Ok(Some(v8_str)) = storage.get_item(LAYOUT_KEY_V8)
            && let Ok(v8) = serde_json::from_str::<CanvasLayoutV8>(&v8_str)
        {
            let migrated = Self::from_v8(&v8);
            migrated.save();
            return migrated;
        }

        let default_layout = Self::default();
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
    /// Non-WASM dummy loader returning default layout.
    #[must_use]
    pub fn load() -> Self {
        Self::default()
    }

    /// Non-WASM dummy save no-op.
    pub fn save(&self) {}
}

/// Undo/Redo stack for desktop layout modifications.
#[derive(Clone, Debug, Default)]
pub struct LayoutHistory {
    undo_stack: Vec<DesktopLayout>,
    redo_stack: Vec<DesktopLayout>,
}

impl LayoutHistory {
    /// Maximum number of undo states retained in memory.
    pub const MAX_HISTORY: usize = 25;

    /// Construct a new empty history stack.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Record current layout state before an arrangement or significant mutation.
    pub fn push(&mut self, layout: DesktopLayout) {
        if self.undo_stack.last() == Some(&layout) {
            return;
        }
        self.undo_stack.push(layout);
        if self.undo_stack.len() > Self::MAX_HISTORY {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// Undo to previous layout state.
    pub fn undo(&mut self, current: DesktopLayout) -> Option<DesktopLayout> {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(current);
            Some(prev)
        } else {
            None
        }
    }

    /// Redo to next layout state.
    pub fn redo(&mut self, current: DesktopLayout) -> Option<DesktopLayout> {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(current);
            Some(next)
        } else {
            None
        }
    }

    /// Whether undo is available.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Whether redo is available.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn default_layout_has_all_system_cards() {
        let layout = DesktopLayout::default();
        assert_eq!(layout.schema_version, 9);
        assert_eq!(layout.cards.len(), 11);

        for card_id in CardId::ALL_SYSTEM_CARDS {
            let card = layout.card(card_id).expect("card exists in layout");
            assert_eq!(card.geometry.width, card_id.spec().default_size.0);
            assert_eq!(card.geometry.height, card_id.spec().default_size.1);
            assert!(!card.presentation.collapsed);
            assert!(!card.presentation.pinned);
        }
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn v8_to_v9_migration_preserves_coordinates() {
        let v8 = CanvasLayoutV8 {
            identity: PointV8 {
                x: 123.0,
                y: 456.0,
                z: 10,
            },
            ..CanvasLayoutV8::default()
        };

        let v9 = DesktopLayout::from_v8(&v8);
        assert_eq!(v9.schema_version, 9);
        let id_card = v9.card(CardId::Identity).expect("identity card present");
        assert_eq!(id_card.geometry.x, 123.0);
        assert_eq!(id_card.geometry.y, 456.0);
        assert_eq!(id_card.geometry.z, 10);
        assert_eq!(id_card.geometry.width, 220.0);
        assert_eq!(id_card.geometry.height, 188.0);
    }

    #[test]
    fn parse_json_supports_both_schemas() {
        let v8_json = serde_json::to_string(&CanvasLayoutV8::default()).expect("serialize v8");
        let parsed_v8 = DesktopLayout::parse_json(&v8_json).expect("parse legacy v8");
        assert_eq!(parsed_v8.schema_version, 9);
        assert_eq!(parsed_v8.cards.len(), 11);

        let v9_json = serde_json::to_string(&parsed_v8).expect("serialize v9");
        let parsed_v9 = DesktopLayout::parse_json(&v9_json).expect("parse v9");
        assert_eq!(parsed_v9, parsed_v8);
    }

    #[test]
    fn bring_forward_advances_z_index() {
        let mut layout = DesktopLayout::default();
        let old_z = layout.geometry(CardId::Identity).z;
        layout.bring_forward(CardId::Identity);
        let new_z = layout.geometry(CardId::Identity).z;
        assert!(new_z > old_z);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn arrangement_preserves_pinned_cards() {
        let mut layout = DesktopLayout::default();
        layout.set_pinned(CardId::Identity, true);
        layout.set_position(CardId::Identity, 777.0, 888.0);

        // Apply grid
        layout.apply_arrangement(ArrangementMode::Grid);
        let id_geom = layout.geometry(CardId::Identity);
        assert_eq!(id_geom.x, 777.0);
        assert_eq!(id_geom.y, 888.0);

        // Apply compact
        layout.apply_arrangement(ArrangementMode::Compact);
        let id_geom = layout.geometry(CardId::Identity);
        assert_eq!(id_geom.x, 777.0);
        assert_eq!(id_geom.y, 888.0);

        // Apply relations
        layout.apply_arrangement(ArrangementMode::Relations);
        let id_geom = layout.geometry(CardId::Identity);
        assert_eq!(id_geom.x, 777.0);
        assert_eq!(id_geom.y, 888.0);

        // Apply focus
        layout.apply_arrangement(ArrangementMode::Focus(CardId::Capabilities));
        let id_geom = layout.geometry(CardId::Identity);
        assert_eq!(id_geom.x, 777.0);
        assert_eq!(id_geom.y, 888.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn arrangement_relations_and_focus_positioning() {
        let mut layout = DesktopLayout::default();

        layout.apply_arrangement(ArrangementMode::Relations);
        let cap_geom = layout.geometry(CardId::Capabilities);
        assert_eq!(cap_geom.x, 440.0);
        assert_eq!(cap_geom.y, 50.0);

        layout.apply_arrangement(ArrangementMode::Focus(CardId::Journal));
        let journal_geom = layout.geometry(CardId::Journal);
        assert_eq!(journal_geom.x, 420.0);
        assert_eq!(journal_geom.y, 60.0);
        assert!(!layout.presentation(CardId::Journal).collapsed);

        // Other non-pinned cards are collapsed
        assert!(layout.presentation(CardId::Identity).collapsed);
    }

    #[test]
    fn deck_management_and_dissolution() {
        let mut layout = DesktopLayout::default();
        let deck_id = layout.create_deck(
            "System Overview",
            vec![CardId::Identity, CardId::Session],
            100.0,
            100.0,
        );

        assert_eq!(layout.decks.len(), 1);
        assert!(layout.is_in_deck(CardId::Identity));
        assert!(layout.is_in_deck(CardId::Session));
        assert!(!layout.is_in_deck(CardId::Capabilities));

        layout.add_to_deck(&deck_id, CardId::Capabilities);
        assert!(layout.is_in_deck(CardId::Capabilities));

        // Detach one card
        layout.detach_from_deck(&deck_id, CardId::Capabilities);
        assert!(!layout.is_in_deck(CardId::Capabilities));
        assert_eq!(layout.decks.len(), 1);

        // Detach another, deck dissolves when 1 card left
        layout.detach_from_deck(&deck_id, CardId::Session);
        assert!(!layout.is_in_deck(CardId::Session));
        assert!(!layout.is_in_deck(CardId::Identity));
        assert_eq!(layout.decks.len(), 0);
    }

    #[test]
    fn layout_history_undo_redo() {
        let mut history = LayoutHistory::new();
        let initial = DesktopLayout::default();

        let mut modified = initial.clone();
        modified.set_position(CardId::Identity, 999.0, 999.0);

        history.push(initial.clone());
        assert!(history.can_undo());
        assert!(!history.can_redo());

        let reverted = history.undo(modified.clone()).expect("undo succeeds");
        // Compared with a tolerance: these are coordinates carried through a clone and back, and
        // asserting bit equality on a float is a test that can fail for reasons the code is not
        // about.
        assert!((reverted.geometry(CardId::Identity).x - 70.0).abs() < f64::EPSILON);
        assert!(history.can_redo());

        let redone = history.redo(reverted).expect("redo succeeds");
        assert!((redone.geometry(CardId::Identity).x - 999.0).abs() < f64::EPSILON);
    }
}
