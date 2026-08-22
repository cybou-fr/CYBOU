// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Reusable Leptos component hierarchy for Living Canvas.

pub mod auth_modal;
pub mod card_controls;
pub mod card_frame;
pub mod cards;
pub mod command_palette;
pub mod deck;
pub mod dock;
pub mod icons;
pub mod minimap;
pub mod relations;
pub mod topbar;
pub mod viewport;

pub use auth_modal::{AuthModal, SignInView};
pub use card_controls::{CardControls, CardResizeHandle, DeckResizeHandle};
pub use card_frame::CardFrame;
pub use cards::*;
pub use command_palette::CommandPalette;
pub use deck::DeckContainerView;
pub use dock::DesktopDock;
pub use icons::*;
pub use minimap::Minimap;
pub use relations::{RelationshipEdge, RelationshipsLayer};
pub use topbar::Topbar;
pub use viewport::CanvasViewport;
