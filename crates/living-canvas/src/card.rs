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
    /// Text, code, and configuration editor instance (ADR-0045).
    Editor(u32),
    /// Universal diff inspector and review panel (ADR-0045).
    Diff(u32),
    /// Universal contextual entity inspector panel (ADR-0046 §5).
    Inspector(u32),
    /// Server-owned background operations and tasks monitor.
    Operations(u32),
    /// Real-time attention, evidence, system, and agent notifications center.
    Notifications(u32),
    /// System services and daemons manager.
    Services(u32),
    /// Operating system processes manager.
    Processes(u32),
    /// Hardware telemetry and resource monitor.
    Monitor(u32),
    /// System and journald log viewer.
    SystemLogs(u32),
    /// Btrfs storage subvolumes and point-in-time snapshots manager.
    Storage(u32),
    /// Network interfaces, Wi-Fi, and VPN tunnels manager.
    Network(u32),
    /// Software package repository search and installation manager.
    Packages(u32),
    /// Governed system and kernel updates manager.
    Updates(u32),
    /// User accounts, PAM profile, and SSH authorized keys settings.
    UserSettings(u32),
    /// System sandboxing policy (Landlock, Bubblewrap, Seccomp) and security audit log.
    Security(u32),
    /// Automated Borg deduplicating backups and snapshot archives.
    Backup(u32),
    /// Personal electronic mail client.
    Mail(u32),
    /// Personal calendar and cognitive event scheduler.
    Calendar(u32),
    /// Personal Markdown knowledge notes with cognitive subject links.
    Notes(u32),
    /// Personal address book and subject contacts directory.
    Contacts(u32),
    /// Deep unified Cognitive Graph & causal DAG explorer.
    CognitiveGraph(u32),
    /// Canonical Event1 chronological journal viewer.
    EventJournal(u32),
    /// Deterministic Meaning1 natural language interpreter and qualified dialogue assistant.
    Meaning(u32),
    /// Lifelong learning candidate evaluation, artifact lineages, and capability governance.
    Learning(u32),
    /// Canvas Outline non-spatial accessibility tree view (ADR-0046 §22, §29).
    Outline,
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
            Self::Editor(_) => "editor",
            Self::Diff(_) => "diff",
            Self::Inspector(_) => "inspector",
            Self::Operations(_) => "operations",
            Self::Notifications(_) => "notifications",
            Self::Services(_) => "services",
            Self::Processes(_) => "processes",
            Self::Monitor(_) => "monitor",
            Self::SystemLogs(_) => "system-logs",
            Self::Storage(_) => "storage",
            Self::Network(_) => "network",
            Self::Packages(_) => "packages",
            Self::Updates(_) => "updates",
            Self::UserSettings(_) => "user-settings",
            Self::Security(_) => "security",
            Self::Backup(_) => "backup",
            Self::Mail(_) => "mail",
            Self::Calendar(_) => "calendar",
            Self::Notes(_) => "notes",
            Self::Contacts(_) => "contacts",
            Self::CognitiveGraph(_) => "cognitive-graph",
            Self::EventJournal(_) => "event-journal",
            Self::Meaning(_) => "meaning",
            Self::Learning(_) => "learning",
            Self::Outline => "outline",
        }
    }

    /// Stable key for this exact card instance.
    ///
    /// [`Self::key`] intentionally identifies a card type for legacy routes. Dynamic cards must
    /// use this key anywhere identity affects reconciliation, persistence, or membership.
    #[must_use]
    pub fn instance_key(self) -> String {
        match self {
            Self::Shell(instance)
            | Self::FileManager(instance)
            | Self::JournalFeed(instance)
            | Self::Editor(instance)
            | Self::Diff(instance)
            | Self::Inspector(instance)
            | Self::Operations(instance)
            | Self::Notifications(instance)
            | Self::Services(instance)
            | Self::Processes(instance)
            | Self::Monitor(instance)
            | Self::SystemLogs(instance)
            | Self::Storage(instance)
            | Self::Network(instance)
            | Self::Packages(instance)
            | Self::Updates(instance)
            | Self::UserSettings(instance)
            | Self::Security(instance)
            | Self::Backup(instance)
            | Self::Mail(instance)
            | Self::Calendar(instance)
            | Self::Notes(instance)
            | Self::Contacts(instance)
            | Self::CognitiveGraph(instance)
            | Self::EventJournal(instance)
            | Self::Meaning(instance)
            | Self::Learning(instance) => format!("{}:{instance}", self.key()),
            _ => self.key().to_string(),
        }
    }

    /// Whether a persisted identity names this exact instance.
    ///
    /// Type-only keys remain readable for layouts created before instance keys existed. New
    /// dynamic memberships must be written with [`Self::instance_key`].
    #[must_use]
    pub fn matches_persisted_key(self, persisted: &str) -> bool {
        persisted == self.instance_key() || (!persisted.contains(':') && persisted == self.key())
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
            Self::JournalFeed(_) => "Presence Stream",
            Self::Editor(_) => "Text Editor",
            Self::Diff(_) => "Diff Viewer",
            Self::Inspector(_) => "Universal Inspector",
            Self::Operations(_) => "Operations",
            Self::Notifications(_) => "Notifications",
            Self::Services(_) => "Services",
            Self::Processes(_) => "Processes",
            Self::Monitor(_) => "System Monitor",
            Self::SystemLogs(_) => "System Logs",
            Self::Storage(_) => "Storage & Snapshots",
            Self::Network(_) => "Network Connections",
            Self::Packages(_) => "Package Manager",
            Self::Updates(_) => "System Updates",
            Self::UserSettings(_) => "Users & SSH Keys",
            Self::Security(_) => "Security & Sandboxing",
            Self::Backup(_) => "Backup & Vault",
            Self::Mail(_) => "Mail & Messages",
            Self::Calendar(_) => "Calendar & Schedule",
            Self::Notes(_) => "Notes & Ideas",
            Self::Contacts(_) => "Contacts Directory",
            Self::CognitiveGraph(_) => "Cognitive Graph & Causal DAG",
            Self::EventJournal(_) => "Canonical Event1 Journal",
            Self::Meaning(_) => "Meaning & Dialogue Assistant",
            Self::Learning(_) => "Lifelong Learning & Governance",
            Self::Outline => "Canvas Outline",
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
            "editor" => Some(Self::Editor(0)),
            "diff" => Some(Self::Diff(0)),
            "inspector" => Some(Self::Inspector(0)),
            "operations" => Some(Self::Operations(0)),
            "notifications" => Some(Self::Notifications(0)),
            "services" => Some(Self::Services(0)),
            "processes" => Some(Self::Processes(0)),
            "monitor" => Some(Self::Monitor(0)),
            "system-logs" => Some(Self::SystemLogs(0)),
            "storage" => Some(Self::Storage(0)),
            "network" => Some(Self::Network(0)),
            "packages" => Some(Self::Packages(0)),
            "updates" => Some(Self::Updates(0)),
            "user-settings" | "users" => Some(Self::UserSettings(0)),
            "security" => Some(Self::Security(0)),
            "backup" => Some(Self::Backup(0)),
            "mail" => Some(Self::Mail(0)),
            "calendar" => Some(Self::Calendar(0)),
            "notes" => Some(Self::Notes(0)),
            "contacts" => Some(Self::Contacts(0)),
            "cognitive-graph" | "cognitive" => Some(Self::CognitiveGraph(0)),
            "event-journal" => Some(Self::EventJournal(0)),
            "meaning" | "assistant" | "dialogue" => Some(Self::Meaning(0)),
            "learning" | "skills" | "adaptation" => Some(Self::Learning(0)),
            "outline" => Some(Self::Outline),
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
            Self::Editor(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (560.0, 420.0),
                min_size: (360.0, 240.0),
                max_size: (1400.0, 1000.0),
            },
            Self::Diff(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (640.0, 440.0),
                min_size: (400.0, 260.0),
                max_size: (1600.0, 1100.0),
            },
            Self::Inspector(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (380.0, 480.0),
                min_size: (280.0, 320.0),
                max_size: (800.0, 900.0),
            },
            Self::Operations(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (560.0, 420.0),
                min_size: (360.0, 260.0),
                max_size: (1200.0, 900.0),
            },
            Self::Notifications(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (520.0, 440.0),
                min_size: (340.0, 260.0),
                max_size: (1000.0, 900.0),
            },
            Self::Services(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (580.0, 420.0),
                min_size: (360.0, 260.0),
                max_size: (1200.0, 900.0),
            },
            Self::Processes(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (600.0, 440.0),
                min_size: (380.0, 260.0),
                max_size: (1400.0, 1000.0),
            },
            Self::Monitor(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (560.0, 460.0),
                min_size: (380.0, 300.0),
                max_size: (1200.0, 900.0),
            },
            Self::SystemLogs(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (600.0, 420.0),
                min_size: (380.0, 260.0),
                max_size: (1400.0, 1000.0),
            },
            Self::Storage(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (580.0, 440.0),
                min_size: (380.0, 280.0),
                max_size: (1200.0, 900.0),
            },
            Self::Network(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (560.0, 420.0),
                min_size: (360.0, 260.0),
                max_size: (1200.0, 900.0),
            },
            Self::Packages(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (600.0, 460.0),
                min_size: (380.0, 280.0),
                max_size: (1400.0, 1000.0),
            },
            Self::Updates(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (560.0, 420.0),
                min_size: (360.0, 260.0),
                max_size: (1200.0, 900.0),
            },
            Self::UserSettings(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (580.0, 440.0),
                min_size: (380.0, 280.0),
                max_size: (1200.0, 900.0),
            },
            Self::Security(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (560.0, 440.0),
                min_size: (360.0, 280.0),
                max_size: (1200.0, 900.0),
            },
            Self::Backup(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (580.0, 460.0),
                min_size: (380.0, 280.0),
                max_size: (1200.0, 900.0),
            },
            Self::Mail(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (640.0, 480.0),
                min_size: (400.0, 300.0),
                max_size: (1400.0, 1000.0),
            },
            Self::Calendar(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (600.0, 460.0),
                min_size: (380.0, 280.0),
                max_size: (1400.0, 1000.0),
            },
            Self::Notes(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (580.0, 460.0),
                min_size: (360.0, 280.0),
                max_size: (1200.0, 900.0),
            },
            Self::Contacts(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (560.0, 440.0),
                min_size: (360.0, 280.0),
                max_size: (1200.0, 900.0),
            },
            Self::CognitiveGraph(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (680.0, 500.0),
                min_size: (420.0, 320.0),
                max_size: (1600.0, 1200.0),
            },
            Self::EventJournal(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (640.0, 460.0),
                min_size: (380.0, 280.0),
                max_size: (1400.0, 1000.0),
            },
            Self::Meaning(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (600.0, 480.0),
                min_size: (380.0, 280.0),
                max_size: (1400.0, 1000.0),
            },
            Self::Learning(_) => CardSpec {
                kind: CardKind::Tool,
                singleton: false,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (620.0, 480.0),
                min_size: (380.0, 280.0),
                max_size: (1400.0, 1000.0),
            },
            Self::Outline => CardSpec {
                kind: CardKind::Tool,
                singleton: true,
                movable: true,
                resizable: true,
                collapsible: true,
                closable: true,
                deckable: true,
                default_size: (300.0, 460.0),
                min_size: (220.0, 300.0),
                max_size: (600.0, 800.0),
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

/// Panel 2.0 representation tier for a Card instance (ADR-0044).
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelRepresentation {
    /// Standard regular working view (~360x260 px).
    #[default]
    Standard,
    /// Highly compact status chip (~220x70 px).
    Glance,
    /// Comprehensive forensic and in-depth view (~640x480 px).
    Expanded,
}

impl PanelRepresentation {
    /// Return human-readable label.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Glance => "Glance",
            Self::Expanded => "Expanded",
        }
    }

    /// Next representation in cycle.
    #[must_use]
    pub const fn cycle(&self) -> Self {
        match self {
            Self::Standard => Self::Expanded,
            Self::Expanded => Self::Glance,
            Self::Glance => Self::Standard,
        }
    }
}

