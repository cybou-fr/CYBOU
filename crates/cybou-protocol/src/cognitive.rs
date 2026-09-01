// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Cognitive Graph & Canonical Event1 Journal representations (Milestone 7).

use crate::epistemic::EpistemicStatus;
use crate::subject::SubjectRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Kind and attributes of a node in the unified CYBOU Cognitive Graph.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "attributes", rename_all = "kebab-case")]
pub enum CognitiveNodeType {
    /// Autonomous or pair agent session.
    Agent {
        /// Agent name.
        name: String,
        /// Model identifier.
        model: String,
        /// Agent lifecycle state.
        state: String,
    },
    /// Operating system daemon or service.
    Service {
        /// Service unit name.
        name: String,
        /// Service runtime state.
        state: String,
    },
    /// Running operating system process.
    Process {
        /// Process ID.
        pid: u32,
        /// Process name.
        name: String,
    },
    /// Host filesystem path or project directory.
    HostPath {
        /// Absolute or relative path.
        path: String,
    },
    /// Epistemic finding / diagnosis from telemetry.
    Finding {
        /// Cause identifier.
        cause_id: String,
        /// Severity classification.
        severity: String,
        /// Finding title.
        title: String,
    },
    /// Formulated causal hypothesis.
    Hypothesis {
        /// Hypothesis title.
        title: String,
    },
    /// Agent or system intention / goal.
    Intention {
        /// Intention goal.
        goal: String,
        /// Status of intention.
        status: String,
    },
    /// Executed or proposed system action.
    Action {
        /// Name of action.
        action_name: String,
        /// Outcome string.
        outcome: String,
    },
    /// Server-owned asynchronous operation.
    Operation {
        /// Operation ID.
        op_id: String,
        /// Status of operation.
        status: String,
    },
    /// Personal email communication.
    MailMessage {
        /// Mail subject.
        subject: String,
        /// Sender address.
        from: String,
    },
    /// Scheduled calendar event.
    CalendarEvent {
        /// Event title.
        title: String,
        /// Event time.
        time: String,
    },
    /// Personal knowledge note.
    Note {
        /// Note title.
        title: String,
    },
    /// Address book contact.
    Contact {
        /// Contact name.
        name: String,
        /// Professional role.
        role: String,
    },
    /// Security sandbox policy or confinement rule.
    SecurityPolicy {
        /// Policy name.
        name: String,
        /// Whether policy is actively enforced.
        enforced: bool,
    },
}

impl CognitiveNodeType {
    /// Human readable category name.
    #[must_use]
    pub const fn category_name(&self) -> &'static str {
        match self {
            Self::Agent { .. } => "Agent",
            Self::Service { .. } => "Service",
            Self::Process { .. } => "Process",
            Self::HostPath { .. } => "File / Path",
            Self::Finding { .. } => "Finding",
            Self::Hypothesis { .. } => "Hypothesis",
            Self::Intention { .. } => "Intention",
            Self::Action { .. } => "Action",
            Self::Operation { .. } => "Operation",
            Self::MailMessage { .. } => "Mail",
            Self::CalendarEvent { .. } => "Calendar",
            Self::Note { .. } => "Note",
            Self::Contact { .. } => "Contact",
            Self::SecurityPolicy { .. } => "Security",
        }
    }
}

/// A node in the Cognitive Graph.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CognitiveProvenance {
    /// Read directly from the runtime or an owning observation surface.
    Observed,
    /// Declared by operator-controlled configuration, but not necessarily present now.
    Configured,
    /// A design relationship declared by the architecture.
    Architectural,
    /// Computed from named evidence.
    Derived,
    /// A conclusion that is neither directly observed nor configured.
    #[default]
    Inferred,
}

impl CognitiveProvenance {
    /// Stable human-readable provenance label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Observed => "Observed",
            Self::Configured => "Configured",
            Self::Architectural => "Architectural",
            Self::Derived => "Derived",
            Self::Inferred => "Inferred",
        }
    }
}

