// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Health1` service implementation on zbus.

// The `#[interface]` expansion emits part of its dispatch surface with the attribute's own span,
// which an `allow` on the impl block cannot reach. Every handler written here is documented.
#![allow(missing_docs)]

use std::sync::Arc;

use cybou_fabric::HEALTH;
use time::OffsetDateTime;
use zbus::{interface, object_server::SignalEmitter};

use crate::HealthCore;

/// D-Bus Service exporting `org.cybou.Mind.Health1`.
pub struct Health1Service {
    core: Arc<HealthCore>,
}

impl Health1Service {
    /// Create a new Health1 D-Bus service handler around `HealthCore`.
    #[must_use]
    pub fn new(core: Arc<HealthCore>) -> Self {
        Self { core }
    }
}

#[allow(
    clippy::unused_async,
    reason = "zbus dispatches every exported handler as a future"
)]
#[interface(name = "org.cybou.Mind.Health1")]
impl Health1Service {
    /// Service readiness.
    async fn ready(&self) -> bool {
        true
    }

    /// Overall health summary ("healthy", "degraded", "unavailable").
    async fn health(&self) -> String {
        self.core.overall_health().to_string()
    }

    /// Last error diagnostic.
    async fn last_error(&self) -> String {
        String::new()
    }

    /// Whether a capability snapshot is currently available.
    async fn has_snapshot(&self) -> bool {
        self.core.current_snapshot().is_some()
    }

    /// Retrieve the current capability snapshot encoded as CBOR.
    async fn snapshot(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        if let Some(snap) = self.core.current_snapshot() {
            let _ = ciborium::into_writer(&snap, &mut buf);
        }
        buf
    }

    /// Whether homeostasis metrics are available.
    async fn has_measurements(&self) -> bool {
        false
    }

    /// Homeostasis measurements encoded as CBOR.
    async fn measurements(&self) -> Vec<u8> {
        vec![]
    }

    /// Refresh capability evaluations and emit Changed signal.
    async fn refresh(&self, #[zbus(signal_emitter)] ctxt: SignalEmitter<'_>) -> bool {
        let _ = self.core.recalculate(OffsetDateTime::now_utc());
        let _ = Self::changed(&ctxt).await;
        true
    }

    /// Signal emitted when capability health snapshot changes.
    #[zbus(signal)]
    async fn changed(ctxt: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// Emit `Changed` from outside a method call, over the connection that owns Health1.
///
/// The periodic probe loop is not a D-Bus method, so it never receives a `SignalEmitter` of its
/// own. It must still emit from the owning connection: a signal sent over any other connection
/// would carry a sender no subscriber matches.
///
/// # Errors
///
/// Returns the zbus error when the path is invalid or the signal cannot be sent.
pub async fn emit_changed(connection: &zbus::Connection) -> zbus::Result<()> {
    let emitter = SignalEmitter::new(connection, HEALTH.object_path)?;
    Health1Service::changed(&emitter).await
}
