// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Commitments, obligations, and intention tracking.
//!
//! Answers "what did Mind promise to do, under what trigger, and what is still open?"
//! Backed by persistent state across daemon reboots.

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

/// An active or historical commitment formed by Mind.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Intention {
    /// Unique intention identity.
    pub id: Uuid,
    /// Human or system description of what was promised.
    pub description: String,
    /// Trigger condition or event under which it becomes active.
    pub trigger: String,
    /// Optional causal event ID that caused this intention.
    pub cause_id: Option<Uuid>,
    /// Formation timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub formed: OffsetDateTime,
}

/// How an intention was closed.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Resolution {
    /// Successfully fulfilled.
    Fulfilled,
    /// Explicitly abandoned.
    Abandoned,
    /// Superseded or obsolete.
    Obsolete,
}

/// Persistent intentions file schema.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct IntentionsState {
    version: u32,
    open: Vec<Intention>,
}

/// Errors occurring within the intentions engine.
#[derive(Debug, Error)]
pub enum IntentionError {
    /// Intention ID was not found among open commitments.
    #[error("no open intention with id {0}")]
    NotFound(Uuid),
    /// State file I/O failure.
    #[error("intention state i/o failed: {0}")]
    Io(#[from] std::io::Error),
    /// State file was corrupted.
    #[error("intention state file corrupted: {0}")]
    CorruptState(String),
    /// Lock poisoning error.
    #[error("internal lock poisoned")]
    LockPoisoned,
}

/// Core domain logic of the intentions organ.
pub struct IntentionCore {
    state_path: Option<PathBuf>,
    open_intentions: RwLock<Vec<Intention>>,
}

impl Default for IntentionCore {
    fn default() -> Self {
        Self::new()
    }
}

impl IntentionCore {
    /// Create a transient in-memory IntentionCore manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state_path: None,
            open_intentions: RwLock::new(Vec::new()),
        }
    }

    /// Open or initialize IntentionCore with a persistent JSON state file.
    ///
    /// # Errors
    ///
    /// Returns [`IntentionError`] on I/O or corrupted file structure (fail-closed).
    pub fn open(path: &Path) -> Result<Self, IntentionError> {
        let open_intentions = if path.exists() {
            let mut file = File::open(path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            let state: IntentionsState = serde_json::from_str(&content)
                .map_err(|e| IntentionError::CorruptState(e.to_string()))?;
            state.open
        } else {
            Vec::new()
        };

        Ok(Self {
            state_path: Some(path.to_path_buf()),
            open_intentions: RwLock::new(open_intentions),
        })
    }

    fn persist(&self) -> Result<(), IntentionError> {
        if let Some(path) = &self.state_path {
            let open = self
                .open_intentions
                .read()
                .map_err(|_| IntentionError::LockPoisoned)?
                .clone();
            let state = IntentionsState { version: 1, open };
            let serialized = serde_json::to_string_pretty(&state)
                .map_err(|e| IntentionError::CorruptState(e.to_string()))?;

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

    /// Form a new intention, add it to open obligations, and persist.
    pub fn form(
        &self,
        description: impl Into<String>,
        trigger: impl Into<String>,
        cause_id: Option<Uuid>,
        now: OffsetDateTime,
    ) -> Result<Uuid, IntentionError> {
        let id = Uuid::new_v4();
        let intention = Intention {
            id,
            description: description.into(),
            trigger: trigger.into(),
            cause_id,
            formed: now,
        };

        if let Ok(mut list) = self.open_intentions.write() {
            list.push(intention);
        }
        self.persist()?;
        Ok(id)
    }

    /// Close an open intention by resolution and persist.
    ///
    /// # Errors
    ///
    /// Returns [`IntentionError::NotFound`] if the intention was not open.
    pub fn close(
        &self,
        id: Uuid,
        _resolution: Resolution,
        _note: Option<&str>,
    ) -> Result<(), IntentionError> {
        let mut list = self
            .open_intentions
            .write()
            .map_err(|_| IntentionError::LockPoisoned)?;
        let pos = list
            .iter()
            .position(|i| i.id == id)
            .ok_or(IntentionError::NotFound(id))?;
        list.remove(pos);
        drop(list);

        self.persist()?;
        Ok(())
    }

    /// Return all currently open intentions.
    #[must_use]
    pub fn open_intentions(&self) -> Vec<Intention> {
        self.open_intentions
            .read()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// Total count of open intentions.
    #[must_use]
    pub fn open_count(&self) -> usize {
        self.open_intentions
            .read()
            .map(|g| g.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn intention_persistence_and_lifecycle() {
        let dir = tempdir().expect("tempdir");
        let state_path = dir.path().join("intentions.json");

        let core = IntentionCore::open(&state_path).expect("open");
        assert_eq!(core.open_count(), 0);

        let now = OffsetDateTime::now_utc();
        let cause = Uuid::new_v4();
        let id1 = core
            .form("Maintain backups", "on-idle", Some(cause), now)
            .expect("form");
        assert_eq!(core.open_count(), 1);

        // Reopen from disk: must survive restart!
        let reopened = IntentionCore::open(&state_path).expect("reopen");
        assert_eq!(reopened.open_count(), 1);
        let list = reopened.open_intentions();
        assert_eq!(list[0].id, id1);
        assert_eq!(list[0].cause_id, Some(cause));

        reopened
            .close(id1, Resolution::Fulfilled, Some("backup complete"))
            .expect("close");
        assert_eq!(reopened.open_count(), 0);

        let reclosed = IntentionCore::open(&state_path).expect("reclosed");
        assert_eq!(reclosed.open_count(), 0);
    }
}
