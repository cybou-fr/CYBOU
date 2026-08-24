// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Telemetry1` service implementation on zbus.

// The `#[interface]` expansion emits part of its dispatch surface with the attribute's own span,
// which an `allow` on the impl block cannot reach. Every handler written here is documented.
#![allow(missing_docs)]

use std::sync::Arc;

use time::OffsetDateTime;
use zbus::{interface, object_server::SignalEmitter};

use crate::TelemetryCore;

/// D-Bus service exporting `org.cybou.Mind.Telemetry1`.
pub struct Telemetry1Service {
    core: Arc<TelemetryCore>,
}

impl Telemetry1Service {
    /// Create a new handler around the telemetry organ.
    #[must_use]
    pub fn new(core: Arc<TelemetryCore>) -> Self {
        Self { core }
    }
}

#[allow(
    clippy::unused_async,
    reason = "zbus dispatches every exported handler as a future"
)]
#[interface(name = "org.cybou.Mind.Telemetry1")]
impl Telemetry1Service {
    /// Whether this organ has watched long enough to have an opinion.
    ///
    /// False for the first minutes after a restart, and that is the honest answer: a window of four
    /// readings has no notion of what is ordinary for this host, and reporting ready would let a
    /// surface show a confident all-clear built on nothing.
    async fn ready(&self) -> bool {
        self.core.has_watched_enough()
    }

    /// Overall health.
    ///
    /// An organ that has not watched long enough is working, not healthy — its answers are about a
    /// fragment of a window while claiming to be about the host. The same distinction every derived
    /// organ here makes on the same grounds.
    async fn health(&self) -> String {
        if self.core.has_watched_enough() {
            "healthy".to_string()
        } else {
            "degraded".to_string()
        }
    }

    /// Last error diagnostic.
    async fn last_error(&self) -> String {
        String::new()
    }

    /// The most recent reading for each subject that has one, as CBOR.
    ///
    /// Subjects with no reading are absent rather than zero: a host without pressure accounting or
    /// without swap has nothing to say about them, and a surface showing `0.0` would be showing a
    /// perfectly calm machine where there is no measurement at all.
    async fn latest(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&self.core.latest(), &mut buf);
        buf
    }

    /// How each subject sits relative to what is ordinary for this host, as CBOR.
    async fn deviations(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&self.core.deviations(), &mut buf);
        buf
    }

    /// What is known about every watched thing, including what is not known, as CBOR.
    ///
    /// Carries the declared things that produced no reading. A surface built from the readings
    /// alone cannot tell a certificate nobody declared from one that was declared and never read,
    /// and those are opposites.
    #[zbus(property)]
    async fn watching(&self) -> Vec<u8> {
        let watching = self
            .core
            .watching(OffsetDateTime::now_utc(), crate::STALE_AFTER);
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&watching, &mut buf);
        buf
    }

    /// What this host currently concludes about itself, as CBOR.
    ///
    /// Every entry is a hypothesis carrying the readings that produced it. None of them is a fact,
    /// and none of them has been written to the Journal by being asked for here — reading what the
    /// host thinks is not the same act as the host committing to it.
    async fn insights(&self) -> Vec<u8> {
        // No identity is supplied here. It used to be a fresh v4 per read, which meant two requests
        // a second apart described one physically identical condition with two different
        // identities. The core derives it from what makes the condition itself, so a proposal that
        // cites a cause cites something that still exists on the next read.
        let insights = self.core.insights(OffsetDateTime::now_utc());
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&insights, &mut buf);
        buf
    }

    /// Where each watched subject is heading, and when it becomes a problem, as CBOR.
    ///
    /// Only the subjects that have a threshold at all. A load average has none — 4 is a crisis on
    /// one machine and a Tuesday on another — so projecting it against a number would be inventing
    /// the number.
    async fn projections(&self) -> Vec<u8> {
        let projections = self.core.projections(OffsetDateTime::now_utc());
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&projections, &mut buf);
        buf
    }

    /// Signal emitted when what the host concludes about itself changes.
    #[zbus(signal)]
    async fn insights_changed(ctxt: &SignalEmitter<'_>) -> zbus::Result<()>;
}
