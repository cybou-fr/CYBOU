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

/// How much of the window's bottom edge the dock covers.
///
/// The dock is drawn over the canvas rather than beside it, so a card placed in this strip is a
/// card behind a toolbar. Measured against the dock's own height plus the room it leaves.
const DOCK_RESERVE: f64 = 96.0;

/// The shortest a column is allowed to be before the spillover rule gives up on it.
///
/// On a window too short to hold anything, refusing to place cards would be worse than placing
/// them where they can be panned to.
const MIN_COLUMN_RUN: f64 = 200.0;

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

    /// Where each column starts, given what is actually in the ones before it.
    ///
    /// Both column arrangements used to step by a constant — 360 in one, 380 in the other — while
    /// cards range from 220 to 560 wide. A wide card therefore ran into the next column, and the
    /// last column could start at a coordinate that put its contents past the edge of the window
    /// with nothing saying so. A column is as wide as its widest member.
    fn column_offsets(
        columns: &[Vec<DesktopItem>],
        start_x: f64,
        gap: f64,
        minimum: f64,
    ) -> Vec<f64> {
        let mut offsets = Vec::with_capacity(columns.len());
        let mut x = start_x;
        for column in columns {
            offsets.push(x);
            let widest = column
                .iter()
                .map(|item| item.geometry.width)
                .fold(minimum, f64::max);
            x += widest + gap;
        }
        offsets
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

        // As many columns as the graph produces, rather than a constant that has to be found again
        // whenever an edge is added. A fixed five silently folded anything deeper into the last one.
        let layers = DesktopRelationshipGraph::layer_count();
        let mut layer_items: Vec<Vec<DesktopItem>> = vec![Vec::new(); layers];

        for item in items {
            if !item.is_pinned() {
                let l = DesktopRelationshipGraph::layer_for_item(&item, self);
                layer_items[l.min(layers - 1)].push(item);
            }
        }

        let offsets = Self::column_offsets(&layer_items, start_x, col_gap, col_width);

        for (l_idx, layer) in layer_items.into_iter().enumerate() {
            let col_x = offsets[l_idx];
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
                    // Insight sits beside Perception: both are about the machine rather than about
                    // Mind, and a reader looking at one usually wants the other.
                    CardId::Session
                    | CardId::Identity
                    | CardId::Perception
                    | CardId::Insight
                    | CardId::Lifecycle
                    | CardId::Disclosure => 0,
                    // Agents sits with Capabilities and Journal: all three are about what this
                    // host is doing on somebody's behalf rather than what it is.
                    CardId::Capabilities | CardId::Journal | CardId::Attention | CardId::Agents => {
                        1 % num_cols
                    }
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

        let offsets = Self::column_offsets(&cols, start_x, col_gap, col_width);

        // Where a column has to stop. The dock stands along the bottom of the window and is drawn
        // over the canvas, so the last stretch of the viewport is not somewhere a card can be seen.
        let floor = (viewport.height - DOCK_RESERVE).max(start_y + MIN_COLUMN_RUN);
        // The first column of a spillover. Everything semantic has already been given an x, so a
        // column that runs out of height continues to the right of all of them rather than on top
        // of one of them.
        let mut next_band_x = offsets
            .iter()
            .zip(&cols)
            .map(|(x, col)| {
                x + col
                    .iter()
                    .map(|item| item.geometry.width)
                    .fold(col_width, f64::max)
            })
            .fold(start_x, f64::max)
            + col_gap;

        for (c_idx, col) in cols.into_iter().enumerate() {
            let mut col_x = offsets[c_idx];
            let mut cur_y = start_y;

            for item in col {
                let eff_w = item.geometry.width;
                let eff_h = item.effective_height();

                // A card that would hang below the dock starts a new column instead. Only when
                // something is already above it: a card taller than the whole viewport has nowhere
                // better to go, and moving it sideways forever would be a loop.
                if cur_y > start_y && cur_y + eff_h > floor {
                    col_x = next_band_x;
                    next_band_x += eff_w.max(col_width) + col_gap;
                    cur_y = start_y;
                }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CardId;
    use crate::layout::engine::DesktopLayout;

    #[test]
    fn the_home_arrangement_puts_nothing_under_the_dock() {
        // The first column held six cards on a viewport nine hundred pixels tall, so the desktop a
        // person met on their first visit ran off the bottom of it with nothing saying so.
        let viewport = UsableViewport {
            width: 1440.0,
            height: 900.0,
        };
        let layout = DesktopLayout::canonical(Some(viewport));
        let floor = viewport.height - DOCK_RESERVE;

        for card in &layout.cards {
            let bottom = card.geometry.y + card.geometry.height;
            assert!(
                bottom <= floor,
                "{:?} ends at {bottom}, below the {floor} the dock leaves",
                card.id
            );
        }
    }

    #[test]
    fn a_card_taller_than_the_window_is_placed_rather_than_moved_sideways_forever() {
        // The spillover rule asks whether something is already above the card. Without that, a card
        // that cannot fit anywhere would take a new column, fail again, and take another.
        let viewport = UsableViewport {
            width: 1440.0,
            height: 200.0,
        };
        let layout = DesktopLayout::canonical(Some(viewport));
        assert_eq!(
            layout.cards.len(),
            14,
            "every canonical card is still on the desktop"
        );
        for card in &layout.cards {
            assert!(
                card.geometry.x.is_finite() && card.geometry.y.is_finite(),
                "{:?} was never given a place",
                card.id
            );
        }
        assert!(
            layout
                .cards
                .iter()
                .any(|card| card.id == CardId::Identity && card.geometry.y > 0.0),
            "the arrangement still ran"
        );
    }
}
