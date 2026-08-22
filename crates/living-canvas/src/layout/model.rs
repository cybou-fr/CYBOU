// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Core 2D geometry models, viewports, and item identifiers for Living Canvas.

use serde::{Deserialize, Serialize};

use crate::card::{CardGeometry, CardId, CardPresentation};

/// 2D bounding rectangle in Desktop world coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
    /// Width in pixels.
    pub width: f64,
    /// Height in pixels.
    pub height: f64,
}

impl Rect {
    /// Construct a new Rect.
    #[must_use]
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if this rectangle overlaps with another rectangle.
    #[must_use]
    pub fn intersects(&self, other: &Self) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    /// Right coordinate.
    #[must_use]
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// Bottom coordinate.
    #[must_use]
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// Center X coordinate.
    #[must_use]
    pub fn center_x(&self) -> f64 {
        self.x + self.width / 2.0
    }

    /// Center Y coordinate.
    #[must_use]
    pub fn center_y(&self) -> f64 {
        self.y + self.height / 2.0
    }
}

/// Usable canvas viewport dimensions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UsableViewport {
    /// Visible width in pixels.
    pub width: f64,
    /// Visible height in pixels.
    pub height: f64,
}

impl Default for UsableViewport {
    fn default() -> Self {
        Self {
            width: 1440.0,
            height: 900.0,
        }
    }
}

/// Top-level desktop spatial item identifier.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum DesktopItemId {
    /// Standalone Card instance.
    Card(CardId),
    /// Grouped Deck container.
    Deck(String),
}

/// Active top-level desktop spatial surface (Card or Deck).
#[derive(Clone, Debug, PartialEq)]
pub struct DesktopItem {
    /// Stable item identifier.
    pub id: DesktopItemId,
    /// Spatial geometry.
    pub geometry: CardGeometry,
    /// Presentation state.
    pub presentation: CardPresentation,
}

impl DesktopItem {
    /// Compute the effective 2D bounding rectangle, respecting collapsed state.
    #[must_use]
    pub fn effective_rect(&self) -> Rect {
        let h = if self.presentation.collapsed {
            44.0
        } else {
            self.geometry.height
        };
        Rect::new(self.geometry.x, self.geometry.y, self.geometry.width, h)
    }

    /// Effective height (44.0 when collapsed).
    #[must_use]
    pub fn effective_height(&self) -> f64 {
        if self.presentation.collapsed {
            44.0
        } else {
            self.geometry.height
        }
    }

    /// Check if item is pinned.
    #[must_use]
    pub fn is_pinned(&self) -> bool {
        self.presentation.pinned
    }
}

/// Desktop automated arrangement algorithms.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArrangementMode {
    /// Free-form spatial layout (manual drag and drop).
    Free,
    /// Structured multi-track column grid.
    Grid,
    /// Dense skyline bin-packing.
    Compact,
    /// Cognitive organ dependencies and relational graph.
    Relations,
    /// Canonical Home default arrangement.
    Home,
}

/// Desktop viewport display mode.
#[derive(Clone, Debug, PartialEq)]
pub enum DesktopViewMode {
    /// Full infinite 2D spatial canvas.
    Spatial,
    /// One item filling the viewport.
    ///
    /// Non-destructive: the persisted geometry underneath is untouched, and `Escape` restores the
    /// desktop as it was. This is the only place focus is recorded — `CardPresentation` used to
    /// carry a `maximized` flag as well, which nothing set and nothing read.
    Focus(DesktopItemId),
}
