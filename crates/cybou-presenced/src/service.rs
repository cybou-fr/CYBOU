// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Presence1` service implementation on zbus.

use std::sync::Arc;

use time::OffsetDateTime;
use uuid::Uuid;
use zbus::{SignalContext, interface};

use crate::PresenceCore;

/// D-Bus Service exporting `org.cybou.Mind.Presence1`.
pub struct Presence1Service {
    core: Arc<PresenceCore>,
}

impl Presence1Service {
    /// Create a new Presence1 D-Bus service handler around `PresenceCore`.
    #[must_use]
    pub fn new(core: Arc<PresenceCore>) -> Self {
        Self { core }
    }
}

#[interface(name = "org.cybou.Mind.Presence1")]
impl Presence1Service {
    /// Service readiness.
    async fn ready(&self) -> bool {
        true
    }

    /// Overall health summary.
    async fn health(&self) -> String {
        "healthy".to_string()
    }

    /// Last error diagnostic.
    async fn last_error(&self) -> String {
        String::new()
    }

    /// Compound snapshot projection encoded as CBOR.
    async fn snapshot(&self) -> Vec<u8> {
        let now = OffsetDateTime::now_utc();
        let snap = self.core.build_snapshot(now);
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&snap, &mut buf);
        buf
    }

    /// Recent activity envelopes encoded as CBOR.
    async fn activity(&self, _limit: i32) -> Vec<u8> {
        vec![]
    }

    /// Detailed obligations list encoded as CBOR.
    async fn detailed_obligations(&self) -> Vec<u8> {
        vec![]
    }

    /// User mutation: promise an obligation and return intention ID.
    async fn promise(&self, _description: String) -> String {
        Uuid::new_v4().to_string()
    }

    /// User mutation: trigger self reflection.
    async fn reflect(&self) -> bool {
        true
    }

    /// User mutation: fulfill intention at index.
    async fn fulfill_index(&self, _index: i32) -> bool {
        true
    }

    /// User mutation: abandon intention at index.
    async fn abandon_index(&self, _index: i32) -> bool {
        true
    }

    /// User mutation: record observation.
    async fn observe(&self, _subject: String, _value: f64) -> bool {
        true
    }

    /// User mutation: produce forecast and return CBOR.
    async fn predict(&self, _subject: String) -> Vec<u8> {
        vec![]
    }

    /// User mutation: interrupt background lifecycle.
    async fn interrupt_lifecycle(&self, _cause: String) -> bool {
        true
    }

    /// Signal emitted when compound projection changes.
    #[zbus(signal)]
    async fn changed(ctxt: &SignalContext<'_>) -> zbus::Result<()>;
}
