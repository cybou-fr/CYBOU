// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Generic Card model for CYBOU Desktop.
//!
//! Every visible surface in CYBOU Desktop is an instance of a `Card`. Cards can represent
//! system-level Mind projections (System cards), interactive tools (Tool cards, e.g. CYBOU Shell),
//! or temporary previews (Ephemeral cards).

use serde::{Deserialize, Serialize};

/// Stable identifier for a Card instance on the Desktop.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardId {
    /// Identity1 subject continuity projection.
    Identity,
    /// Established trust and gateway session mode.
    Session,
    /// Health1 capability dependency and health projection.
    Capabilities,
    /// Event1 canonical Journal feed and integrity.
    Journal,
    /// Lifecycle1 sleep/wake and consolidation state.
    Lifecycle,
    /// Intention1 open obligations and commitments.
    Commitments,
    /// Self1 autobiographical assessment and narration.
    SelfModel,
    /// Workspace1 Global Workspace Theory attention focus.
    Attention,
    /// Epistemic1 derived beliefs and validity.
    Beliefs,
    /// Perception1 host observations.
    Perception,
    /// Context1 associative context graph.
    Context,
    /// What this reader was supplied, and what was kept from them (ADR-0030 B1, B6).
    Disclosure,
    /// Telemetry1: what this host makes of itself, and what it would offer to do (ADR-0041 S0).
    Insight,
    /// Agent1: what is running in a capsule, on whose say-so, and with what left to spend.
    Agents,
    /// Dynamic bounded CYBOU Shell instance (Zone 3 `DemoReadOnly` capability).
    Shell(u32),
    /// Dynamic bounded File Manager instance (Zone 3 Read-Only storage).
    FileManager(u32),
    /// Real-time Journal event stream.
    JournalFeed(u32),
}

impl CardId {
    /// All 14 canonical System cards.
    pub const ALL_SYSTEM_CARDS: [Self; 14] = [
        Self::Identity,
        Self::Session,
        Self::Capabilities,
        Self::Journal,
        Self::Lifecycle,
        Self::Commitments,
        Self::SelfModel,
        Self::Attention,
        Self::Beliefs,
        Self::Perception,
        Self::Context,
        Self::Disclosure,
        Self::Insight,
        Self::Agents,
    ];

