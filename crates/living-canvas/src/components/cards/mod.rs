// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Modular card components for Living Canvas.

pub mod attention;
pub mod beliefs;
pub mod capabilities;
pub mod commitments;
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

pub use attention::AttentionCard;
pub use beliefs::BeliefsCard;
pub use capabilities::CapabilitiesCard;
pub use commitments::CommitmentsCard;
pub use context::ContextCard;
pub use file_manager::FileManagerCard;
pub use identity::IdentityCard;
pub use journal::JournalCard;
pub use journal_feed::JournalFeedCard;
pub use lifecycle::LifecycleCard;
pub use perception::PerceptionCard;
pub use self_model::SelfModelCard;
pub use session::SessionCard;
pub use shell::ShellCard;
