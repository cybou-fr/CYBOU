// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Health1` service implementation on zbus.

use std::sync::Arc;

use time::OffsetDateTime;
use zbus::{SignalContext, interface};

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
    async fn refresh(&self, #[zbus(signal_context)] ctxt: SignalContext<'_>) -> bool {
        self.core.recalculate(OffsetDateTime::now_utc());
        let _ = Self::changed(&ctxt).await;
        true
    }

    /// Signal emitted when capability health snapshot changes.
    #[zbus(signal)]
    async fn changed(ctxt: &SignalContext<'_>) -> zbus::Result<()>;
}
