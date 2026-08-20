// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Desktop layout persistence and migration engine for CYBOU Desktop.
//!
//! Owns the `DesktopLayout` v9 schema and provides transparent, loss-less migration
//! from legacy `cybou.living-canvas.layout.v8`.

use serde::{Deserialize, Serialize};

use crate::card::{CardGeometry, CardId, CardInstance, CardPresentation};

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

/// Legacy CanvasLayout schema v8.
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
        self.card(id)
            .map_or_else(|| CardGeometry::new(50.0, 50.0, id.spec().default_size, 1), |c| c.geometry)
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

    /// Bring a card to the front by setting its z-index above all others.
    pub fn bring_forward(&mut self, id: CardId) {
        let max_z = self.cards.iter().map(|c| c.geometry.z).max().unwrap_or(0);
        if let Some(card) = self.card_mut(id) {
            card.geometry.z = max_z + 1;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
}
