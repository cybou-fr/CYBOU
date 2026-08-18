// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Transport-independent protocol vocabulary shared by native and WASM components.
//!
//! This crate does not yet replace the canonical C++ protocol implementation. Its types define
//! the Rust compatibility seam and must gain golden byte fixtures before an owner cutover.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub mod canonical;
pub mod observation;

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
