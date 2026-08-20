// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Workspace1` service implementation on zbus.

// The `#[interface]` expansion emits part of its dispatch surface with the attribute's own span,
// which an `allow` on the impl block cannot reach. Every handler written here is documented.
#![allow(missing_docs)]

use std::sync::Arc;

use time::OffsetDateTime;
use zbus::{interface, object_server::SignalEmitter};

use crate::WorkspaceCore;

/// D-Bus Service exporting `org.cybou.Mind.Workspace1`.
pub struct Workspace1Service {
    core: Arc<WorkspaceCore>,
}

impl Workspace1Service {
    /// Create a new Workspace1 D-Bus service handler around `WorkspaceCore`.
    #[must_use]
    pub fn new(core: Arc<WorkspaceCore>) -> Self {
        Self { core }
    }
}

#[allow(
    clippy::unused_async,
    reason = "zbus dispatches every exported handler as a future"
)]
#[interface(name = "org.cybou.Mind.Workspace1")]
impl Workspace1Service {
    /// Whether this organ has read the whole Journal it derives from.
    ///
    /// Answering `true` unconditionally made readiness meaningless: an organ that had just started
    /// and had read nothing reported exactly what one holding the complete projection reported, so
    /// a control plane could not tell a system coming up from a system that is up.
    async fn ready(&self) -> bool {
        self.core.is_caught_up()
    }

    /// Return buffer capacity.
    async fn capacity(&self) -> u32 {
        u32::try_from(self.core.capacity()).unwrap_or(u32::MAX)
    }

    /// Return all active coalitions ordered by salience encoded as CBOR.
    async fn coalitions(&self) -> Vec<u8> {
        let now = OffsetDateTime::now_utc();
        let list = self.core.coalitions(now);
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&list, &mut buf);
        buf
    }

    /// Return winning focus coalition encoded as CBOR.
    async fn focus(&self) -> Vec<u8> {
        let now = OffsetDateTime::now_utc();
        let focus = self.core.focus(now);
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&focus, &mut buf);
        buf
    }

    /// Return momentary workspace state encoded as CBOR.
    async fn moment_state(&self) -> Vec<u8> {
        let now = OffsetDateTime::now_utc();
        let state = self.core.moment_state(now);
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&state, &mut buf);
        buf
    }

    /// Signal emitted when winning focus changes.
    #[zbus(signal)]
    async fn focus_changed(ctxt: &SignalEmitter<'_>, correlation_id: String) -> zbus::Result<()>;
}
