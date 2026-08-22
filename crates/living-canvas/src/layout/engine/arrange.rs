// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The automated arrangements, and the one thing they all have to respect.
//!
//! Every mode below moves unpinned items only: pinning is a person saying where something belongs,
//! and an arrangement that overrode it would make the pin a suggestion. Each mode is otherwise
//! free to choose its own geometry.

use crate::card::CardId;
use crate::layout::model::{ArrangementMode, DesktopItem, DesktopItemId, Rect, UsableViewport};
use crate::layout::relations::DesktopRelationshipGraph;

use super::DesktopLayout;

impl DesktopLayout {
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
        let num_tracks =
            (((available_w + col_gap) / (track_width + col_gap)).floor() as usize).clamp(2, 6);

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

        // Categorize items into 5 causal layers using DesktopRelationshipGraph
        let mut layer_items: [Vec<DesktopItem>; 5] =
            [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()];

        for item in items {
            if !item.is_pinned() {
                let l = DesktopRelationshipGraph::layer_for_item(&item, self);
                layer_items[l.min(4)].push(item);
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
                    // Disclosure sits beside Session, the other card about who is being answered.
                    CardId::Session
                    | CardId::Identity
                    | CardId::Perception
                    | CardId::Lifecycle
                    | CardId::Disclosure => 0,
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