/// A node in the Cognitive Graph.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveNodeRecord {
    /// Unique node identifier (e.g. `node:service:cybou-web-gateway`).
    pub id: String,
    /// Display label.
    pub label: String,
    /// Type and structured attributes.
    pub node_type: CognitiveNodeType,
    /// Epistemic validity standing.
    pub epistemic_status: EpistemicStatus,
    /// Confidence metric (0.0 to 1.0).
    pub confidence: f64,
    /// How this record entered the graph.
    #[serde(default)]
    pub provenance: CognitiveProvenance,
    /// Canonical evidence identifiers supporting this record.
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    /// When the underlying runtime fact was observed, when applicable.
    #[serde(default)]
    pub observed_at: Option<String>,
    /// Optional underlying `SubjectRef` for desktop deep-linking.
    pub subject: Option<SubjectRef>,
    /// Creation timestamp (ISO 8601).
    pub created_at: String,
    /// Last update timestamp (ISO 8601).
    pub updated_at: String,
    /// Arbitrary key-value metadata tags.
    pub metadata: HashMap<String, String>,
}

/// Relationship / edge type between cognitive entities.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CognitiveEdgeType {
    /// Direct causal predecessor (`A causes B`).
    Causes,
    /// Observation or telemetry collection link (`Observer observes Target`).
    Observes,
    /// Epistemic derivation (`Finding derives from Evidence`).
    DerivesFrom,
    /// Executed impact (`Action acts upon Entity`).
    ActsUpon,
    /// Security governance (`Policy governs Target`).
    Governs,
    /// Semantic association (`Note / Mail references Entity`).
    References,
    /// Temporal chronological order.
    Precedes,
}

impl CognitiveEdgeType {
    /// Edge label.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Causes => "causes",
            Self::Observes => "observes",
            Self::DerivesFrom => "derives from",
            Self::ActsUpon => "acts upon",
            Self::Governs => "governs",
            Self::References => "references",
            Self::Precedes => "precedes",
        }
    }
}

/// A directed edge in the Cognitive Graph.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveEdgeRecord {
    /// Unique edge identifier.
    pub id: String,
    /// Source node ID.
    pub source_id: String,
    /// Target node ID.
    pub target_id: String,
    /// Relationship type.
    pub edge_type: CognitiveEdgeType,
    /// Connection weight / strength (0.0 to 1.0).
    pub weight: f64,
    /// How this relationship entered the graph.
    #[serde(default)]
    pub provenance: CognitiveProvenance,
    /// Canonical evidence identifiers supporting this relationship.
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    /// When the relationship was observed, if it is an observed relationship.
    #[serde(default)]
    pub observed_at: Option<String>,
    /// Human-readable explanation of the relationship.
    pub description: String,
}

/// The complete or queried subgraph of the Cognitive Graph.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveGraphRecord {
    /// Nodes in the graph.
    pub nodes: Vec<CognitiveNodeRecord>,
    /// Directed relationships.
    pub edges: Vec<CognitiveEdgeRecord>,
}

/// Canonical Event1 journal log entry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventJournalEntry {
    /// Unique event identifier.
    pub event_id: String,
    /// Causal predecessor event ID, if any.
    pub causation_id: Option<String>,
    /// Correlation root ID.
    pub correlation_id: String,
    /// Originating CYBOU organ / daemon (e.g. `actiond`, `presenced`, `systemd`).
    pub origin_organ: String,
    /// Canonical event classification (e.g. `ObservationRecorded`, `ActionDispatched`, `PolicyUpdated`).
    pub event_type: String,
    /// Human-readable summary.
    pub summary: String,
    /// Truncated JSON / plaintext payload preview.
    pub payload_preview: String,
    /// UTC timestamp (ISO 8601).
    pub timestamp: String,
    /// Optional underlying `SubjectRef`.
    pub subject: Option<SubjectRef>,
    /// Epistemic validity standing.
    pub epistemic_status: EpistemicStatus,
}
