// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Transport-independent protocol vocabulary shared by native and WASM components.
//!
//! This crate does not yet replace the canonical C++ protocol implementation. Its types define
//! the Rust compatibility seam and must gain golden byte fixtures before an owner cutover.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

pub mod admission;
pub mod attention;
pub mod canonical;
pub mod capability;
pub mod observation;

// Contract-only vocabulary: the modules below name the types of ADR-0031 through ADR-0036 so the
// future agent/worker runtime, model broker, action executor and security control plane are
// designed against one shared spelling. None of them has a runtime owner in this repository yet
// and no daemon depends on them; see `docs/CURRENT_STATE.md`. Adding a type here is not evidence
// that the corresponding behaviour exists.
pub mod action;
pub mod disclosure;
pub mod epistemic;
pub mod governance;
pub mod learning;
pub mod meaning;
pub mod model;
pub mod promotion;
pub mod security;
pub mod telemetry;

pub use admission::Kind;

/// Canonical Qt-compatible UTC wall-clock spelling: whole milliseconds since the Unix epoch.
///
/// The `time` crate reports nanoseconds as `i128`; narrowing that with `as i64` would wrap
/// silently rather than fail, so the division happens first and the result saturates.
#[must_use]
pub fn unix_millis(instant: OffsetDateTime) -> i64 {
    let millis = instant.unix_timestamp_nanos() / 1_000_000;
    i64::try_from(millis).unwrap_or(i64::MAX)
}

/// Version of a serialized contract schema.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct SchemaVersion(pub u16);

/// Stable identifier for an object exposed through a projection.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ObjectId(pub Uuid);

/// Whether a projected value is known independently of whether it is empty.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum KnowledgeState {
    /// The owner produced a value; the value itself may legitimately be empty.
    Known,
    /// The owner cannot currently determine a value.
    Unknown,
}

/// End-to-end availability of a capability.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CapabilityState {
    /// The capability can currently accept its declared operations.
    Available,
    /// A declared dependency or policy prevents use.
    Unavailable,
    /// Availability could not be established within the bounded request.
    Unknown,
}

/// A failure value that must not be collapsed into an empty projection.
#[derive(Clone, Debug, Deserialize, Eq, Error, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProjectionError {
    /// The owner or transport did not respond within the declared budget.
    #[error("projection timed out after {budget_ms} ms")]
    Timeout {
        /// Bounded request budget.
        budget_ms: u32,
    },
    /// The owning capability is currently unavailable.
    #[error("capability {capability} is unavailable: {reason}")]
    Unavailable {
        /// Stable capability identifier.
        capability: String,
        /// Human-readable diagnostic; never an authorization decision.
        reason: String,
    },
    /// The payload used an unsupported schema.
    #[error("unsupported schema version {received}")]
    UnsupportedSchema {
        /// Schema version received from the peer.
        received: u16,
    },
}

#[cfg(test)]
mod tests {
    use super::{CapabilityState, KnowledgeState, ProjectionError, SchemaVersion};

    #[test]
    fn unknown_is_distinct_from_unavailable_and_empty() {
        assert_ne!(CapabilityState::Unknown, CapabilityState::Unavailable);
        assert_ne!(KnowledgeState::Unknown, KnowledgeState::Known);
        assert_eq!(SchemaVersion(1), SchemaVersion(1));
    }

    #[test]
    fn errors_remain_typed() {
        let error = ProjectionError::Timeout { budget_ms: 900 };
        assert_eq!(error.to_string(), "projection timed out after 900 ms");
    }
}
