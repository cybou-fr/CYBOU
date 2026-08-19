// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Self1` service implementation on zbus.

use std::sync::Arc;

use time::OffsetDateTime;
use zbus::interface;

use crate::{SelfCore, SelfReport, narrate_self_report};

/// D-Bus Service exporting `org.cybou.Mind.Self1`.
pub struct Self1Service {
    core: Arc<SelfCore>,
}

impl Self1Service {
    /// Create a new Self1 D-Bus service handler around `SelfCore`.
    #[must_use]
    pub fn new(core: Arc<SelfCore>) -> Self {
        Self { core }
    }
}

#[interface(name = "org.cybou.Mind.Self1")]
impl Self1Service {
    /// Service readiness.
    async fn ready(&self) -> bool {
        true
    }

    /// Measure the self model and return the SelfReport encoded as CBOR.
    async fn measure(&self) -> Vec<u8> {
        let now = OffsetDateTime::now_utc();
        let report = self.core.measure(now, 0);
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&report, &mut buf);
        buf
    }

    /// Assess self against a specific cause contribution and return SelfReport CBOR.
    async fn assess(&self, _cause_id: String) -> Vec<u8> {
        let now = OffsetDateTime::now_utc();
        let report = self.core.measure(now, 0);
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&report, &mut buf);
        buf
    }

    /// Narrate a given CBOR encoded SelfReport into a human-readable text.
    async fn narrate(&self, encoded_report: Vec<u8>) -> String {
        if let Ok(report) = ciborium::from_reader::<SelfReport, _>(encoded_report.as_slice()) {
            narrate_self_report(&report)
        } else {
            let now = OffsetDateTime::now_utc();
            let current = self.core.measure(now, 0);
            narrate_self_report(&current)
        }
    }
}
