// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `PlacementResolver` for locating collision-free layout positions for cards and detached items.

use crate::layout::model::{DesktopItem, Rect, UsableViewport};

/// Helper for resolving safe placements for detached or newly instantiated cards.
pub struct PlacementResolver;

impl PlacementResolver {
    /// Find a safe, non-overlapping placement for an item with dimensions (w, h),
    /// attempting right, bottom, left, top of a reference position, constrained to reachable bounds.
    #[must_use]
    pub fn find_placement(
        existing_items: &[DesktopItem],
        width: f64,
        height: f64,
        preferred_near: Option<Rect>,
        viewport: UsableViewport,
    ) -> (f64, f64) {
        let gap = 20.0;
        let min_bound_x = 20.0;
        let min_bound_y = 20.0;

        if let Some(pref) = preferred_near {
            let candidates = [
                // 1. Right of reference
                (pref.right() + gap, pref.y),
                // 2. Below reference
                (pref.x, pref.bottom() + gap),
                // 3. Left of reference
                (pref.x - width - gap, pref.y),
                // 4. Above reference
                (pref.x, pref.y - height - gap),
            ];

            for (cx, cy) in candidates {
                if cx >= min_bound_x && cy >= min_bound_y {
                    let candidate_rect = Rect::new(cx, cy, width, height);
                    let collides = existing_items
                        .iter()
                        .any(|item| item.effective_rect().intersects(&candidate_rect));
                    if !collides {
                        return (cx, cy);
                    }
                }
            }
        }

        // Fallback: search row by row / shelf scan
        let mut y = min_bound_y;
        while y < 3000.0 {
            let mut x = min_bound_x;
            let max_x = viewport.width.max(1200.0);
            while x < max_x {
                let candidate_rect = Rect::new(x, y, width, height);
                let collides = existing_items
                    .iter()
                    .any(|item| item.effective_rect().intersects(&candidate_rect));
                if !collides {
                    return (x, y);
                }
                x += 120.0;
            }
            y += 80.0;
        }

        (50.0, 50.0)
    }
}
