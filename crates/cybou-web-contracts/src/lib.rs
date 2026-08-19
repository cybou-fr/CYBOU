// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Explicit versioned contract between Living Canvas and `cybou-web-gateway`.

use cybou_protocol::{CapabilityState, KnowledgeState, SchemaVersion};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// First web contract version. It is independent of internal D-Bus encoding versions.
pub const WEB_SCHEMA_V1: SchemaVersion = SchemaVersion(1);

/// Trust context established by the gateway, never by a frontend toggle.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionMode {
    /// Device-bound loopback session created by the desktop shell.
    LocalDesktop,
    /// Explicitly unauthenticated surface reachable by anyone who has the address.
    ///
    /// The name records the trust level, not the content: it promises no authentication was
    /// performed, and makes no claim that what it shows is non-personal. Whether a deployment
    /// points this mode at fixtures or at a live Mind is the owner's decision.
    PublicPreview,
    /// Authenticated browser session crossing the external network boundary.
    RemoteBrowser,
}

/// Freshness carried with every owner projection.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Freshness {
    /// Projection is within the owner's declared freshness budget.
    Current,
    /// Projection is usable only as explicitly labelled stale context.
    Stale,
    /// Freshness could not be established.
    Unknown,
}

/// Authenticated session projection returned by `/api/v1/session`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// Opaque revocable session identifier.
    pub session_id: Uuid,
    /// Server-established trust context.
    pub mode: SessionMode,
    /// Stable named-consumer identifier used for context delivery policy.
    pub consumer_id: String,
    /// RFC 3339 expiry timestamp supplied by the gateway.
    pub expires_at: String,
}

/// Minimal capability row used by deterministic frontend fixtures.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityProjection {
    /// Stable capability identifier.
    pub id: String,
    /// Owner-projected state.
    pub state: CapabilityState,
    /// Whether the state itself is known.
    pub knowledge: KnowledgeState,
    /// Freshness of the projection.
    pub freshness: Freshness,
    /// Optional non-authoritative explanation.
    pub reason: Option<String>,
}

/// Atomic read model returned by `/api/v1/snapshot`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// Monotonic opaque projection revision.
    pub projection_version: u64,
    /// Cursor from which event resumption may be attempted.
    pub cursor: String,
    /// RFC 3339 observation timestamp.
    pub observed_at: String,
    /// Whether the aggregate projection is fresh.
    pub freshness: Freshness,
    /// Whether the aggregate projection is known; distinguishes known-empty from unavailable.
    pub knowledge: KnowledgeState,
    /// Current capability rows; an empty list is meaningful only when the projection is known.
    pub capabilities: Vec<CapabilityProjection>,
}

#[cfg(test)]
mod tests {
    use super::{SessionMode, SessionProjection, SnapshotProjection, WEB_SCHEMA_V1};

    const SESSION_FIXTURE: &str = include_str!("../../../fixtures/web/v1/session-local.json");
    const SNAPSHOT_FIXTURE: &str = include_str!("../../../fixtures/web/v1/snapshot-nominal.json");
    const SESSION_SCHEMA: &str = include_str!("../../../schemas/web/v1/session.schema.json");
    const SNAPSHOT_SCHEMA: &str = include_str!("../../../schemas/web/v1/snapshot.schema.json");

    #[test]
    fn local_session_fixture_is_explicitly_local() {
        let projection: SessionProjection =
            serde_json::from_str(SESSION_FIXTURE).expect("valid local session fixture");
        assert_eq!(projection.schema_version, WEB_SCHEMA_V1);
        assert_eq!(projection.mode, SessionMode::LocalDesktop);
        assert!(!projection.consumer_id.is_empty());
    }

    #[test]
    fn nominal_snapshot_round_trips_without_losing_state() {
        let projection: SnapshotProjection =
            serde_json::from_str(SNAPSHOT_FIXTURE).expect("valid nominal snapshot fixture");
        let encoded = serde_json::to_string(&projection).expect("serialize nominal snapshot");
        let decoded: SnapshotProjection =
            serde_json::from_str(&encoded).expect("round-trip nominal snapshot");
        assert_eq!(decoded, projection);
        assert_eq!(projection.schema_version, WEB_SCHEMA_V1);
        assert!(!projection.cursor.is_empty());
        assert_eq!(projection.knowledge, cybou_protocol::KnowledgeState::Known);
    }

    #[test]
    fn checked_in_json_schemas_are_v1_and_closed() {
        for raw in [SESSION_SCHEMA, SNAPSHOT_SCHEMA] {
            let schema: serde_json::Value = serde_json::from_str(raw).expect("valid JSON schema");
            assert_eq!(
                schema["$schema"],
                "https://json-schema.org/draft/2020-12/schema"
            );
            assert_eq!(schema["additionalProperties"], false);
            assert_eq!(schema["properties"]["schemaVersion"]["const"], 1);
        }
    }
}
