// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Modular card and content components for CYBOU Living Canvas.

pub mod agents;
pub mod attention;
pub mod beliefs;
pub mod capabilities;
pub mod commitments;
pub mod content;
pub mod context;
pub mod diff;
pub mod disclosure;
pub mod editor;
pub mod file_manager;
pub mod identity;
pub mod insight;
pub mod journal;
pub mod journal_feed;
pub mod lifecycle;
pub mod perception;
pub mod self_model;
pub mod session;
pub mod shell;

pub use agents::{AgentsCard, AgentsContent};
pub use attention::{AttentionCard, AttentionContent};
pub use beliefs::{BeliefsCard, BeliefsContent};
pub use capabilities::{CapabilitiesCard, CapabilitiesContent};
pub use commitments::{CommitmentsCard, CommitmentsContent};
pub use content::CardContent;
pub use context::{ContextCard, ContextContent};
pub use diff::{DiffCard, DiffContent};
pub use disclosure::{DisclosureCard, DisclosureContent};
pub use editor::{EditorCard, EditorContent};
pub use file_manager::{FileManagerCard, FileManagerContent};
pub use identity::{IdentityCard, IdentityContent};
pub use insight::{InsightCard, InsightContent};
pub use journal::{JournalCard, JournalContent};
pub use journal_feed::{JournalFeedCard, JournalFeedContent};
pub use lifecycle::{LifecycleCard, LifecycleContent};
pub use perception::{PerceptionCard, PerceptionContent};
pub use self_model::{SelfModelCard, SelfModelContent};
pub use session::{SessionCard, SessionContent};
pub use shell::{ShellCard, ShellContent};
