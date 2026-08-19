// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Cognitive sleep/wake lifecycle scheduling and run management.
//!
//! Orchestrates the transition between Awake, Rest, Dreaming, and Consolidation modes,
//! ensuring background maintenance runs do not disturb interactive foreground attention.
//! State is persistently stored across reboots.

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::RwLock,
};

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
    /// File I/O error.
    #[error("lifecycle state i/o error: {0}")]
    Io(#[from] std::io::Error),
    /// State file corrupted.
    #[error("lifecycle state corrupted: {0}")]
    CorruptState(String),
    /// Lock poisoned.
    #[error("internal lock poisoned")]
    LockPoisoned,
}

/// Core domain logic of the lifecycle scheduler.
pub struct LifecycleCore {
    state_path: Option<PathBuf>,
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
    /// Create a transient LifecycleCore scheduler.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state_path: None,
            mode: RwLock::new(LifecycleMode::Awake),
            last_user_activity: RwLock::new(OffsetDateTime::now_utc()),
            active_run: RwLock::new(None),
        }
    }

    /// Open or initialize LifecycleCore with a persistent JSON state file.
    ///
    /// # Errors
    ///
    /// Returns [`LifecycleError`] on I/O or corrupted file structure (fail-closed).
    pub fn open(path: &Path) -> Result<Self, LifecycleError> {
        let (mode, last_user_activity, active_run) = if path.exists() {
            let mut file = File::open(path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            let state: LifecycleState = serde_json::from_str(&content)
                .map_err(|e| LifecycleError::CorruptState(e.to_string()))?;
            (state.mode, state.last_user_activity_at, state.run)
        } else {
            (LifecycleMode::Awake, OffsetDateTime::now_utc(), None)
        };

        Ok(Self {
            state_path: Some(path.to_path_buf()),
            mode: RwLock::new(mode),
            last_user_activity: RwLock::new(last_user_activity),
            active_run: RwLock::new(active_run),
        })
    }

    fn persist(&self) -> Result<(), LifecycleError> {
        if let Some(path) = &self.state_path {
            let state = self.state();
            let serialized = serde_json::to_string_pretty(&state)
                .map_err(|e| LifecycleError::CorruptState(e.to_string()))?;

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let temp_path = path.with_extension("tmp");
            {
                let mut temp_file = File::create(&temp_path)?;
                temp_file.write_all(serialized.as_bytes())?;
                temp_file.sync_all()?;
            }
            fs::rename(&temp_path, path)?;
        }
        Ok(())
    }

    /// Notify that user activity occurred, immediately returning mode to `Awake` (durable before visible).
    pub fn notify_user_activity(&self, _cause: &str, now: OffsetDateTime) -> Result<(), LifecycleError> {
        let prev_mode = self.mode();
        let prev_act = self.last_user_activity.read().map(|g| *g).unwrap_or(now);

        if let Ok(mut lock) = self.last_user_activity.write() {
            *lock = now;
        }
        if let Ok(mut mode_lock) = self.mode.write() {
            *mode_lock = LifecycleMode::Awake;
        }

        if let Err(e) = self.persist() {
            // Rollback in-memory state on persistence failure
            if let Ok(mut lock) = self.last_user_activity.write() {
                *lock = prev_act;
            }
            if let Ok(mut mode_lock) = self.mode.write() {
                *mode_lock = prev_mode;
            }
            return Err(e);
        }
        Ok(())
    }

    /// Manually transition lifecycle mode (durable before visible).
    pub fn transition(&self, mode: LifecycleMode) -> Result<(), LifecycleError> {
        let prev_mode = self.mode();
        if let Ok(mut lock) = self.mode.write() {
            *lock = mode;
        }

        if let Err(e) = self.persist() {
            // Rollback in-memory state on persistence failure
            if let Ok(mut lock) = self.mode.write() {
                *lock = prev_mode;
            }
            return Err(e);
        }
        Ok(())
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
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn lifecycle_persistence_and_recovery() {
        let dir = tempdir().expect("tempdir");
        let state_path = dir.path().join("lifecycle.json");

        let core = LifecycleCore::open(&state_path).expect("open");
        assert_eq!(core.mode(), LifecycleMode::Awake);

        core.transition(LifecycleMode::Dozing).expect("transition success");
        assert_eq!(core.mode(), LifecycleMode::Dozing);

        // Reopen from disk: must survive restart
        let reopened = LifecycleCore::open(&state_path).expect("reopen");
        assert_eq!(reopened.mode(), LifecycleMode::Dozing);

        let now = OffsetDateTime::now_utc();
        reopened.notify_user_activity("mouse-move", now).expect("notify success");
        assert_eq!(reopened.mode(), LifecycleMode::Awake);

        let reclosed = LifecycleCore::open(&state_path).expect("reclosed");
        assert_eq!(reclosed.mode(), LifecycleMode::Awake);
    }
}
