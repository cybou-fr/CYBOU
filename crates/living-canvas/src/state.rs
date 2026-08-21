// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Runtime state and formatting helpers for Living Canvas.

use cybou_protocol::{CapabilityState, KnowledgeState};
use cybou_web_contracts::{Freshness, MindProjection, SessionMode, SessionProjection, SnapshotProjection};

/// High-level runtime connection and projection state.
#[derive(Clone, Debug)]
pub enum RuntimeState {
    /// Initializing connection to the Mind gateway.
    Loading,
    /// Connected with server-established session and projections.
    Ready {
        /// Gateway session mode (LocalDesktop, RemoteBrowser, PublicPreview).
        mode: SessionMode,
        /// Server-established session projection.
        session: SessionProjection,
        /// Current state snapshot projection.
        snapshot: SnapshotProjection,
        /// Full Mind owner projection if available.
        mind: Option<MindProjection>,
    },
    /// Connection or protocol error.
    Error(String),
}

/// Placeholder string for unread/withheld data fields.
#[must_use]
pub fn unread() -> String {
    "—".to_owned()
}

/// Human-readable label for a capability state.
#[must_use]
pub const fn capability_state_label(state: CapabilityState) -> &'static str {
    match state {
        CapabilityState::Available => "Available",
        CapabilityState::Unavailable => "Unavailable",
        CapabilityState::Unknown => "Unknown",
    }
}

/// Human-readable label for a knowledge state.
#[must_use]
pub const fn knowledge_label(state: KnowledgeState) -> &'static str {
    match state {
        KnowledgeState::Known => "Known",
        KnowledgeState::Unknown => "Unknown",
    }
}

/// Human-readable label for projection freshness.
#[must_use]
pub const fn freshness_label(state: Freshness) -> &'static str {
    match state {
        Freshness::Current => "Current",
        Freshness::Stale => "Stale",
        Freshness::Unknown => "Unknown freshness",
    }
}

/// Helper matching command palette queries.
#[must_use]
pub fn command_matches(query: &str, haystack: &str) -> bool {
    let query = query.trim().to_ascii_lowercase();
    query.is_empty() || haystack.contains(&query)
}

/// Match first matching panel for a command query.
#[must_use]
pub fn first_command_match(query: &str) -> Option<&'static str> {
    [
        ("capabilities", "capabilities health"),
        ("identity", "identity subject continuity"),
        ("session", "session trust mode"),
        ("journal", "journal contributions event1"),
        ("lifecycle", "lifecycle sleep wake"),
        ("commitments", "commitments obligations intention1"),
        ("self", "self assessment narration self1"),
        ("attention", "attention focus workspace1"),
        ("beliefs", "beliefs epistemic1 validity"),
        ("perception", "perception host observation"),
        ("context", "context association concepts context1"),
        ("shell", "shell terminal body capability"),
    ]
    .into_iter()
    .find_map(|(panel, label)| command_matches(query, label).then_some(panel))
}
