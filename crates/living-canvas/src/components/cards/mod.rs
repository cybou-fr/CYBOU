// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Modular card and content components for CYBOU Living Canvas.

pub mod attention;
pub mod beliefs;
pub mod capabilities;
pub mod commitments;
pub mod content;
pub mod context;
pub mod file_manager;
pub mod identity;
pub mod journal;
pub mod journal_feed;
pub mod lifecycle;
pub mod perception;
pub mod self_model;
pub mod session;
pub mod shell;

pub use attention::{AttentionCard, AttentionContent};
pub use beliefs::{BeliefsCard, BeliefsContent};
pub use capabilities::{CapabilitiesCard, CapabilitiesContent};
pub use commitments::{CommitmentsCard, CommitmentsContent};
pub use content::CardContent;
pub use context::{ContextCard, ContextContent};
pub use file_manager::{FileManagerCard, FileManagerContent};
pub use identity::{IdentityCard, IdentityContent};
pub use journal::{JournalCard, JournalContent};
pub use journal_feed::{JournalFeedCard, JournalFeedContent};
pub use lifecycle::{LifecycleCard, LifecycleContent};
pub use perception::{PerceptionCard, PerceptionContent};
pub use self_model::{SelfModelCard, SelfModelContent};
pub use session::{SessionCard, SessionContent};
pub use shell::{ShellCard, ShellContent};
