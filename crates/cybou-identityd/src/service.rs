// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Identity1` service implementation on zbus.

use std::sync::Arc;

use time::format_description::well_known::Rfc3339;
use zbus::interface;

use crate::IdentityCore;

/// D-Bus Service exporting `org.cybou.Mind.Identity1`.
pub struct Identity1Service {
    core: Arc<IdentityCore>,
}

impl Identity1Service {
    /// Create a new Identity1 D-Bus service handler around `IdentityCore`.
    #[must_use]
    pub fn new(core: Arc<IdentityCore>) -> Self {
        Self { core }
    }
}

#[interface(name = "org.cybou.Mind.Identity1")]
impl Identity1Service {
    /// Unique subject UUID string.
    async fn identity_id(&self) -> String {
        self.core
            .current_state()
            .map(|s| s.identity_id.to_string())
            .unwrap_or_default()
    }

    /// Origin creation timestamp in RFC3339.
    async fn origin(&self) -> String {
        self.core
            .current_state()
            .and_then(|s| s.origin.format(&Rfc3339).ok())
            .unwrap_or_default()
    }

    /// Monotonic session count.
    async fn session_count(&self) -> u64 {
        self.core
            .current_state()
            .map(|s| s.session_count)
            .unwrap_or(0)
    }

    /// Age of the identity in whole days.
    async fn age_in_days(&self) -> i64 {
        self.core
            .current_state()
            .map(|s| s.age_in_days())
            .unwrap_or(0)
    }

    /// Active architecture version.
    async fn architecture_version(&self) -> String {
        self.core
            .current_state()
            .map(|s| s.architecture_version)
            .unwrap_or_default()
    }

    /// Whether this run created the identity (first run).
    async fn is_first_run(&self) -> bool {
        self.core.is_first_run()
    }

    /// Serialized CBOR representation of the identity state.
    async fn state(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        if let Some(state) = self.core.current_state() {
            let _ = ciborium::into_writer(&state, &mut buf);
        }
        buf
    }
}
