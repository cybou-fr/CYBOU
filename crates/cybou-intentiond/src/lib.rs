// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Commitments, obligations, and intention tracking.
//!
//! Answers "what did Mind promise to do, under what trigger, and what is still open?"

use std::sync::RwLock;

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

/// Errors occurring within the intentions engine.
#[derive(Debug, Error)]
pub enum IntentionError {
    /// Intention ID was not found among open commitments.
    #[error("no open intention with id {0}")]
    NotFound(Uuid),
    /// Lock poisoning error.
    #[error("internal lock poisoned")]
    LockPoisoned,
}

/// Core domain logic of the intentions organ.
pub struct IntentionCore {
    open_intentions: RwLock<Vec<Intention>>,
}

impl Default for IntentionCore {
    fn default() -> Self {
        Self::new()
    }
}

impl IntentionCore {
    /// Create a new IntentionCore manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            open_intentions: RwLock::new(Vec::new()),
        }
    }

    /// Form a new intention and add it to open obligations.
    pub fn form(
        &self,
        description: impl Into<String>,
        trigger: impl Into<String>,
        now: OffsetDateTime,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let intention = Intention {
            id,
            description: description.into(),
            trigger: trigger.into(),
            formed: now,
        };

        if let Ok(mut list) = self.open_intentions.write() {
            list.push(intention);
        }
        id
    }

    /// Close an open intention by resolution.
    ///
    /// # Errors
    ///
    /// Returns [`IntentionError::NotFound`] if the intention was not open.
    pub fn close(&self, intention_id: Uuid, _resolution: Resolution) -> Result<(), IntentionError> {
        let mut list = self
            .open_intentions
            .write()
            .map_err(|_| IntentionError::LockPoisoned)?;
        let initial_len = list.len();
        list.retain(|i| i.id != intention_id);
        if list.len() == initial_len {
            return Err(IntentionError::NotFound(intention_id));
        }
        Ok(())
    }

    /// Return all currently open intentions in order of formation.
    #[must_use]
    pub fn open(&self) -> Vec<Intention> {
        self.open_intentions
            .read()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// Number of open commitments.
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
    use super::*;

    #[test]
    fn intention_lifecycle() {
        let core = IntentionCore::new();
        assert_eq!(core.open_count(), 0);

        let now = OffsetDateTime::now_utc();
        let id = core.form("Respond to review comments", "on PR update", now);
        assert_eq!(core.open_count(), 1);

        let open = core.open();
        assert_eq!(open[0].id, id);
        assert_eq!(open[0].description, "Respond to review comments");

        core.close(id, Resolution::Fulfilled).expect("close ok");
        assert_eq!(core.open_count(), 0);
    }
}
