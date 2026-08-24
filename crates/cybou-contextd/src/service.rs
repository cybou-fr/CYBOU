// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Context1` service implementation on zbus.

// The `#[interface]` expansion emits part of its dispatch surface with the attribute's own span,
// which an `allow` on the impl block cannot reach. Every handler written here is documented.
#![allow(missing_docs)]

use std::sync::Arc;

use zbus::{interface, object_server::SignalEmitter};

use crate::{ActivationBudget, ContextCore};

/// D-Bus Service exporting `org.cybou.Mind.Context1`.
pub struct Context1Service {
    core: Arc<ContextCore>,
}

impl Context1Service {
    /// Create a new Context1 D-Bus service handler around `ContextCore`.
    #[must_use]
    pub fn new(core: Arc<ContextCore>) -> Self {
        Self { core }
    }
}

#[allow(
    clippy::unused_async,
    reason = "zbus dispatches every exported handler as a future"
)]
#[interface(name = "org.cybou.Mind.Context1")]
impl Context1Service {
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
        // A projection that has not read the whole Journal is working, not healthy: its answers
        // are about part of a biography while claiming to be about all of it. Saying "healthy"
        // regardless made the one state worth reporting the one state it could not report.
        if self.core.is_caught_up() {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        }
    }

    /// Last error diagnostic.
    async fn last_error(&self) -> String {
        String::new()
    }

    /// Active context entries encoded as CBOR.
    async fn active_context(&self) -> Vec<u8> {
        let list = self.core.active_context();
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&list, &mut buf);
        buf
    }

    /// What the named concepts bring to mind, as a bounded, inspectable walk, encoded as CBOR.
    ///
    /// Every returned concept carries the path it was reached by, and the session says what
    /// stopped it. A caller that wants only the labels can drop the rest; a caller that wants to
    /// know why a thing came back does not have to ask anything to invent a reason.
    async fn bring_to_mind(&self, seeds: Vec<String>) -> Vec<u8> {
        let seeds: Vec<crate::Seed> = seeds.into_iter().map(crate::Seed::Concept).collect();
        let session = self
            .core
            .bring_to_mind(&seeds, &ActivationBudget::default());
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&session, &mut buf);
        buf
    }

    /// The same walk, from seeds that are not words, given as CBOR.
    ///
    /// A separate method rather than a second signature on the first, because D-Bus carries strings
    /// and a typed seed is not one. ADR-0029 is explicit that restricting seeds to text would make
    /// this layer an accessory to a chat box: what the workspace is looking at, an intention being
    /// held, a finding about this host and a metric it watches are all things worth bringing to
    /// mind, and none of them is a word anybody typed.
    ///
    /// Seeds that decode to nothing this graph holds are named back in the session, rather than
    /// counted. A caller that cannot tell which of four seeds found nothing cannot tell an empty
    /// corner of the graph from a mistyped one.
    async fn bring_to_mind_from(&self, seeds: Vec<u8>) -> Vec<u8> {
        // An undecodable request activates nothing rather than activating everything. A malformed
        // seed list read as an empty one would walk from no seeds, which returns an empty session
        // that looks exactly like a graph with nothing in it.
        let Ok(seeds) = ciborium::from_reader::<Vec<crate::Seed>, _>(seeds.as_slice()) else {
            return Vec::new();
        };
        let session = self
            .core
            .bring_to_mind(&seeds, &ActivationBudget::default());
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&session, &mut buf);
        buf
    }

    /// Related associative tags for a concept.
    async fn related_tags(&self, tag: String) -> Vec<String> {
        self.core.related_tags(&tag)
    }

    /// Signal emitted when context vector changes.
    #[zbus(signal)]
    async fn changed(ctxt: &SignalEmitter<'_>) -> zbus::Result<()>;
}