/// Presentation mode of a Card instance.
///
/// Unknown fields are ignored on the way in, so a layout saved while the flag existed still loads.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct CardPresentation {
    /// Whether the card is collapsed into a single-line summary pill.
    pub collapsed: bool,
    /// Whether the card is pinned (locked against automatic arrangement).
    pub pinned: bool,
    /// Panel 2.0 representation tier (Standard, Glance, Expanded). Focus is a separate viewport mode.
    pub representation: PanelRepresentation,
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
                representation: PanelRepresentation::Standard,
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

    #[test]
    fn dynamic_card_instance_keys_do_not_alias() {
        assert_eq!(CardId::Editor(7).instance_key(), "editor:7");
        assert_ne!(
            CardId::Editor(7).instance_key(),
            CardId::Editor(8).instance_key()
        );
        assert_ne!(
            CardId::FileManager(7).instance_key(),
            CardId::Editor(7).instance_key()
        );
        assert_eq!(CardId::UserSettings(2).instance_key(), "user-settings:2");
        assert_eq!(CardId::Security(3).instance_key(), "security:3");
        assert_eq!(CardId::Backup(4).instance_key(), "backup:4");
        assert_eq!(CardId::Mail(5).instance_key(), "mail:5");
        assert_eq!(CardId::Calendar(6).instance_key(), "calendar:6");
        assert_eq!(CardId::Notes(7).instance_key(), "notes:7");
        assert_eq!(CardId::Contacts(8).instance_key(), "contacts:8");
        assert_eq!(CardId::CognitiveGraph(9).instance_key(), "cognitive-graph:9");
        assert_eq!(CardId::EventJournal(10).instance_key(), "event-journal:10");
        assert_eq!(CardId::Identity.instance_key(), "identity");
    }
}
