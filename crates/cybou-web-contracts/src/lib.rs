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

/// One owner's contribution to the Mind panel.
///
/// Every section carries its own [`KnowledgeState`] because the owners are separate processes and
/// fail separately: a Journal the gateway could not reach must not render as a Journal with no
/// contributions. `Unknown` means the owner was not reached, and the payload fields are then
/// absent rather than zero.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityProjection {
    /// Whether Identity1 answered.
    pub knowledge: KnowledgeState,
    /// Stable subject identifier.
    pub identity_id: Option<String>,
    /// RFC 3339 instant the subject was first created.
    pub origin: Option<String>,
    /// Number of sessions since origin.
    pub session_count: Option<u64>,
    /// Whole days since origin.
    pub age_in_days: Option<i64>,
    /// Architecture version the subject was created under.
    pub architecture_version: Option<String>,
}

/// What the canonical Journal holds, as reported by Event1.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalProjection {
    /// Whether Event1 answered.
    pub knowledge: KnowledgeState,
    /// Total accepted contributions.
    pub contribution_count: Option<u64>,
    /// Current erasure epoch.
    pub erasure_epoch: Option<u64>,
}

/// One open commitment as Intention1 holds it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitmentProjection {
    /// Intention identity.
    pub id: String,
    /// What was promised.
    pub description: String,
    /// Condition under which it became active.
    pub trigger: String,
    /// RFC 3339 formation instant.
    pub formed: String,
}

/// Open obligations, as reported by Intention1.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitmentsProjection {
    /// Whether Intention1 answered. An empty list is meaningful only when this is `Known`.
    pub knowledge: KnowledgeState,
    /// Number of open obligations.
    pub open_count: Option<u32>,
    /// The open obligations themselves.
    pub open: Vec<CommitmentProjection>,
}

/// Sleep/wake state, as reported by Lifecycle1.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleProjection {
    /// Whether Lifecycle1 answered.
    pub knowledge: KnowledgeState,
    /// Current mode, in the owner's own spelling.
    pub mode: Option<String>,
    /// RFC 3339 instant of the last observed user activity.
    pub last_user_activity_at: Option<String>,
}

/// What Mind actually holds right now, returned by `/api/v1/mind`.
///
/// Only owners that hold real state appear here. Nothing in this projection is composed by the
/// gateway: each section is what one owner answered, or an explicit unknown.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MindProjection {
    /// Web contract version.
    pub schema_version: SchemaVersion,
    /// RFC 3339 instant the gateway assembled this read.
    pub observed_at: String,
    /// Subject continuity.
    pub identity: IdentityProjection,
    /// Canonical Journal.
    pub journal: JournalProjection,
    /// Open obligations.
    pub commitments: CommitmentsProjection,
    /// Sleep/wake state.
    pub lifecycle: LifecycleProjection,
}

#[cfg(test)]
mod tests {
    use super::{
        CommitmentsProjection, MindProjection, SessionMode, SessionProjection, SnapshotProjection,
        WEB_SCHEMA_V1,
    };

    const SESSION_FIXTURE: &str = include_str!("../../../fixtures/web/v1/session-local.json");
    const SNAPSHOT_FIXTURE: &str = include_str!("../../../fixtures/web/v1/snapshot-nominal.json");
    const SESSION_SCHEMA: &str = include_str!("../../../schemas/web/v1/session.schema.json");
    const SNAPSHOT_SCHEMA: &str = include_str!("../../../schemas/web/v1/snapshot.schema.json");
    const MIND_FIXTURE: &str = include_str!("../../../fixtures/web/v1/mind-nominal.json");
    const MIND_SCHEMA: &str = include_str!("../../../schemas/web/v1/mind.schema.json");

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
    fn mind_fixture_round_trips_and_keeps_unknown_distinct_from_empty() {
        let projection: MindProjection =
            serde_json::from_str(MIND_FIXTURE).expect("valid nominal mind fixture");
        let encoded = serde_json::to_string(&projection).expect("serialize mind projection");
        let decoded: MindProjection =
            serde_json::from_str(&encoded).expect("round-trip mind projection");
        assert_eq!(decoded, projection);
        assert_eq!(projection.schema_version, WEB_SCHEMA_V1);

        // A section the gateway could not read must not be readable as a section holding nothing.
        let unreached = CommitmentsProjection {
            knowledge: cybou_protocol::KnowledgeState::Unknown,
            open_count: None,
            open: Vec::new(),
        };
        let known_empty = CommitmentsProjection {
            knowledge: cybou_protocol::KnowledgeState::Known,
            open_count: Some(0),
            open: Vec::new(),
        };
        assert_ne!(unreached, known_empty);
    }

    #[test]
    fn checked_in_json_schemas_are_v1_and_closed() {
        for raw in [SESSION_SCHEMA, SNAPSHOT_SCHEMA, MIND_SCHEMA] {
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
