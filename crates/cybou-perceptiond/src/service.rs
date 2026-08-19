// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Perception1` service implementation on zbus.

use std::sync::Arc;

use zbus::{SignalContext, interface};

use crate::PerceptionCore;

/// D-Bus Service exporting `org.cybou.Mind.Perception1`.
pub struct Perception1Service {
    core: Arc<PerceptionCore>,
}

impl Perception1Service {
    /// Create a new Perception1 D-Bus service handler around `PerceptionCore`.
    #[must_use]
    pub fn new(core: Arc<PerceptionCore>) -> Self {
        Self { core }
    }
}

#[interface(name = "org.cybou.Mind.Perception1")]
impl Perception1Service {
    /// Service readiness.
    async fn ready(&self) -> bool {
        true
    }

    /// Overall health summary ("healthy", "degraded", "unavailable").
    async fn health(&self) -> String {
        self.core.health().to_string()
    }

    /// Last error diagnostic.
    async fn last_error(&self) -> String {
        String::new()
    }

    /// Return latest perception state encoded as CBOR.
    async fn state(&self) -> Vec<u8> {
        let state = self.core.current_state();
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&state, &mut buf);
        buf
    }

    /// Signal emitted when perception acquisition state changes.
    #[zbus(signal)]
    async fn changed(ctxt: &SignalContext<'_>) -> zbus::Result<()>;
}