    /// Canonical string key for selection, routing, and legacy mapping.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Session => "session",
            Self::Capabilities => "capabilities",
            Self::Journal => "journal",
            Self::Lifecycle => "lifecycle",
            Self::Commitments => "commitments",
            Self::SelfModel => "self",
            Self::Attention => "attention",
            Self::Beliefs => "beliefs",
            Self::Perception => "perception",
            Self::Context => "context",
            Self::Disclosure => "disclosure",
            Self::Insight => "insight",
            Self::Agents => "agents",
            Self::Shell(_) => "shell",
            Self::FileManager(_) => "files",
            Self::JournalFeed(_) => "journal-feed",
        }
    }

    /// Human-readable title of the card.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Identity => "Identity",
            Self::Session => "Session",
            Self::Capabilities => "Capabilities",
            Self::Journal => "Journal",
            Self::Lifecycle => "Lifecycle",
            Self::Commitments => "Commitments",
            Self::SelfModel => "Self-assessment",
            Self::Attention => "Attention",
            Self::Beliefs => "Beliefs",
            Self::Perception => "Perception",
            Self::Context => "Context",
            Self::Disclosure => "Disclosure",
            Self::Insight => "System Insight",
            Self::Agents => "Agents",
            Self::Shell(_) => "Shell",
            Self::FileManager(_) => "File Manager",
            Self::JournalFeed(_) => "Event Stream",
        }
    }

    /// Resolve string key to static `CardId` if matching.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "identity" => Some(Self::Identity),
            "session" => Some(Self::Session),
            "capabilities" => Some(Self::Capabilities),
            "journal" => Some(Self::Journal),
            "lifecycle" => Some(Self::Lifecycle),
            "commitments" => Some(Self::Commitments),
            "self" => Some(Self::SelfModel),
            "attention" => Some(Self::Attention),
            "beliefs" => Some(Self::Beliefs),
            "perception" => Some(Self::Perception),
            "context" => Some(Self::Context),
            "disclosure" => Some(Self::Disclosure),
            "insight" => Some(Self::Insight),
            "agents" => Some(Self::Agents),
            "shell" => Some(Self::Shell(0)),
            "files" => Some(Self::FileManager(0)),
            "journal-feed" => Some(Self::JournalFeed(0)),
            _ => None,
        }
    }

    /// Whether this card is a permanent System Mind card.
    #[must_use]
    pub const fn is_system(self) -> bool {
        matches!(
            self,
            Self::Identity
                | Self::Session
                | Self::Capabilities
                | Self::Journal
                | Self::Lifecycle
                | Self::Commitments
                | Self::SelfModel
                | Self::Attention
                | Self::Beliefs
                | Self::Perception
                | Self::Context
                | Self::Disclosure
                | Self::Insight
                | Self::Agents
        )
    }

    /// Return static specification for this card type.
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub const fn spec(self) -> CardSpec {
        match self {
            Self::Identity => CardSpec {
                kind: CardKind::System,
                singleton: true,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: false,
                deckable: true,
                default_size: (220.0, 188.0),
                min_size: (180.0, 140.0),
                max_size: (450.0, 400.0),
            },
            Self::Session => CardSpec {
                kind: CardKind::System,
                singleton: true,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: false,
                deckable: true,
                default_size: (240.0, 236.0),
                min_size: (200.0, 160.0),
                max_size: (450.0, 450.0),
            },
            Self::Capabilities => CardSpec {
                kind: CardKind::System,
                singleton: true,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: false,
                deckable: true,
                default_size: (390.0, 294.0),
                min_size: (280.0, 200.0),
                max_size: (650.0, 600.0),
            },
            Self::Journal => CardSpec {
                kind: CardKind::System,
                singleton: true,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: false,
                deckable: true,
                default_size: (300.0, 285.0),
                min_size: (240.0, 180.0),
                max_size: (600.0, 600.0),
            },
            Self::Lifecycle => CardSpec {
                kind: CardKind::System,
                singleton: true,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: false,
                deckable: true,
                default_size: (335.0, 252.0),
                min_size: (260.0, 180.0),
                max_size: (550.0, 500.0),
            },
            Self::Commitments => CardSpec {
                kind: CardKind::System,
                singleton: true,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: false,
                deckable: true,
                default_size: (310.0, 184.0),
                min_size: (240.0, 140.0),
                max_size: (500.0, 400.0),
            },
            Self::SelfModel => CardSpec {
                kind: CardKind::System,
                singleton: true,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: false,
                deckable: true,
                default_size: (330.0, 210.0),
                min_size: (260.0, 150.0),
                max_size: (550.0, 450.0),
            },
            Self::Attention => CardSpec {
                kind: CardKind::System,
                singleton: true,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: false,
                deckable: true,
                default_size: (320.0, 170.0),
                min_size: (240.0, 130.0),
                max_size: (500.0, 400.0),
            },
            Self::Beliefs => CardSpec {
                kind: CardKind::System,
                singleton: true,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: false,
                deckable: true,
                default_size: (330.0, 260.0),
                min_size: (260.0, 180.0),
                max_size: (550.0, 550.0),
            },
            Self::Perception => CardSpec {
                kind: CardKind::System,
                singleton: true,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: false,
                deckable: true,
                default_size: (330.0, 170.0),
                min_size: (260.0, 140.0),
                max_size: (500.0, 400.0),
            },
            Self::Context => CardSpec {
                kind: CardKind::System,
                singleton: true,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: false,
                deckable: true,
                default_size: (330.0, 200.0),
                min_size: (260.0, 160.0),
                max_size: (550.0, 500.0),
            },
            // Wider than the other system cards, and not closable. What was withheld is a list of
            // subjects and reasons that has to stay readable as a list, and a surface a person can
            // dismiss is one they can be encouraged to dismiss.
            Self::Disclosure => CardSpec {
                kind: CardKind::System,
                singleton: true,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: false,
                deckable: true,
                default_size: (360.0, 260.0),
                min_size: (280.0, 180.0),
                max_size: (620.0, 620.0),
            },
            Self::Insight => CardSpec {
                kind: CardKind::System,
                singleton: true,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: false,
                deckable: true,
                // Larger than the rest by default. A finding carries its readings and its offers,
                // and a card that showed the headline with everything behind a scrollbar would be
                // a card whose whole reason for existing is one scroll away.
                default_size: (420.0, 340.0),
                min_size: (300.0, 200.0),
                max_size: (720.0, 720.0),
            },
            // Sized like Insight, and for the same reason: a session is a row of ceilings, a
            // countdown and a spend figure, and every one of them is the one somebody opened the
            // card to read.
            Self::Agents => CardSpec {
                kind: CardKind::System,
                singleton: true,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: false,
                deckable: true,
                default_size: (420.0, 320.0),
                min_size: (300.0, 200.0),
                max_size: (720.0, 720.0),
            },
            Self::Shell(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (440.0, 290.0),
                min_size: (320.0, 200.0),
                max_size: (800.0, 600.0),
            },
            Self::FileManager(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (560.0, 380.0),
                min_size: (360.0, 240.0),
                max_size: (1200.0, 800.0),
            },
            Self::JournalFeed(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: false,
                default_size: (580.0, 360.0),
                min_size: (360.0, 240.0),
                max_size: (1200.0, 800.0),
            },
        }
    }
}

