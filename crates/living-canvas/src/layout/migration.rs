// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Legacy schema version 8 types and migration pathways into version 9.

use serde::{Deserialize, Serialize};

use crate::card::{CardGeometry, CardId, CardInstance, CardPresentation};
use crate::layout::engine::DesktopLayout;

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

/// Legacy `CanvasLayout` schema v8.
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

/// Migrate a legacy v8 layout into v9 format.
#[must_use]
pub fn from_v8(v8: &CanvasLayoutV8) -> DesktopLayout {
    let mut layout = DesktopLayout::new();
    let entries = [
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

    for (id, pt) in entries {
        let spec = id.spec();
        layout.cards.push(CardInstance {
            id,
            geometry: CardGeometry::new(pt.x, pt.y, spec.default_size, pt.z),
            presentation: CardPresentation::default(),
        });
    }

    layout
}
