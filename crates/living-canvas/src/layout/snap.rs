// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Magnetic snap alignment calculations and visual guidelines.

use crate::layout::model::{DesktopItem, DesktopItemId};

/// Alignment snap guide lines for spatial compositor canvas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SnapGuide {
    /// Vertical snap alignment line at x coordinate.
    Vertical(f64),
    /// Horizontal snap alignment line at y coordinate.
    Horizontal(f64),
}

/// Snap calculation result including snapped coordinates and active visual guides.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SnapResult {
    /// Snapped X coordinate.
    pub snapped_x: f64,
    /// Snapped Y coordinate.
    pub snapped_y: f64,
    /// Active alignment guide lines to render on canvas.
    pub guides: Vec<SnapGuide>,
}

/// Compute snap alignment against other desktop items during drag/resize operations.
#[allow(clippy::similar_names)]
#[must_use]
pub fn compute_snap(
    items: &[DesktopItem],
    dragged_id: &DesktopItemId,
    candidate_x: f64,
    candidate_y: f64,
    width: f64,
    height: f64,
    snap_threshold: f64,
) -> SnapResult {
    let mut snapped_x = candidate_x;
    let mut snapped_y = candidate_y;
    let mut guides = Vec::new();

    let cand_left = candidate_x;
    let cand_mid_x = candidate_x + width / 2.0;
    let cand_right = candidate_x + width;

    let cand_top = candidate_y;
    let cand_mid_y = candidate_y + height / 2.0;
    let cand_bottom = candidate_y + height;

    let mut best_dx = snap_threshold + 1.0;
    let mut best_snap_x = None;
    let mut best_guide_x = None;

    let mut best_dy = snap_threshold + 1.0;
    let mut best_snap_y = None;
    let mut best_guide_y = None;

    for item in items {
        if &item.id == dragged_id {
            continue;
        }
        let r = item.effective_rect();
        let other_left = r.x;
        let other_mid_x = r.center_x();
        let other_right = r.right();

        let other_top = r.y;
        let other_mid_y = r.center_y();
        let other_bottom = r.bottom();

        let x_pairs = [
            (cand_left, other_left, other_left, other_left),
            (cand_left, other_right, other_right, other_right),
            (cand_right, other_left, other_left - width, other_left),
            (cand_right, other_right, other_right - width, other_right),
            (
                cand_mid_x,
                other_mid_x,
                other_mid_x - width / 2.0,
                other_mid_x,
            ),
        ];

        for (c_val, o_val, target_x, guide_x) in x_pairs {
            let dist = (c_val - o_val).abs();
            if dist <= snap_threshold && dist < best_dx {
                best_dx = dist;
                best_snap_x = Some(target_x);
                best_guide_x = Some(guide_x);
            }
        }

        let y_pairs = [
            (cand_top, other_top, other_top, other_top),
            (cand_top, other_bottom, other_bottom, other_bottom),
            (cand_bottom, other_top, other_top - height, other_top),
            (
                cand_bottom,
                other_bottom,
                other_bottom - height,
                other_bottom,
            ),
            (
                cand_mid_y,
                other_mid_y,
                other_mid_y - height / 2.0,
                other_mid_y,
            ),
        ];

        for (c_val, o_val, target_y, guide_y) in y_pairs {
            let dist = (c_val - o_val).abs();
            if dist <= snap_threshold && dist < best_dy {
                best_dy = dist;
                best_snap_y = Some(target_y);
                best_guide_y = Some(guide_y);
            }
        }
    }

    if let Some(sx) = best_snap_x {
        snapped_x = sx;
        if let Some(gx) = best_guide_x {
            guides.push(SnapGuide::Vertical(gx));
        }
    }

    if let Some(sy) = best_snap_y {
        snapped_y = sy;
        if let Some(gy) = best_guide_y {
            guides.push(SnapGuide::Horizontal(gy));
        }
    }

    SnapResult {
        snapped_x,
        snapped_y,
        guides,
    }
}