/// Architectural class of a Card.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    /// Persistent Mind and cognitive projection surfaces.
    System,
    /// Interactive capabilities and tools (e.g. CYBOU Shell).
    Tool,
    /// Ephemeral preview, search result, or inspection card.
    Ephemeral,
}

/// Spatial geometry of a Card on the Desktop.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct CardGeometry {
    /// Left horizontal offset in pixels.
    pub x: f64,
    /// Top vertical offset in pixels.
    pub y: f64,
    /// Width in pixels.
    pub width: f64,
    /// Height in pixels.
    pub height: f64,
    /// Stacking order (z-index).
    pub z: u32,
}

impl CardGeometry {
    /// Construct geometry from coordinates and default size.
    #[must_use]
    pub const fn new(x: f64, y: f64, size: (f64, f64), z: u32) -> Self {
        Self {
            x,
            y,
            width: size.0,
            height: size.1,
            z,
        }
    }

    /// Center point (x, y) of the card rectangle.
    #[must_use]
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Clamp width and height within specified boundaries.
    #[must_use]
    pub fn clamp_size(&self, min: (f64, f64), max: (f64, f64)) -> Self {
        Self {
            width: self.width.clamp(min.0, max.0),
            height: self.height.clamp(min.1, max.1),
            ..*self
        }
    }
}

/// Presentation mode of a Card instance.
///
/// There was a `maximized` flag here until 2026-08-22. Nothing ever set it and nothing ever read
/// it: focus is [`DesktopViewMode::Focus`](crate::DesktopViewMode), which fills the viewport
/// without touching the geometry underneath and restores it on `Escape`. Two fields that could
/// each answer "is this card filling the screen?" is one field too many, and the one that was
/// persisted was the one that never knew.
///
/// Unknown fields are ignored on the way in, so a layout saved while the flag existed still loads.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct CardPresentation {
    /// Whether the card is collapsed into a single-line summary pill.
    pub collapsed: bool,
    /// Whether the card is pinned (locked against automatic arrangement).
    pub pinned: bool,
}

