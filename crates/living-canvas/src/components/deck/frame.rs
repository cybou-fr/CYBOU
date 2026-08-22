// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Deck container frame styling and spatial interaction logic.

use crate::{DesktopItemId, DesktopLayout, DesktopViewMode};

/// Compute CSS style string for a deck container.
#[must_use]
pub fn compute_deck_style(
    deck_id: &str,
    layout: DesktopLayout,
    view_mode: DesktopViewMode,
) -> String {
    if view_mode == DesktopViewMode::Focus(DesktopItemId::Deck(deck_id.to_string())) {
        "position: fixed; left: 20px; top: 20px; width: calc(100vw - 40px); height: calc(100vh - 100px); z-index: 9999; box-shadow: 0 0 0 9999px rgba(0,0,0,0.65);".to_string()
    } else if let Some(deck) = layout.deck(deck_id) {
        let geom = deck.geometry;
        let h = if deck.presentation.collapsed {
            44.0
        } else {
            geom.height
        };
        format!(
            "transform: translate3d({:.1}px, {:.1}px, 0); width: {:.1}px; height: {:.1}px; z-index: {};",
            geom.x, geom.y, geom.width, h, geom.z
        )
    } else {
        String::new()
    }
}
