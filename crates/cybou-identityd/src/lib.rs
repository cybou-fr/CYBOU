// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Continuity of the subject across reboots and architectural change.
//!
//! Identity is not the database and not a random UUID. It is the fact that the
//! same subject persists across reboots and transitions, carrying its biography.

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::RwLock,
};

use cybou_protocol::canonical::CanonicalEnvelope;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

#[cfg(target_os = "linux")]
pub mod service;

/// Current active architecture version.
pub const ARCHITECTURE_VERSION: &str = "debian-rust-1.0";

/// Errors occurring within the identity organ.
#[derive(Debug, Error)]
pub enum IdentityError {
    /// The identity state file exists but is corrupted.
    #[error("identity state is present but unreadable: {0}")]
    CorruptState(String),
    /// File system or I/O error.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// File path.
        path: PathBuf,
        /// Underlying error.
        source: std::io::Error,
    },
    /// Internal lock poisoning.
    #[error("identity state lock poisoned")]
    LockPoisoned,
}

/// Persistent identity state snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityState {
    /// Unique subject identifier created once at birth.
    pub identity_id: Uuid,
    /// Instant when this identity was born.
    #[serde(with = "time::serde::rfc3339")]
    pub origin: OffsetDateTime,
    /// Number of times the system has started as this identity.
    pub session_count: u64,
    /// Architecture version that last wrote this state.
    pub architecture_version: String,
}

impl IdentityState {
    /// Create a brand new identity state at the current instant.
    #[must_use]
    pub fn new_birth(now: OffsetDateTime, architecture_version: impl Into<String>) -> Self {
        Self {
            identity_id: Uuid::new_v4(),
            origin: now,
            session_count: 1,
            architecture_version: architecture_version.into(),
        }
    }

    /// Check if the identity is valid (non-nil UUID).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.identity_id.is_nil()
    }

    /// Age of the identity in whole days.
    #[must_use]
    pub fn age_in_days(&self) -> i64 {
        let now = OffsetDateTime::now_utc();
        (now - self.origin).whole_days()
    }
}

/// Action taken during `begin_session`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionAction {
    /// First time run: identity was born.
    Born,
    /// Existing identity resumed regular session.
    Continued,
    /// Existing identity migrated from a previous architecture.
    Migrated {
        /// Previous architecture string.
        from: String,
        /// Target architecture string.
        to: String,
    },
}

/// Core domain logic and state manager for `Identity`.
pub struct IdentityCore {
    state_path: PathBuf,
    state: RwLock<Option<IdentityState>>,
    is_first_run: RwLock<bool>,
}

impl IdentityCore {
    /// Open the identity manager around the state file path (`identity.json`).
    #[must_use]
    pub fn open(state_path: impl AsRef<Path>) -> Self {
        Self {
            state_path: state_path.as_ref().to_path_buf(),
            state: RwLock::new(None),
            is_first_run: RwLock::new(false),
        }
    }

    /// Return the path to the state file.
    #[must_use]
    pub fn state_path(&self) -> &Path {
        &self.state_path
    }

    /// Read existing state from disk, or return None if file does not exist.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::CorruptState`] if the file exists but cannot be parsed.
    pub fn load_state(&self) -> Result<Option<IdentityState>, IdentityError> {
        if !self.state_path.exists() {
            return Ok(None);
        }
        let mut file = File::open(&self.state_path).map_err(|source| IdentityError::Io {
            path: self.state_path.clone(),
            source,
        })?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|source| IdentityError::Io {
                path: self.state_path.clone(),
                source,
            })?;

        let parsed: IdentityState = serde_json::from_str(&contents)
            .map_err(|err| IdentityError::CorruptState(err.to_string()))?;

        if !parsed.is_valid() {
            return Err(IdentityError::CorruptState(
                "identity state contains nil UUID".into(),
            ));
        }

