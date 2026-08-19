// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Workspace1` service implementation on zbus.

use std::sync::Arc;

use time::OffsetDateTime;
use zbus::{SignalContext, interface};

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

#[interface(name = "org.cybou.Mind.Workspace1")]
impl Workspace1Service {
    /// Service readiness.
    async fn ready(&self) -> bool {
        true
    }

    /// Return buffer capacity.
    async fn capacity(&self) -> u32 {
        self.core.capacity() as u32
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
    async fn focus_changed(ctxt: &SignalContext<'_>, correlation_id: String) -> zbus::Result<()>;
}
