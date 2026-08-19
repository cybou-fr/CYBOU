// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Lifecycle1` service implementation on zbus.

use std::sync::Arc;

use time::OffsetDateTime;
use zbus::{SignalContext, interface};

use crate::{LifecycleCore, LifecycleMode};

/// D-Bus Service exporting `org.cybou.Mind.Lifecycle1`.
pub struct Lifecycle1Service {
    core: Arc<LifecycleCore>,
}

impl Lifecycle1Service {
    /// Create a new Lifecycle1 D-Bus service handler around `LifecycleCore`.
    #[must_use]
    pub fn new(core: Arc<LifecycleCore>) -> Self {
        Self { core }
    }
}

#[interface(name = "org.cybou.Mind.Lifecycle1")]
impl Lifecycle1Service {
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

    /// Return lifecycle state encoded as CBOR.
    async fn state(&self) -> Vec<u8> {
        let state = self.core.state();
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&state, &mut buf);
        buf
    }

    /// Notify that user interaction occurred.
    async fn notify_user_activity(&self, cause: String) -> bool {
        let now = OffsetDateTime::now_utc();
        self.core.notify_user_activity(&cause, now).is_ok()
    }

    /// Manually transition mode (rejects unknown mode strings).
    async fn transition(&self, mode: String) -> bool {
        let parsed = match mode.to_lowercase().as_str() {
            "awake" => LifecycleMode::Awake,
            "dozing" => LifecycleMode::Dozing,
            "dreaming" => LifecycleMode::Dreaming,
            "deep-rest" => LifecycleMode::DeepRest,
            "consolidating" => LifecycleMode::Consolidating,
            "maintenance" => LifecycleMode::Maintenance,
            "interrupted" => LifecycleMode::Interrupted,
            _ => return false, // reject unknown mode string without changing state
        };
        self.core.transition(parsed).is_ok()
    }

    /// Signal emitted when lifecycle state changes.
    #[zbus(signal)]
    async fn changed(ctxt: &SignalContext<'_>) -> zbus::Result<()>;
}
