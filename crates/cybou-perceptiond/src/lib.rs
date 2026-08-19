// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Grounded system perception and observation daemon.
//!
//! Periodically samples local OS state via `LinuxSystemSource`, produces
//! `Observation` envelopes, and exposes the `org.cybou.Mind.Perception1` D-Bus interface.

use std::sync::RwLock;

use cybou_perception::{AcquisitionStatus, LinuxSystemSource};
use cybou_protocol::{canonical::CanonicalEnvelope, observation::ObservationV1};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

#[cfg(target_os = "linux")]
pub mod service;

/// Snapshot of the latest perception acquisition state.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerceptionState {
    /// Status of the last acquisition.
    pub status: AcquisitionStatus,
    /// Timestamp when acquisition was completed.
    #[serde(with = "time::serde::rfc3339")]
    pub acquired_at: OffsetDateTime,
    /// Last observed payload if successful.
    pub observation: Option<ObservationV1>,
    /// Source ID.
    pub source_id: String,
}

/// Errors occurring in perception subsystem.
#[derive(Debug, Error)]
pub enum PerceptionError {
    /// Internal lock poisoned.
    #[error("perception state lock poisoned")]
    LockPoisoned,
}

/// Core domain logic of the perception adapter.
pub struct PerceptionCore {
    source: LinuxSystemSource,
    state: RwLock<PerceptionState>,
    last_contributed_payload: RwLock<Option<Vec<u8>>>,
    last_contributed_fresh_until: RwLock<Option<OffsetDateTime>>,
}

impl Default for PerceptionCore {
    fn default() -> Self {
        Self::new(LinuxSystemSource::new_standard(300))
    }
}

impl PerceptionCore {
    /// Create a new PerceptionCore manager around a source.
    #[must_use]
    pub fn new(source: LinuxSystemSource) -> Self {
        let initial_state = PerceptionState {
            status: AcquisitionStatus::SourceUnavailable,
            acquired_at: OffsetDateTime::UNIX_EPOCH,
            observation: None,
            source_id: "linux.system".to_string(),
        };

        Self {
            source,
            state: RwLock::new(initial_state),
            last_contributed_payload: RwLock::new(None),
            last_contributed_fresh_until: RwLock::new(None),
        }
    }

    /// Read source once and return a CanonicalEnvelope if a contribution is warranted.
    pub fn acquire_once(
        &self,
        now: OffsetDateTime,
        monotonic_time: u64,
    ) -> Option<CanonicalEnvelope> {
        let reading = self.source.acquire(now);
        let status = reading.status;
        let observation = reading.observation.and_then(|o| o.into_protocol().ok());

        let new_state = PerceptionState {
            status,
            acquired_at: now,
            observation: observation.clone(),
            source_id: "linux.system".to_string(),
        };

        if let Ok(mut lock) = self.state.write() {
            *lock = new_state;
        }

        let obs = observation?;
        let should_contribute = self.should_contribute(&obs, now);

        if !should_contribute {
            return None;
        }

        // Build canonical envelope
        let payload = obs.encode().unwrap_or_default();

        if let Ok(mut last_payload) = self.last_contributed_payload.write() {
            *last_payload = Some(payload.clone());
        }
        if let Ok(mut fresh_until) = self.last_contributed_fresh_until.write() {
            // Freshness horizon
            *fresh_until = Some(now + time::Duration::seconds(300));
        }

        Some(CanonicalEnvelope {
            schema_version: 3,
            message_id: Uuid::new_v4(),
            correlation_id: Uuid::new_v4(),
            causation_id: Uuid::nil(),
            origin_organ: "perceptiond".to_string(),
            origin_node: String::new(),
            kind: 1, // Observation
            wall_time_ms: now.unix_timestamp_nanos() as i64 / 1_000_000,
            monotonic_time,
            logical_clock: 1,
            confidence: 1.0,
            evidence: vec![],
            payload,
            privacy: 1, // Node
            capability_scope: String::new(),
            sealed: false,
            key_domain_id: Uuid::nil(),
            key_epoch: 0,
            retention_class: 2,
            retention_policy_version: 0,
            retain_until_ms: 0,
            sensitivity: 1,
        })
    }

    /// Current health state summary ("healthy", "degraded", "unavailable").
    #[must_use]
    pub fn health(&self) -> &'static str {
        let state = match self.state.read() {
            Ok(g) => g.clone(),
            Err(_) => return "unavailable",
        };

        match state.status {
            AcquisitionStatus::Acquired => "healthy",
            AcquisitionStatus::SourceMalformed => "degraded",
            AcquisitionStatus::SourceUnavailable => "unavailable",
        }
    }

    /// Current perception state.
    #[must_use]
    pub fn current_state(&self) -> PerceptionState {
        self.state
            .read()
            .map(|g| g.clone())
            .unwrap_or_else(|_| PerceptionState {
                status: AcquisitionStatus::SourceUnavailable,
                acquired_at: OffsetDateTime::UNIX_EPOCH,
                observation: None,
                source_id: "linux.system".to_string(),
            })
    }

    fn should_contribute(&self, obs: &ObservationV1, now: OffsetDateTime) -> bool {
        let prev_payload = self
            .last_contributed_payload
            .read()
            .ok()
            .and_then(|g| g.clone());
        let prev_fresh = self
            .last_contributed_fresh_until
            .read()
            .ok()
            .and_then(|g| *g);

        let (Some(prev_p), Some(prev_f)) = (prev_payload, prev_fresh) else {
            return true; // First time contribution
        };

        let current_payload = obs.encode().unwrap_or_default();

        // If payload changed, contribute immediately
        if prev_p != current_payload {
            return true;
        }

        // If unchanged, only contribute if the previous freshness horizon has elapsed
        now >= prev_f
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use time::OffsetDateTime;

    use super::*;

    #[test]
    fn perception_acquisition_and_deduplication() {
        let dir = tempdir().expect("tempdir");
        let os_release = dir.path().join("os-release");
        let machine_id = dir.path().join("machine-id");

        std::fs::write(&os_release, b"NAME=\"Debian GNU/Linux\"\nVERSION_ID=\"13\"\n").expect("write");
        std::fs::write(&machine_id, b"0123456789abcdef0123456789abcdef\n").expect("write");

        let source = LinuxSystemSource::new(os_release, Some(machine_id), 300);
        let core = PerceptionCore::new(source);

        let now = OffsetDateTime::now_utc();
        let env1 = core.acquire_once(now, 100).expect("first contribution");
        assert_eq!(env1.origin_organ, "perceptiond");
        assert_eq!(env1.kind, 1);
        assert_eq!(core.health(), "healthy");

        // Second acquisition immediately with unchanged data -> deduplicated / None
        let env2 = core.acquire_once(now, 101);
        assert!(env2.is_none());
    }
}