        Ok(Some(parsed))
    }

    /// Save state atomically to disk via a temporary file and rename.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError::Io`] on write failure.
    pub fn save_state(&self, state: &IdentityState) -> Result<(), IdentityError> {
        if let Some(parent) = self.state_path.parent() {
            fs::create_dir_all(parent).map_err(|source| IdentityError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        let json = serde_json::to_string_pretty(state)
            .map_err(|err| IdentityError::CorruptState(err.to_string()))?;

        let temp_path = self.state_path.with_extension("tmp");
        {
            let mut file = File::create(&temp_path).map_err(|source| IdentityError::Io {
                path: temp_path.clone(),
                source,
            })?;
            file.write_all(json.as_bytes())
                .map_err(|source| IdentityError::Io {
                    path: temp_path.clone(),
                    source,
                })?;
            file.flush().map_err(|source| IdentityError::Io {
                path: temp_path.clone(),
                source,
            })?;
        }

        fs::rename(&temp_path, &self.state_path).map_err(|source| IdentityError::Io {
            path: self.state_path.clone(),
            source,
        })?;

        Ok(())
    }

    /// Begin a session: loads state or creates birth state, increments session counter, and saves.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityError`] if loading or saving fails.
    pub fn begin_session(
        &self,
        now: OffsetDateTime,
        arch_version: &str,
    ) -> Result<SessionAction, IdentityError> {
        let existing = self.load_state()?;

        let (action, new_state) = match existing {
            None => {
                let state = IdentityState::new_birth(now, arch_version);
                (SessionAction::Born, state)
            }
            Some(mut state) => {
                state.session_count += 1;
                if state.architecture_version != arch_version {
                    let from = state.architecture_version.clone();
                    state.architecture_version = arch_version.to_string();
                    (
                        SessionAction::Migrated {
                            from,
                            to: arch_version.to_string(),
                        },
                        state,
                    )
                } else {
                    (SessionAction::Continued, state)
                }
            }
        };

        self.save_state(&new_state)?;

        if let Ok(mut first_run) = self.is_first_run.write() {
            *first_run = action == SessionAction::Born;
        }

        if let Ok(mut lock) = self.state.write() {
            *lock = Some(new_state);
        }

        Ok(action)
    }

    /// Current identity state, if initialized.
    #[must_use]
    pub fn current_state(&self) -> Option<IdentityState> {
        self.state.read().ok().and_then(|g| g.clone())
    }

    /// Whether this run was a first-run birth.
    #[must_use]
    pub fn is_first_run(&self) -> bool {
        self.is_first_run.read().ok().map(|g| *g).unwrap_or(false)
    }

    /// Construct a canonical cognitive envelope to record this session action in Event1.
    #[must_use]
    pub fn build_envelope(
        &self,
        action: &SessionAction,
        now: OffsetDateTime,
        monotonic_time: u64,
    ) -> Option<CanonicalEnvelope> {
        let state = self.current_state()?;
        let (kind, payload_str) = match action {
            SessionAction::Born => (1, "identity created".to_string()),
            SessionAction::Continued => (1, format!("session {} began", state.session_count)),
            SessionAction::Migrated { from, to } => (
                12, // SelfAssessment
                format!("architecture changed from {from} to {to}, identity preserved"),
            ),
        };

        let mut payload = Vec::new();
        let _ = ciborium::into_writer(&payload_str, &mut payload);

        Some(CanonicalEnvelope {
            schema_version: 3,
            message_id: Uuid::new_v4(),
            correlation_id: state.identity_id, // the whole life is one episode
            causation_id: Uuid::nil(),
            origin_organ: "identityd".to_string(),
            origin_node: String::new(),
            kind,
            wall_time_ms: now.unix_timestamp_nanos() as i64 / 1_000_000,
            monotonic_time,
            logical_clock: 1,
            confidence: 1.0,
            evidence: vec![],
            payload,
            privacy: 1, // Node
            capability_scope: String::new(),
            sealed: false,
            key_domain_id: Uuid::nil(),
            key_epoch: 0,
            retention_class: 2,
            retention_policy_version: 0,
            retain_until_ms: 0,
            sensitivity: 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use time::OffsetDateTime;

    use super::*;

    #[test]
    fn identity_birth_and_continuation_lifecycle() {
        let dir = tempdir().expect("tempdir");
        let state_path = dir.path().join("identity.json");
        let core = IdentityCore::open(&state_path);

        let now = OffsetDateTime::now_utc();
        let action1 = core
            .begin_session(now, "arch-v1")
            .expect("begin birth session");
        assert_eq!(action1, SessionAction::Born);
        assert!(core.is_first_run());

        let state1 = core.current_state().expect("state exists");
        assert_eq!(state1.session_count, 1);
        assert_eq!(state1.architecture_version, "arch-v1");

        // Second run on same architecture -> Continued
        let action2 = core
            .begin_session(now, "arch-v1")
            .expect("continue session");
        assert_eq!(action2, SessionAction::Continued);
        assert!(!core.is_first_run());

        let state2 = core.current_state().expect("state exists");
        assert_eq!(state2.identity_id, state1.identity_id);
        assert_eq!(state2.origin, state1.origin);
        assert_eq!(state2.session_count, 2);

        // Third run on new architecture -> Migrated
        let action3 = core
            .begin_session(now, "arch-v2")
            .expect("migrate session");
        assert_eq!(
            action3,
            SessionAction::Migrated {
                from: "arch-v1".into(),
                to: "arch-v2".into(),
            }
        );
        let state3 = core.current_state().expect("state exists");
        assert_eq!(state3.identity_id, state1.identity_id);
        assert_eq!(state3.session_count, 3);
        assert_eq!(state3.architecture_version, "arch-v2");
    }

    #[test]
    fn corrupt_identity_state_fails_closed() {
        let dir = tempdir().expect("tempdir");
        let state_path = dir.path().join("identity.json");
        fs::write(&state_path, b"invalid json content").expect("write corrupt");

        let core = IdentityCore::open(&state_path);
        let err = core
            .begin_session(OffsetDateTime::now_utc(), "arch-v1")
            .expect_err("should refuse");
        assert!(matches!(err, IdentityError::CorruptState(_)));
    }
}
