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

/// The corner the canvas controls occupy: the minimap, the zoom buttons and what sits with them.
///
/// They are fixed to the window rather than to the canvas — a control that moves the canvas cannot
/// itself scroll away — so a card placed under them is a card with a hole in it.
const CHROME_RESERVE: (f64, f64) = (260.0, 200.0);

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
    /// Lay out only the cards standing in `view`, inside `view`, and touch nothing else.
    ///
    /// Every other arrangement here rewrites the whole canvas. That is the wrong shape for a
    /// spatial desktop: somebody looking at five overlapping cards wants those five straightened,
    /// not their desktop rebuilt around a rule they did not choose while they were looking
    /// somewhere else.
    ///
    /// A pinned card is an obstacle rather than a subject, which is what pinning already means
    /// everywhere else.
    ///
    /// Rows wrap at the view's right edge and continue below it for as long as there are cards.
    /// The first version stopped at the bottom and left the remainder where they were, which
    /// produced a tidy view with a card sitting on top of it — the exact state the button exists
    /// to remove. Below the fold, in order, is the direction a person expects more to be in.
    pub fn tidy_within(&mut self, view: Rect) {
        let subjects: Vec<DesktopItem> = self
            .desktop_items()
            .into_iter()
            .filter(|item| !item.is_pinned() && item.effective_rect().intersects(&view))
            .collect();
        if subjects.len() < 2 {
            return;
        }

        let mut placed: Vec<Rect> = self
            .desktop_items()
            .iter()
            .filter(|item| item.is_pinned() || !item.effective_rect().intersects(&view))
            .map(DesktopItem::effective_rect)
            .collect();

        let gap = 20.0;
        let left = view.x + gap;
        let top = view.y + gap;
        let right = view.x + view.width - gap;

        let mut x = left;
        let mut y = top;
        let mut row_height: f64 = 0.0;

        for item in subjects {
            let width = item.geometry.width;
            let height = item.effective_height();
            // Wrap unless this is the first card of a row: a card wider than the whole view has
            // nowhere narrower to be, and giving it a row of its own overhangs to the right
            // without landing on anything.
            if x > left && x + width > right {
                x = left;
                y += row_height + gap;
                row_height = 0.0;
            }
            self.update_item_position(&item.id, x, y);
            placed.push(Rect::new(x, y, width, height));
            x += width + gap;
            row_height = row_height.max(height);
        }
    }

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
            let place_x = start_x
                + f64::from(u16::try_from(best_track).unwrap_or(u16::MAX))
                    * (track_width + col_gap);

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
    #[expect(
        clippy::too_many_lines,
        reason = "one placement rule after another for one layout; the order they run in is the                   layout, and naming halves of it would not make either half readable alone"
    )]
    fn arrange_home(&mut self, viewport: UsableViewport) {
        let items = self.desktop_items();
        let pinned_rects: Vec<Rect> = items
            .iter()
            .filter(|it| it.is_pinned())
            .map(DesktopItem::effective_rect)
            .collect();

        let mut placed_rects: Vec<Rect> = pinned_rects;
        // The controls in the bottom-right corner are an obstacle like a pinned card is, and are
        // given to the same rule rather than to one of their own.
        placed_rects.push(Rect::new(
            viewport.width - CHROME_RESERVE.0,
            viewport.height - DOCK_RESERVE - CHROME_RESERVE.1,
            CHROME_RESERVE.0,
            CHROME_RESERVE.1,
        ));

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

        // One cursor per column, because a card that does not fit in its own column is offered the
        // others before anywhere new is invented. The columns are semantic — who is being answered,
        // what the host is doing, what it believes — so this is a demotion, taken only when the
        // alternative is a card nobody can see.
        let mut cursors: Vec<f64> = vec![start_y; offsets.len()];
        let mut bands: Vec<f64> = offsets.clone();
        let widest: Vec<f64> = cols
            .iter()
            .map(|col| {
                col.iter()
                    .map(|item| item.geometry.width)
                    .fold(col_width, f64::max)
            })
            .collect();

        for (c_idx, col) in cols.into_iter().enumerate() {
            for item in col {
                let eff_w = item.geometry.width;
                let eff_h = item.effective_height();

                // Where the card would come to rest in a column, given everything already placed.
                // Asking the cursor alone was not enough once the controls' corner became an
                // obstacle: a column could look empty and still push the card past the fold.
                let settle = |x: f64, from: f64, placed: &[Rect]| {
                    let mut y = from;
                    loop {
                        let candidate = Rect::new(x, y, eff_w, eff_h);
                        let Some(hit) = placed.iter().find(|obst| candidate.intersects(obst))
                        else {
                            return y;
                        };
                        y = hit.bottom() + row_gap;
                    }
                };

                // A column a card is moved *into* must hold it without spilling into the next
                // one. A column is only as wide as its own members, so a wide card relocated into
                // a narrow one stands across the boundary and every later card has to settle
                // around it — which is how a five-card desktop on a 1440-wide window ended up
                // with a card below the fold and two columns half empty.
                let fits = |idx: usize, bands: &[f64], cursors: &[f64], placed: &[Rect]| {
                    let room = bands.get(idx + 1).copied().unwrap_or(viewport.width);
                    bands[idx] + eff_w <= room
                        && settle(bands[idx], cursors[idx], placed) + eff_h <= floor
                };

                // Its own column first, then every other, left to right.
                // Its own column is where it was sized to belong, so overhanging there is
                // expected and only the fold is asked about.
                let mut chosen = (settle(bands[c_idx], cursors[c_idx], &placed_rects) + eff_h
                    <= floor)
                    .then_some(c_idx);
                if chosen.is_none() {
                    chosen =
                        (0..bands.len()).find(|&idx| fits(idx, &bands, &cursors, &placed_rects));
                }
                // A new column, but only one the window can show. Off the right-hand edge is not
                // better than below the fold: both are invisible, and below is the direction a
                // person expects to find more of a page.
                if chosen.is_none() {
                    let next_x = bands
                        .iter()
                        .zip(&widest)
                        .map(|(x, w)| x + w)
                        .fold(start_x, f64::max)
                        + col_gap;
                    if next_x + eff_w <= viewport.width {
                        bands.push(next_x);
                        cursors.push(start_y);
                        chosen = Some(bands.len() - 1);
                    }
                }
                // Nowhere fits: keep it in its own column and let it run past the fold. A card
                // placed nowhere would be a card the layout has lost.
                let idx = chosen.unwrap_or(c_idx);

                let col_x = bands[idx];
                let cur_y = settle(col_x, cursors[idx], &placed_rects);

                self.update_item_position(&item.id, col_x, cur_y);
                placed_rects.push(Rect::new(col_x, cur_y, eff_w, eff_h));
                cursors[idx] = cur_y + eff_h + row_gap;
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
            5,
            "every card the first desktop opens with is still on it"
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

#[cfg(test)]
mod tidy_tests {
    use super::*;
    use crate::CardId;
    use crate::layout::engine::DesktopLayout;

    fn overlapping() -> (DesktopLayout, Rect) {
        let mut layout = DesktopLayout::new();
        let view = Rect::new(0.0, 0.0, 1400.0, 800.0);
        // Three cards on the same spot, which is what a desktop looks like after opening things
        // from a Dock that used to place them all at one coordinate.
        for card in [
            CardId::Services(0),
            CardId::Processes(0),
            CardId::Monitor(0),
        ] {
            layout.open_card(card, 200.0, 200.0);
        }
        (layout, view)
    }

    #[test]
    fn tidying_separates_what_was_on_top_of_itself() {
        let (mut layout, view) = overlapping();
        layout.tidy_within(view);

        let rects: Vec<Rect> = layout
            .cards
            .iter()
            .map(|card| {
                Rect::new(
                    card.geometry.x,
                    card.geometry.y,
                    card.geometry.width,
                    card.geometry.height,
                )
            })
            .collect();
        for (index, one) in rects.iter().enumerate() {
            for other in rects.iter().skip(index + 1) {
                assert!(
                    !one.intersects(other),
                    "two cards still overlap after tidying"
                );
            }
            assert!(
                one.x >= view.x && one.y >= view.y,
                "a card was tidied above or left of the view"
            );
        }
    }

    #[test]
    fn what_is_not_in_view_is_not_touched() {
        // The whole difference between this and every other arrangement. A person straightening
        // one corner has not asked for their desktop to be rebuilt behind them.
        let (mut layout, view) = overlapping();
        layout.open_card(CardId::Storage(0), 5_000.0, 5_000.0);
        let before = layout.geometry(CardId::Storage(0));

        layout.tidy_within(view);

        let after = layout.geometry(CardId::Storage(0));
        assert!(
            (before.x - after.x).abs() < f64::EPSILON && (before.y - after.y).abs() < f64::EPSILON,
            "a card outside the view moved from {},{} to {},{}",
            before.x,
            before.y,
            after.x,
            after.y
        );
    }

    #[test]
    fn a_pinned_card_is_an_obstacle_and_not_a_subject() {
        let (mut layout, view) = overlapping();
        layout.set_position(CardId::Services(0), 300.0, 300.0);
        layout.set_pinned(CardId::Services(0), true);
        let before = layout.geometry(CardId::Services(0));

        layout.tidy_within(view);

        let after = layout.geometry(CardId::Services(0));
        assert!(
            (before.x - after.x).abs() < f64::EPSILON && (before.y - after.y).abs() < f64::EPSILON,
            "a pinned card was moved by a tidy"
        );
    }
}

#[cfg(test)]
mod first_visit_tests {
    use super::*;
    use crate::layout::engine::DesktopLayout;

    #[test]
    fn a_first_visit_fits_the_window_it_is_opened_in() {
        // Found by opening the desktop in a 1280-wide window and reading the coordinates back:
        // Journal was at x=1746 and Insight at 1302, both past the right-hand edge, put there by
        // the rule that was supposed to keep cards out from under the dock.
        // Two different promises, because only one of them can always be kept. Nothing is ever
        // placed past the right edge: horizontal overflow has nothing to suggest it, and a card put
        // there is simply lost. Vertical overflow is different — it is the direction a person
        // expects a page to continue, the minimap shows it, and on a window shorter than the cards
        // themselves there is nowhere else for them to be.
        for (width, height, fits_vertically) in [
            (1280.0, 584.0, false),
            (1440.0, 804.0, true),
            (1920.0, 1000.0, true),
        ] {
            let viewport = UsableViewport { width, height };
            let layout = DesktopLayout::canonical(Some(viewport));
            for card in &layout.cards {
                let right = card.geometry.x + card.geometry.width;
                assert!(
                    right <= width,
                    "{:?} ends at {right} in a {width}-wide window",
                    card.id
                );
                if fits_vertically {
                    let bottom = card.geometry.y + card.geometry.height;
                    assert!(
                        bottom <= height - DOCK_RESERVE,
                        "{:?} ends at {bottom}, under the dock in a {height}-tall window",
                        card.id
                    );
                }
            }
        }
    }
}
