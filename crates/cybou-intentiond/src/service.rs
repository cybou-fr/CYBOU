// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Intention1` service implementation on zbus.

use std::sync::Arc;

use time::OffsetDateTime;
use uuid::Uuid;
use zbus::interface;

use crate::{IntentionCore, Resolution};

/// D-Bus Service exporting `org.cybou.Mind.Intention1`.
pub struct Intention1Service {
    core: Arc<IntentionCore>,
}

impl Intention1Service {
    /// Create a new Intention1 D-Bus service handler around `IntentionCore`.
    #[must_use]
    pub fn new(core: Arc<IntentionCore>) -> Self {
        Self { core }
    }
}

#[interface(name = "org.cybou.Mind.Intention1")]
impl Intention1Service {
    /// Service readiness.
    async fn ready(&self) -> bool {
        true
    }

    /// Form a new intention obligation and return its UUID string (rejects invalid cause UUIDs).
    async fn form(&self, description: String, trigger: String, cause_id: String) -> String {
        let now = OffsetDateTime::now_utc();
        let cause = if cause_id.is_empty() {
            None
        } else {
            match Uuid::parse_str(&cause_id) {
                Ok(id) => Some(id),
                Err(_) => return String::new(), // reject invalid cause
            }
        };
        match self.core.form(description, trigger, cause, now) {
            Ok(id) => id.to_string(),
            Err(_) => String::new(),
        }
    }

    /// Close an intention by resolution ("fulfilled", "abandoned", "obsolete") or reject.
    async fn close(&self, intention_id: String, resolution: String, note: String) -> bool {
        let Ok(id) = Uuid::parse_str(&intention_id) else {
            return false;
        };
        let res = match resolution.to_lowercase().as_str() {
            "fulfilled" => Resolution::Fulfilled,
            "abandoned" => Resolution::Abandoned,
            "obsolete" => Resolution::Obsolete,
            _ => return false, // reject unknown resolution
        };
        let note_opt = if note.is_empty() {
            None
        } else {
            Some(note.as_str())
        };
        self.core.close(id, res, note_opt).is_ok()
    }

    /// Return open intentions encoded as CBOR.
    async fn open(&self) -> Vec<u8> {
        let list = self.core.open_intentions();
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&list, &mut buf);
        buf
    }

    /// Return open intention count.
    async fn open_count(&self) -> u32 {
        self.core.open_count() as u32
    }
}
