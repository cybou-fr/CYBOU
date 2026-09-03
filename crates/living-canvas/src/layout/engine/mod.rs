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
    /// System cards this desktop is deliberately not showing.
    ///
    /// Without this, a System card that is not in `cards` is ambiguous: it is either one somebody
    /// closed, or one that did not exist when they last saved. The layout used to resolve that by
    /// putting every missing one back, which made closing a card last exactly until the next
    /// refresh. Recording the choice is what lets a genuinely new card still arrive.
    #[serde(default)]
    pub closed: Vec<CardId>,
}

/// How far from the origin anything may be placed, in either direction.
///
/// The canvas is unbounded as far as anybody using it is concerned; this exists so that a layout
/// arriving from storage with a coordinate of 1e300 cannot make the desktop undrawable. It is a
/// guard against nonsense, not a fence.
const CANVAS_REACH: f64 = 100_000.0;

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
            closed: Vec::new(),
        }
    }

    /// Construct the Desktop layout a person meets on their first visit.
    ///
    /// Five cards rather than all fourteen. The other nine are not gone and were never optional to
    /// build: every one of them is one click away in the Dock, in the command palette, and in
    /// [`CardId::ALL_SYSTEM_CARDS`], which is still what the desktop is checked against. What
    /// changed is what happens before anybody has chosen anything. Fourteen panels of unfamiliar
    /// vocabulary opening at once is not a demonstration of what this host knows about itself; it
    /// is a wall, and the first thing it teaches is that the desktop is not for you.
    ///
    /// These five answer the questions somebody actually arrives with. Who am I here and is this
    /// session real (Identity, Session); what is this host able to do right now (Capabilities);
    /// what has it been doing (Journal); and what does it currently think is wrong (Insight).
    #[must_use]
    pub fn canonical(viewport: Option<UsableViewport>) -> Self {
        let mut layout = Self::new();
        let canonical_cards = [
            CardId::Identity,
            CardId::Session,
            CardId::Capabilities,
            CardId::Journal,
            CardId::Insight,
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
        // The nine that are not open are recorded as closed rather than merely absent. That is
        // the first-visit decision stated: these exist, they are one click away, and they start
        // shut. Absent would mean "we have not heard of them", and the next load would open them.
        layout.closed = CardId::ALL_SYSTEM_CARDS
            .into_iter()
            .filter(|card| !canonical_cards.contains(card))
            .collect();
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

    /// What to call a place, given what is standing in it.
    ///
    /// The cards a view holds are the only description of it anybody would recognise. Two names is
    /// the limit: a list of six is not a name, it is the view written out, and by the time somebody
    /// is choosing between anchors they are reading a row of pills.
    ///
    /// A view with nothing in it is still a place worth keeping — somewhere to put things — and
    /// gets a number, because there is nothing else true to say about it.
    #[must_use]
    pub fn name_for_view(&self, view: crate::layout::model::Rect) -> String {
        let mut inside: Vec<&'static str> = self
            .cards
            .iter()
            .filter(|card| {
                let rect = crate::layout::model::Rect::new(
                    card.geometry.x,
                    card.geometry.y,
                    card.geometry.width,
                    card.geometry.height,
                );
                rect.intersects(&view)
            })
            .map(|card| card.id.title())
            .collect();
        inside.dedup();

        let base = match inside.as_slice() {
            [] => format!("Place {}", self.anchors.len() + 1),
            [one] => (*one).to_owned(),
            [first, second, ..] => format!("{first} and {second}"),
        };

        // Anchors are unique by name and `add_anchor` refuses a repeat. Somebody standing in the
        // same place twice meant to keep it twice.
        if !self
            .anchors
            .iter()
            .any(|anchor| anchor.name.eq_ignore_ascii_case(&base))
        {
            return base;
        }
        // Bounded: a desktop cannot hold more anchors than this, so the search ends even if every
        // name below the bound is taken.
        (2..=1_000_u32)
            .map(|nth| format!("{base} {nth}"))
            .find(|candidate| {
                !self
                    .anchors
                    .iter()
                    .any(|anchor| anchor.name.eq_ignore_ascii_case(candidate))
            })
            .unwrap_or(base)
    }

    /// Add a named camera anchor when its trimmed name is non-empty and unique.
    pub fn add_anchor(&mut self, name: &str, center_x: f64, center_y: f64, zoom: f64) -> bool {
        const MAX_ANCHORS: usize = 32;
        let name = name.trim();
        let normalized_name = name.to_lowercase();
        if self.anchors.len() >= MAX_ANCHORS
            || name.is_empty()
            || self
                .anchors
                .iter()
                .any(|anchor| anchor.name.to_lowercase() == normalized_name)
        {
            return false;
        }

        self.anchors.push(crate::layout::model::CanvasAnchor {
            id: format!("anchor-{}", uuid::Uuid::new_v4()),
            name: name.to_owned(),
            center_x: center_x.clamp(-CANVAS_REACH, CANVAS_REACH),
            center_y: center_y.clamp(-CANVAS_REACH, CANVAS_REACH),
            preferred_zoom: zoom.clamp(0.4, 2.0),
        });
        true
    }

    /// Rename an existing camera anchor, rejecting empty and duplicate names.
    pub fn rename_anchor(&mut self, id: &str, name: &str) -> bool {
        let name = name.trim();
        let normalized_name = name.to_lowercase();
        if name.is_empty()
            || self
                .anchors
                .iter()
                .any(|anchor| anchor.id != id && anchor.name.to_lowercase() == normalized_name)
        {
            return false;
        }
        let Some(anchor) = self.anchors.iter_mut().find(|anchor| anchor.id == id) else {
            return false;
        };
        name.clone_into(&mut anchor.name);
        true
    }

    /// Remove an existing camera anchor.
    pub fn remove_anchor(&mut self, id: &str) -> bool {
        let previous_len = self.anchors.len();
        self.anchors.retain(|anchor| anchor.id != id);
        self.anchors.len() != previous_len
    }

    /// Validate all desktop layout invariants and normalize state:
    /// 1. Ensures all 11 Mind organ system cards are present (instantiating defaults if missing).
    /// 2. Clamps all card and deck dimensions to spec min/max bounds and reachable positions.
    /// 3. Normalizes z-order monotonically to prevent gaps and overflow (Invariant L14).
    /// 4. Validates decks: dissolves invalid/empty/<2 card decks, removes duplicate cards, ensures `active_card` is in deck (Invariants L1–L4).
    /// 5. Ensures no card is in multiple decks simultaneously (Invariant L1).
    pub fn validate_and_normalize(&mut self) {
        // 1. A System card that is neither open nor deliberately closed is one that did not
        // exist when this layout was saved. Those arrive; Disclosure did, and Insight after it.
        // A card in `closed` is one somebody shut, and it stays shut — which it did not, for as
        // long as this step could not tell the two apart.
        for sys_id in CardId::ALL_SYSTEM_CARDS {
            let known = self.cards.iter().any(|c| c.id == sys_id) || self.closed.contains(&sys_id);
            if !known {
                let spec = sys_id.spec();
                let max_z = self.cards.iter().map(|c| c.geometry.z).max().unwrap_or(0);
                self.cards.push(CardInstance {
                    id: sys_id,
                    geometry: CardGeometry::new(60.0, 60.0, spec.default_size, max_z + 1),
                    presentation: CardPresentation::default(),
                });
            }
        }
        // A card cannot be both open and closed, and a layout that says so is one to believe about
        // what is on screen.
        self.closed
            .retain(|card| !self.cards.iter().any(|open| open.id == *card));

        // 2. Clamp and normalize all card geometries
        for card in &mut self.cards {
            let spec = card.id.spec();
            card.geometry.width = card.geometry.width.clamp(spec.min_size.0, spec.max_size.0);
            card.geometry.height = card.geometry.height.clamp(spec.min_size.1, spec.max_size.1);
            // Bounded far away rather than at zero. This is here to catch a corrupted layout
            // rather than to fence a person in: a card dragged left of the origin is where they
            // put it, and clamping it to zero on the next load would take it back.
            card.geometry.x = card.geometry.x.clamp(-CANVAS_REACH, CANVAS_REACH);
            card.geometry.y = card.geometry.y.clamp(-CANVAS_REACH, CANVAS_REACH);
        }

        // 3. Validate decks and resolve multi-deck card conflicts
        let mut assigned_cards = std::collections::HashSet::new();
        let mut valid_decks = Vec::new();

        for mut deck in self.decks.drain(..) {
            // Remove cards already claimed by another deck
            deck.card_ids.retain(|c| assigned_cards.insert(*c));

            if deck.validate_and_normalize() {
                deck.geometry.x = deck.geometry.x.clamp(-CANVAS_REACH, CANVAS_REACH);
                deck.geometry.y = deck.geometry.y.clamp(-CANVAS_REACH, CANVAS_REACH);
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
        let mut anchor_ids = std::collections::HashSet::new();
        let mut anchor_names = std::collections::HashSet::new();
        self.anchors.retain_mut(|anchor| {
            anchor.name = anchor.name.trim().to_owned();
            let normalized_name = anchor.name.to_lowercase();
            !anchor.id.trim().is_empty()
                && !anchor.name.is_empty()
                && anchor_ids.insert(anchor.id.clone())
                && anchor_names.insert(normalized_name)
        });
        self.anchors.truncate(32);
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