/// Static capabilities and bounds of a Card type.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CardSpec {
    /// The architectural category of this card.
    pub kind: CardKind,
    /// Only one instance may exist if true.
    pub singleton: bool,
    /// Can be repositioned spatially.
    pub movable: bool,
    /// Can be resized by the user.
    pub resizable: bool,
    /// Can be collapsed into a compact header pill.
    pub collapsible: bool,
    /// Can be closed without destroying canonical state.
    pub closable: bool,
    /// Can be grouped into tabbed Decks.
    pub deckable: bool,
    /// Default width and height.
    pub default_size: (f64, f64),
    /// Minimum width and height constraint.
    pub min_size: (f64, f64),
    /// Maximum width and height constraint.
    pub max_size: (f64, f64),
}

/// Complete runtime representation of a Card instance on the Desktop.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct CardInstance {
    /// Unique identifier.
    pub id: CardId,
    /// Spatial geometry.
    pub geometry: CardGeometry,
    /// Presentation state.
    pub presentation: CardPresentation,
}

impl CardInstance {
    /// Construct a new `CardInstance` at point (x, y) with default spec size.
    #[must_use]
    pub const fn new(id: CardId, x: f64, y: f64, z: u32) -> Self {
        let spec = id.spec();
        Self {
            id,
            geometry: CardGeometry::new(x, y, spec.default_size, z),
            presentation: CardPresentation {
                collapsed: false,
                pinned: false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_system_card_survives_a_round_trip_through_its_key() {
        // The key is what a saved layout and a command palette both name a card by. A card whose
        // key does not resolve back is a card that silently disappears from a restored desktop.
        for card_id in CardId::ALL_SYSTEM_CARDS {
            assert_eq!(
                CardId::from_key(card_id.key()),
                Some(card_id),
                "{} did not survive its key",
                card_id.title()
            );
            assert!(
                card_id.is_system(),
                "{} is not a system card",
                card_id.key()
            );
        }
    }

    #[test]
    fn a_layout_saved_before_a_card_existed_gains_it_rather_than_losing_the_card() {
        // Disclosure was added after layouts had already been saved, and Insight after that. A
        // desktop restored from an older one must end up with the card, or the surface exists and
        // nobody sees it.
        //
        // Written over every system card rather than the one that prompted it: the next card added
        // will be the next one that could go missing, and a test naming a single card would pass
        // while it did.
        for card_id in CardId::ALL_SYSTEM_CARDS {
            let mut older = crate::DesktopLayout::canonical(None);
            older.cards.retain(|card| card.id != card_id);
            assert!(!older.cards.iter().any(|card| card.id == card_id));

            older.validate_and_normalize();
            assert!(
                older.cards.iter().any(|card| card.id == card_id),
                "a desktop restored without {} never got it back",
                card_id.title()
            );
        }
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn card_spec_defaults_and_keys() {
        for card_id in CardId::ALL_SYSTEM_CARDS {
            let spec = card_id.spec();
            assert_eq!(spec.kind, CardKind::System);
            assert!(spec.singleton);
            assert!(spec.movable);
            assert!(spec.resizable);
            assert!(spec.collapsible);
            assert!(!spec.closable);
            assert!(spec.deckable);

            assert!(spec.default_size.0 >= spec.min_size.0);
            assert!(spec.default_size.1 >= spec.min_size.1);
            assert!(spec.default_size.0 <= spec.max_size.0);
            assert!(spec.default_size.1 <= spec.max_size.1);

            let key = card_id.key();
            assert_eq!(CardId::from_key(key), Some(card_id));
        }
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn geometry_center_and_clamp() {
        let geom = CardGeometry::new(100.0, 200.0, (300.0, 150.0), 5);
        assert_eq!(geom.center(), (250.0, 275.0));

        let clamped = geom.clamp_size((320.0, 100.0), (400.0, 120.0));
        assert_eq!(clamped.width, 320.0);
        assert_eq!(clamped.height, 120.0);
    }
}
