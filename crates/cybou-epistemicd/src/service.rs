// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Epistemic1` service implementation on zbus.

// The `#[interface]` expansion emits part of its dispatch surface with the attribute's own span,
// which an `allow` on the impl block cannot reach. Every handler written here is documented.
#![allow(missing_docs)]

use std::sync::Arc;

use zbus::{interface, object_server::SignalEmitter};

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

#[allow(
    clippy::unused_async,
    reason = "zbus dispatches every exported handler as a future"
)]
#[interface(name = "org.cybou.Mind.Epistemic1")]
impl Epistemic1Service {
    /// Whether this organ has read the whole Journal it derives from.
    ///
    /// Answering `true` unconditionally made readiness meaningless: an organ that had just started
    /// and had read nothing reported exactly what one holding the complete projection reported, so
    /// a control plane could not tell a system coming up from a system that is up.
    async fn ready(&self) -> bool {
        self.core.is_caught_up()
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
    async fn changed(ctxt: &SignalEmitter<'_>) -> zbus::Result<()>;
}
