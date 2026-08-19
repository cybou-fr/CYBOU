// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Epistemic1` service implementation on zbus.

use std::sync::Arc;

use zbus::{SignalContext, interface};

use crate::EpistemicCore;

/// D-Bus Service exporting `org.cybou.Mind.Epistemic1`.
pub struct Epistemic1Service {
    core: Arc<EpistemicCore>,
}

impl Epistemic1Service {
    /// Create a new Epistemic1 D-Bus service handler around `EpistemicCore`.
    #[must_use]
    pub fn new(core: Arc<EpistemicCore>) -> Self {
        Self { core }
    }
}

#[interface(name = "org.cybou.Mind.Epistemic1")]
impl Epistemic1Service {
    /// Service readiness.
    async fn ready(&self) -> bool {
        true
    }

    /// Overall health.
    async fn health(&self) -> String {
        "healthy".to_string()
    }

    /// Last error diagnostic.
    async fn last_error(&self) -> String {
        String::new()
    }

    /// Full epistemic projection encoded as CBOR.
    async fn beliefs(&self) -> Vec<u8> {
        let list = self.core.projection();
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&list, &mut buf);
        buf
    }

    /// Query belief for a single subject encoded as CBOR.
    async fn query(&self, subject: String) -> Vec<u8> {
        let belief = self.core.query(&subject);
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&belief, &mut buf);
        buf
    }

    /// Signal emitted when epistemic belief state changes.
    #[zbus(signal)]
    async fn changed(ctxt: &SignalContext<'_>) -> zbus::Result<()>;
}
