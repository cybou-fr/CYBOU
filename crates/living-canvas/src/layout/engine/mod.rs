// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Core Spatial Layout Engine v2 managing top-level desktop surfaces, persistence, and arrangements.
//!
//! This module holds the layout itself and the operations that are about the desktop as a whole:
//! construction, what counts as a top-level item, the geometry of all of them together, and the
//! normalization that makes a loaded layout safe to trust. Cards, decks, arrangement and
//! persistence each have their own module, because each was a different reason to change one file.

use serde::{Deserialize, Serialize};

use crate::card::{CardGeometry, CardId, CardInstance, CardPresentation};
use crate::deck::DeckInstance;
use crate::layout::{
    migration::{CanvasLayoutV8, from_v8},
    model::{ArrangementMode, DesktopItem, DesktopItemId, Rect, UsableViewport},
    snap::{SnapResult, compute_snap},
};

mod arrange;
mod cards;
mod clusters;
mod decks;
mod persistence;

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
    /// Optional semantic spatial clusters grouping related panels.
    #[serde(default)]
    pub clusters: Vec<crate::layout::model::DesktopCluster>,
    /// Optional named spatial anchors / camera landmarks.
    #[serde(default)]
    pub anchors: Vec<crate::layout::model::CanvasAnchor>,
}

impl Default for DesktopLayout {
    fn default() -> Self {
        Self::canonical(None)
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
            clusters: Vec::new(),
            anchors: Vec::new(),
        }
    }

    /// Construct the canonical Desktop layout containing all 12 system cards in Home arrangement.
    #[must_use]
    pub fn canonical(viewport: Option<UsableViewport>) -> Self {
        let mut layout = Self::new();
        let canonical_cards = [
            CardId::Identity,
            CardId::Session,
            CardId::Capabilities,
            CardId::Journal,
            CardId::Lifecycle,
            CardId::Commitments,
            CardId::SelfModel,
            CardId::Attention,
            CardId::Beliefs,
            CardId::Perception,
            CardId::Context,
            CardId::Disclosure,
            CardId::Insight,
            CardId::Agents,
        ];
        for (idx, card_id) in canonical_cards.iter().enumerate() {
            let spec = card_id.spec();
            let z = u32::try_from(idx + 1).unwrap_or(1);
            layout.cards.push(CardInstance {
                id: *card_id,
                geometry: CardGeometry {
                    x: 0.0,
                    y: 0.0,
                    width: spec.default_size.0,
                    height: spec.default_size.1,
                    z,
                },
                presentation: CardPresentation::default(),
            });
        }
        layout.apply_arrangement(ArrangementMode::Home, viewport);
        layout
    }

    /// Reset the desktop to the canonical set of 12 system cards in Home arrangement,
    /// removing all temporary tool cards and dissolving all decks.
    pub fn reset_desktop(&mut self, viewport: Option<UsableViewport>) {
        *self = Self::canonical(viewport);
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

        // 5. Normalize cluster card keys to exact instance keys
        for cluster in &mut self.clusters {
            let mut normalized_keys = Vec::new();
            for key in &cluster.card_keys {
                if !key.contains(':') {
                    let matching_instances: Vec<String> = self
                        .cards
                        .iter()
                        .filter(|c| c.id.key() == key)
                        .map(|c| c.id.instance_key())
                        .collect();
                    if matching_instances.is_empty() {
                        if !normalized_keys.contains(key) {
                            normalized_keys.push(key.clone());
                        }
                    } else {
                        for inst in matching_instances {
                            if !normalized_keys.contains(&inst) {
                                normalized_keys.push(inst);
                            }
                        }
                    }
                } else if !normalized_keys.contains(key) {
                    normalized_keys.push(key.clone());
                }
            }
            cluster.card_keys = normalized_keys;
        }

        // 6. Normalize canvas anchors
        for anchor in &mut self.anchors {
            anchor.center_x = anchor.center_x.clamp(0.0, 10000.0);
            anchor.center_y = anchor.center_y.clamp(0.0, 10000.0);
            anchor.preferred_zoom = anchor.preferred_zoom.clamp(0.4, 2.0);
        }
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
