// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Cognitive sleep/wake lifecycle scheduling and run management.
//!
//! Orchestrates the transition between Awake, Rest, Dreaming, and Consolidation modes,
//! ensuring background maintenance runs do not disturb interactive foreground attention.

use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

#[cfg(target_os = "linux")]
pub mod service;

/// High-level cognitive state of Mind.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LifecycleMode {
    /// Actively responding to user interaction.
    Awake,
    /// Low activity, ready to rest.
    Dozing,
    /// Offline associative consolidation.
    Dreaming,
    /// Minimal power / deep rest.
    DeepRest,
    /// Active memory consolidation.
    Consolidating,
    /// System maintenance.
    Maintenance,
    /// Interrupted by user activity.
    Interrupted,
}

impl LifecycleMode {
    /// String representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Awake => "awake",
            Self::Dozing => "dozing",
            Self::Dreaming => "dreaming",
            Self::DeepRest => "deep-rest",
            Self::Consolidating => "consolidating",
            Self::Maintenance => "maintenance",
            Self::Interrupted => "interrupted",
        }
    }
}

/// An active background maintenance or consolidation run.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleRun {
    /// Run identifier.
    pub run_id: Uuid,
    /// Run kind.
    pub kind: String,
    /// Scheduling policy ID.
    pub policy_id: String,
    /// Journal input sequence mark.
    pub input_high_water_mark: u64,
    /// Status.
    pub status: String,
}

/// Full lifecycle state snapshot.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleState {
    /// Current mode.
    pub mode: LifecycleMode,
    /// Timestamp of last user interaction.
    #[serde(with = "time::serde::rfc3339")]
    pub last_user_activity_at: OffsetDateTime,
    /// Active run, if any.
    pub run: Option<LifecycleRun>,
}

/// Errors occurring within the lifecycle organ.
#[derive(Debug, Error)]
pub enum LifecycleError {
    /// Invalid mode string.
    #[error("unknown lifecycle mode '{0}'")]
    UnknownMode(String),
    /// Lock poisoned.
    #[error("internal lock poisoned")]
    LockPoisoned,
}

/// Core domain logic of the lifecycle scheduler.
pub struct LifecycleCore {
    mode: RwLock<LifecycleMode>,
    last_user_activity: RwLock<OffsetDateTime>,
    active_run: RwLock<Option<LifecycleRun>>,
}

impl Default for LifecycleCore {
    fn default() -> Self {
        Self::new()
    }
}

impl LifecycleCore {
    /// Create a new LifecycleCore scheduler.
    #[must_use]
    pub fn new() -> Self {
        Self {
            mode: RwLock::new(LifecycleMode::Awake),
            last_user_activity: RwLock::new(OffsetDateTime::now_utc()),
            active_run: RwLock::new(None),
        }
    }

    /// Notify that user activity occurred, immediately returning mode to `Awake`.
    pub fn notify_user_activity(&self, _cause: &str, now: OffsetDateTime) {
        if let Ok(mut lock) = self.last_user_activity.write() {
            *lock = now;
        }
        if let Ok(mut mode_lock) = self.mode.write() {
            if *mode_lock != LifecycleMode::Awake {
                *mode_lock = LifecycleMode::Awake;
            }
        }
    }

    /// Manually transition lifecycle mode.
    pub fn transition(&self, mode: LifecycleMode) {
        if let Ok(mut lock) = self.mode.write() {
            *lock = mode;
        }
    }

    /// Retrieve full current lifecycle state.
    #[must_use]
    pub fn state(&self) -> LifecycleState {
        let mode = self.mode.read().map(|g| *g).unwrap_or(LifecycleMode::Awake);
        let last_user_activity_at = self
            .last_user_activity
            .read()
            .map(|g| *g)
            .unwrap_or_else(|_| OffsetDateTime::now_utc());
        let run = self.active_run.read().ok().and_then(|g| g.clone());

        LifecycleState {
            mode,
            last_user_activity_at,
            run,
        }
    }

    /// Current mode.
    #[must_use]
    pub fn mode(&self) -> LifecycleMode {
        self.mode.read().map(|g| *g).unwrap_or(LifecycleMode::Awake)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_activity_wakes_dozing_system() {
        let core = LifecycleCore::new();
        assert_eq!(core.mode(), LifecycleMode::Awake);

        core.transition(LifecycleMode::Dozing);
        assert_eq!(core.mode(), LifecycleMode::Dozing);

        let now = OffsetDateTime::now_utc();
        core.notify_user_activity("mouse-move", now);
        assert_eq!(core.mode(), LifecycleMode::Awake);
    }
}
