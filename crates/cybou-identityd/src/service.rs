// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Identity1` service implementation on zbus.

// The `#[interface]` expansion emits part of its dispatch surface with the attribute's own span,
// which an `allow` on the impl block cannot reach. Every handler written here is documented.
#![allow(missing_docs)]

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

#[allow(
    clippy::unused_async,
    reason = "zbus dispatches every exported handler as a future"
)]
#[interface(name = "org.cybou.Mind.Identity1")]
impl Identity1Service {
    /// Service readiness.
    ///
    /// Health1 probes this method on every organ; Identity1 answers with whether identity state
    /// actually loaded, because that is the fact the `identity-continuity` capability claims.
    async fn ready(&self) -> bool {
        self.core.current_state().is_some()
    }

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
        self.core.current_state().map_or(0, |s| s.session_count)
    }

    /// Age of the identity in whole days.
    async fn age_in_days(&self) -> i64 {
        self.core.current_state().map_or(0, |s| s.age_in_days())
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
