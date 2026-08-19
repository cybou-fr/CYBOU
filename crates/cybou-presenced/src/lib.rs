// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Unified Mind presentation, compound projection, and user command gateway.
//!
//! Provides the primary read-model projection and transactional user mutation
//! dispatch across the Mind organ graph for Living Canvas and desktop shells.

use std::sync::RwLock;

use cybou_protocol::{
    CapabilityState, KnowledgeState,
    capability::CapabilityRegistry,
};
use cybou_web_contracts::{CapabilityProjection, Freshness, SnapshotProjection, WEB_SCHEMA_V1};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[cfg(target_os = "linux")]
pub mod service;

/// High-level presentation state snapshot.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenceSnapshot {
    /// Projection snapshot.
    pub snapshot: SnapshotProjection,
    /// Narration string if available.
    pub narration: String,
    /// Awake status.
    pub is_awake: bool,
}

/// Errors occurring in the presence organ.
#[derive(Debug, Error)]
pub enum PresenceError {
    /// Internal lock poisoned.
    #[error("presence lock poisoned")]
    LockPoisoned,
}

/// Core domain logic of the Presence organ.
pub struct PresenceCore {
    is_awake: RwLock<bool>,
    narration: RwLock<String>,
}

impl Default for PresenceCore {
    fn default() -> Self {
        Self::new()
    }
}

impl PresenceCore {
    /// Create a new PresenceCore engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            is_awake: RwLock::new(true),
            narration: RwLock::new("This is my first day.".to_string()),
        }
    }

    /// Whether Mind is currently awake.
    #[must_use]
    pub fn is_awake(&self) -> bool {
        self.is_awake.read().map(|g| *g).unwrap_or(true)
    }

    /// Set awake state.
    pub fn set_awake(&self, awake: bool) {
        if let Ok(mut lock) = self.is_awake.write() {
            *lock = awake;
        }
    }

    /// Current self narrative text.
    #[must_use]
    pub fn narration(&self) -> String {
        self.narration
            .read()
            .map(|g| g.clone())
            .unwrap_or_else(|_| "This is my first day.".to_string())
    }

    /// Update narration string.
    pub fn set_narration(&self, narration: impl Into<String>) {
        if let Ok(mut lock) = self.narration.write() {
            *lock = narration.into();
        }
    }

    /// Build a default nominal snapshot projection.
    #[must_use]
    pub fn build_snapshot(&self, now: OffsetDateTime) -> SnapshotProjection {
        let mut caps = Vec::new();
        for decl in CapabilityRegistry::capabilities() {
            caps.push(CapabilityProjection {
                id: decl.capability_id.to_string(),
                state: CapabilityState::Available,
                knowledge: KnowledgeState::Known,
                freshness: Freshness::Current,
                reason: None,
            });
        }

        SnapshotProjection {
            schema_version: WEB_SCHEMA_V1,
            projection_version: 1,
            cursor: "0".into(),
            observed_at: now.format(&Rfc3339).unwrap_or_default(),
            freshness: Freshness::Current,
            knowledge: KnowledgeState::Known,
            capabilities: caps,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nominal_presence_snapshot() {
        let core = PresenceCore::new();
        let now = OffsetDateTime::now_utc();
        let snap = core.build_snapshot(now);
        assert_eq!(snap.capabilities.len(), 11);
        assert_eq!(core.is_awake(), true);
    }
}
