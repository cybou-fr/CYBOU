// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Presence1` service implementation on zbus.

// The `#[interface]` expansion emits part of its dispatch surface with the attribute's own span,
// which an `allow` on the impl block cannot reach. Every handler written here is documented, and
// the generated dispatch reads parameters that the fail-closed stubs below deliberately ignore.
#![allow(clippy::used_underscore_binding, missing_docs)]

use std::sync::Arc;

use cybou_fabric::PRESENCE;
use time::OffsetDateTime;
use zbus::{interface, object_server::SignalEmitter};

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

#[allow(
    clippy::unused_async,
    reason = "zbus dispatches every exported handler as a future"
)]
#[interface(name = "org.cybou.Mind.Presence1")]
impl Presence1Service {
    /// Service readiness.
    async fn ready(&self) -> bool {
        true
    }

    /// Overall health summary querying Health1 owner when available.
    async fn health(&self, #[zbus(connection)] conn: &zbus::Connection) -> String {
        #[cfg(target_os = "linux")]
        {
            use cybou_fabric::HEALTH;
            if let Ok(reply) = conn
                .call_method(
                    Some(HEALTH.service),
                    HEALTH.object_path,
                    Some(HEALTH.interface),
                    "Health",
                    &(),
                )
                .await
                && let Ok(h) = reply.body().deserialize::<String>()
            {
                return h;
            }
        }
        let _ = conn;
        "unknown".to_string()
    }

    /// Last error diagnostic.
    async fn last_error(&self) -> String {
        String::new()
    }

    /// Compound snapshot projection encoded as CBOR from Health1, or honest unknown default.
    async fn snapshot(&self, #[zbus(connection)] conn: &zbus::Connection) -> Vec<u8> {
        #[cfg(target_os = "linux")]
        {
            use cybou_fabric::HEALTH;
            if let Ok(reply) = conn
                .call_method(
                    Some(HEALTH.service),
                    HEALTH.object_path,
                    Some(HEALTH.interface),
                    "Snapshot",
                    &(),
                )
                .await
                && let Ok(snap_bytes) = reply.body().deserialize::<Vec<u8>>()
                && !snap_bytes.is_empty()
            {
                return snap_bytes;
            }
        }
        let _ = conn;
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

    /// User mutation: promise an obligation (fails closed with empty string when unexecuted).
    async fn promise(&self, _description: String) -> String {
        String::new()
    }

    /// User mutation: trigger self reflection (fails closed with false when unexecuted).
    async fn reflect(&self) -> bool {
        false
    }

    /// User mutation: fulfill intention at index (fails closed with false when unexecuted).
    async fn fulfill_index(&self, _index: i32) -> bool {
        false
    }

    /// User mutation: abandon intention at index (fails closed with false when unexecuted).
    async fn abandon_index(&self, _index: i32) -> bool {
        false
    }

    /// User mutation: record observation (fails closed with false when unexecuted).
    async fn observe(&self, _subject: String, _value: f64) -> bool {
        false
    }

    /// User mutation: produce forecast (returns empty vec when unexecuted).
    async fn predict(&self, _subject: String) -> Vec<u8> {
        vec![]
    }

    /// User mutation: interrupt background lifecycle (fails closed with false when unexecuted).
    async fn interrupt_lifecycle(&self, _cause: String) -> bool {
        false
    }

    /// Signal emitted when compound projection changes.
    #[zbus(signal)]
    async fn changed(ctxt: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// Emit `Changed` from outside a method call, over the connection that owns Presence1.
///
/// Presence1 has no clock of its own: it changes when the owners it presents change. The
/// re-emission task is therefore not a D-Bus method and needs an emitter bound to the owning
/// connection.
///
/// # Errors
///
/// Returns the zbus error when the path is invalid or the signal cannot be sent.
pub async fn emit_changed(connection: &zbus::Connection) -> zbus::Result<()> {
    let emitter = SignalEmitter::new(connection, PRESENCE.object_path)?;
    Presence1Service::changed(&emitter).await
}
