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

    /// Form a new intention obligation and return its UUID string.
    async fn form(&self, description: String, trigger: String, _cause_id: String) -> String {
        let now = OffsetDateTime::now_utc();
        let id = self.core.form(description, trigger, now);
        id.to_string()
    }

    /// Close an intention by resolution ("fulfilled", "abandoned", "obsolete").
    async fn close(&self, intention_id: String, resolution: String, _note: String) -> bool {
        let Ok(id) = Uuid::parse_str(&intention_id) else {
            return false;
        };
        let res = match resolution.to_lowercase().as_str() {
            "fulfilled" => Resolution::Fulfilled,
            "abandoned" => Resolution::Abandoned,
            _ => Resolution::Obsolete,
        };
        self.core.close(id, res).is_ok()
    }

    /// Return open intentions encoded as CBOR.
    async fn open(&self) -> Vec<u8> {
        let list = self.core.open();
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&list, &mut buf);
        buf
    }

    /// Return open intention count.
    async fn open_count(&self) -> u32 {
        self.core.open_count() as u32
    }
}
