// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Thread-safe in-memory provider and governor for the Canonical Event1 Journal & Deep Cognitive Graph (Milestone 7).

use std::collections::HashMap;
use std::sync::RwLock;
use cybou_protocol::cognitive::{
    CognitiveEdgeRecord, CognitiveEdgeType, CognitiveGraphRecord, CognitiveNodeRecord,
    CognitiveNodeType, EventJournalEntry,
};
use cybou_protocol::epistemic::EpistemicStatus;
use cybou_protocol::subject::SubjectRef;
use cybou_web_contracts::{
    CognitiveGraphProjection, CognitiveQueryRequest, EventJournalProjection, WEB_SCHEMA_V1,
};

/// Hub managing the deep cross-subsystem Cognitive Graph and the Canonical Event1 Journal.
pub struct CognitiveHub {
    nodes: RwLock<Vec<CognitiveNodeRecord>>,
    edges: RwLock<Vec<CognitiveEdgeRecord>>,
    journal: RwLock<Vec<EventJournalEntry>>,
}

impl Default for CognitiveHub {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveHub {
    /// Build a new `CognitiveHub` pre-populated with interconnected cognitive nodes across all CYBOU subsystems.
    #[must_use]
    pub fn new() -> Self {
        let default_nodes = vec![
            CognitiveNodeRecord {
                id: "node:agent:opencode-main".to_owned(),
                label: "OpenCode Refactor Agent".to_owned(),
                node_type: CognitiveNodeType::Agent {
                    name: "opencode-main".to_owned(),
                    model: "claude-3-5-sonnet".to_owned(),
                    state: "running".to_owned(),
                },
                epistemic_status: EpistemicStatus::Observed,
                confidence: 0.98,
                subject: Some(SubjectRef::Agent {
                    capsule_id: "capsule-opencode-01".to_owned(),
                    agent_type: "opencode-main".to_owned(),
                }),
                created_at: "2026-08-28T20:00:00Z".to_owned(),
                updated_at: "2026-08-28T23:45:00Z".to_owned(),
                metadata: HashMap::from([
                    ("workspace".to_owned(), "/mnt/c/Users/cybou/Documents/CYBOU".to_owned()),
                    ("sandbox".to_owned(), "landlock+bubblewrap".to_owned()),
                ]),
            },
            CognitiveNodeRecord {
                id: "node:service:cybou-web-gateway".to_owned(),
                label: "CYBOU Web Gateway".to_owned(),
                node_type: CognitiveNodeType::Service {
                    name: "cybou-web-gateway.service".to_owned(),
                    state: "active (running)".to_owned(),
                },
                epistemic_status: EpistemicStatus::Observed,
                confidence: 1.0,
                subject: Some(SubjectRef::Service {
                    name: "cybou-web-gateway.service".to_owned(),
                    node_id: None,
                }),
                created_at: "2026-08-28T18:00:00Z".to_owned(),
                updated_at: "2026-08-28T23:50:00Z".to_owned(),
                metadata: HashMap::from([
                    ("port".to_owned(), "4000".to_owned()),
                    ("protocol".to_owned(), "HTTP/1.1 + WebSocket/SSE".to_owned()),
                ]),
            },
            CognitiveNodeRecord {
                id: "node:service:cybou-actiond".to_owned(),
                label: "Action1 Causality Engine".to_owned(),
                node_type: CognitiveNodeType::Service {
                    name: "cybou-actiond.service".to_owned(),
                    state: "active (running)".to_owned(),
                },
                epistemic_status: EpistemicStatus::Observed,
                confidence: 1.0,
                subject: Some(SubjectRef::Service {
                    name: "cybou-actiond.service".to_owned(),
                    node_id: None,
                }),
                created_at: "2026-08-28T18:00:00Z".to_owned(),
                updated_at: "2026-08-28T23:50:00Z".to_owned(),
                metadata: HashMap::from([
                    ("causality_mode".to_owned(), "strict_dag".to_owned()),
                ]),
            },
            CognitiveNodeRecord {
                id: "node:sec:landlock-v3".to_owned(),
                label: "Landlock LSM Policy (v3)".to_owned(),
                node_type: CognitiveNodeType::SecurityPolicy {
                    name: "landlock-capsule-strict".to_owned(),
                    enforced: true,
                },
                epistemic_status: EpistemicStatus::Observed,
                confidence: 1.0,
                subject: None,
                created_at: "2026-08-28T19:00:00Z".to_owned(),
                updated_at: "2026-08-28T23:00:00Z".to_owned(),
                metadata: HashMap::from([
                    ("abi_version".to_owned(), "3".to_owned()),
                    ("scope".to_owned(), "filesystem_write_confinement".to_owned()),
                ]),
            },
            CognitiveNodeRecord {
                id: "node:finding:audit-pass".to_owned(),
                label: "Security Audit: Zero Violations".to_owned(),
                node_type: CognitiveNodeType::Finding {
                    cause_id: "sec-audit-001".to_owned(),
                    severity: "info".to_owned(),
                    title: "Landlock & Seccomp Integrity Verified".to_owned(),
                },
                epistemic_status: EpistemicStatus::Observed,
                confidence: 0.99,
                subject: None,
                created_at: "2026-08-28T21:40:00Z".to_owned(),
                updated_at: "2026-08-28T21:40:00Z".to_owned(),
                metadata: HashMap::new(),
            },
            CognitiveNodeRecord {
                id: "node:mail:audit-report".to_owned(),
                label: "Mail: Weekly Sandbox Audit".to_owned(),
                node_type: CognitiveNodeType::MailMessage {
                    subject: "Weekly Sandbox & Landlock Confinement Audit".to_owned(),
                    from: "security@cybou.local".to_owned(),
                },
                epistemic_status: EpistemicStatus::Observed,
                confidence: 1.0,
                subject: None,
                created_at: "2026-08-28T21:40:00Z".to_owned(),
                updated_at: "2026-08-28T21:40:00Z".to_owned(),
                metadata: HashMap::new(),
            },
            CognitiveNodeRecord {
                id: "node:note:invariants".to_owned(),
                label: "Note: Living Canvas Invariants".to_owned(),
                node_type: CognitiveNodeType::Note {
                    title: "Living Canvas Invariants & Principles".to_owned(),
                },
                epistemic_status: EpistemicStatus::Observed,
                confidence: 1.0,
                subject: None,
                created_at: "2026-08-28T22:00:00Z".to_owned(),
                updated_at: "2026-08-28T22:00:00Z".to_owned(),
                metadata: HashMap::new(),
            },
            CognitiveNodeRecord {
                id: "node:contact:elena".to_owned(),
                label: "Contact: Dr. Elena Rostova".to_owned(),
                node_type: CognitiveNodeType::Contact {
                    name: "Dr. Elena Rostova".to_owned(),
                    role: "Cognitive Systems Architect".to_owned(),
                },
                epistemic_status: EpistemicStatus::Observed,
                confidence: 1.0,
                subject: None,
                created_at: "2026-08-28T18:00:00Z".to_owned(),
                updated_at: "2026-08-28T23:00:00Z".to_owned(),
                metadata: HashMap::new(),
            },
        ];

        let default_edges = vec![
            CognitiveEdgeRecord {
                id: "edge-01".to_owned(),
                source_id: "node:sec:landlock-v3".to_owned(),
                target_id: "node:agent:opencode-main".to_owned(),
                edge_type: CognitiveEdgeType::Governs,
                weight: 1.0,
                description: "Landlock LSM strictly confines OpenCode capsule filesystem access".to_owned(),
            },
            CognitiveEdgeRecord {
                id: "edge-02".to_owned(),
                source_id: "node:agent:opencode-main".to_owned(),
                target_id: "node:service:cybou-web-gateway".to_owned(),
                edge_type: CognitiveEdgeType::Observes,
                weight: 0.95,
                description: "OpenCode interacts with CYBOU REST API & events socket".to_owned(),
            },
            CognitiveEdgeRecord {
                id: "edge-03".to_owned(),
                source_id: "node:service:cybou-actiond".to_owned(),
                target_id: "node:finding:audit-pass".to_owned(),
                edge_type: CognitiveEdgeType::Causes,
                weight: 1.0,
                description: "Action1 evaluation derives audit pass confirmation".to_owned(),
            },
            CognitiveEdgeRecord {
                id: "edge-04".to_owned(),
                source_id: "node:mail:audit-report".to_owned(),
                target_id: "node:finding:audit-pass".to_owned(),
                edge_type: CognitiveEdgeType::References,
                weight: 0.9,
                description: "Automated audit dispatch emailed to operator".to_owned(),
            },
            CognitiveEdgeRecord {
                id: "edge-05".to_owned(),
                source_id: "node:contact:elena".to_owned(),
                target_id: "node:note:invariants".to_owned(),
                edge_type: CognitiveEdgeType::References,
                weight: 0.85,
                description: "Elena authored Living Canvas spatial invariants".to_owned(),
            },
            CognitiveEdgeRecord {
                id: "edge-06".to_owned(),
                source_id: "node:note:invariants".to_owned(),
                target_id: "node:service:cybou-web-gateway".to_owned(),
                edge_type: CognitiveEdgeType::DerivesFrom,
                weight: 0.92,
                description: "Gateway deck and cluster engine derives from spatial invariants".to_owned(),
            },
        ];

        let default_journal = vec![
            EventJournalEntry {
                event_id: "evt-jnl-001".to_owned(),
                causation_id: None,
                correlation_id: "corr-init-01".to_owned(),
                origin_organ: "systemd".to_owned(),
                event_type: "UnitStarted".to_owned(),
                summary: "Started CYBOU Web Gateway Service".to_owned(),
                payload_preview: "{\"unit\":\"cybou-web-gateway.service\",\"status\":\"active\"}".to_owned(),
                timestamp: "2026-08-28T18:00:00Z".to_owned(),
                subject: Some(SubjectRef::Service {
                    name: "cybou-web-gateway.service".to_owned(),
                    node_id: None,
                }),
                epistemic_status: EpistemicStatus::Observed,
            },
            EventJournalEntry {
                event_id: "evt-jnl-002".to_owned(),
                causation_id: Some("evt-jnl-001".to_owned()),
                correlation_id: "corr-init-01".to_owned(),
                origin_organ: "actiond".to_owned(),
                event_type: "ConfinementEnforced".to_owned(),
                summary: "Enforced Landlock v3 rules on agent runtime".to_owned(),
                payload_preview: "{\"policy\":\"landlock-capsule-strict\",\"abi\":3}".to_owned(),
                timestamp: "2026-08-28T19:00:00Z".to_owned(),
                subject: None,
                epistemic_status: EpistemicStatus::Observed,
            },
            EventJournalEntry {
                event_id: "evt-jnl-003".to_owned(),
                causation_id: Some("evt-jnl-002".to_owned()),
                correlation_id: "corr-agent-01".to_owned(),
                origin_organ: "agentd".to_owned(),
                event_type: "CapsuleSpawned".to_owned(),
                summary: "Spawned OpenCode Refactor Agent capsule".to_owned(),
                payload_preview: "{\"capsule_id\":\"capsule-opencode-01\",\"model\":\"claude-3-5-sonnet\"}".to_owned(),
                timestamp: "2026-08-28T20:00:00Z".to_owned(),
                subject: Some(SubjectRef::Agent {
                    capsule_id: "capsule-opencode-01".to_owned(),
                    agent_type: "opencode-main".to_owned(),
                }),
                epistemic_status: EpistemicStatus::Observed,
            },
            EventJournalEntry {
                event_id: "evt-jnl-004".to_owned(),
                causation_id: Some("evt-jnl-003".to_owned()),
                correlation_id: "corr-audit-01".to_owned(),
                origin_organ: "securityd".to_owned(),
                event_type: "AuditReportGenerated".to_owned(),
                summary: "Security audit verified: 0 violations across active processes".to_owned(),
                payload_preview: "{\"violations\":0,\"checked_processes\":4}".to_owned(),
                timestamp: "2026-08-28T21:40:00Z".to_owned(),
                subject: None,
                epistemic_status: EpistemicStatus::Observed,
            },
        ];

        Self {
            nodes: RwLock::new(default_nodes),
            edges: RwLock::new(default_edges),
            journal: RwLock::new(default_journal),
        }
    }

    /// Retrieve the full or focused Cognitive Graph.
    #[must_use]
    pub fn get_graph(&self, focus_node_id: Option<String>) -> CognitiveGraphProjection {
        let nodes = self.nodes.read().expect("read nodes").clone();
        let edges = self.edges.read().expect("read edges").clone();
        CognitiveGraphProjection {
            schema_version: WEB_SCHEMA_V1,
            graph: CognitiveGraphRecord { nodes, edges },
            focus_node_id,
        }
    }

    /// Query and filter the Cognitive Graph by term, node type, and traversal depth.
    #[must_use]
    pub fn query_graph(&self, req: CognitiveQueryRequest) -> CognitiveGraphProjection {
        let q = req.query.trim().to_lowercase();
        let all_nodes = self.nodes.read().expect("read nodes");
        let all_edges = self.edges.read().expect("read edges");

        let matching_nodes: Vec<CognitiveNodeRecord> = all_nodes
            .iter()
            .filter(|n| {
                if q.is_empty() {
                    return true;
                }
                n.id.to_lowercase().contains(&q)
                    || n.label.to_lowercase().contains(&q)
                    || n.node_type.category_name().to_lowercase().contains(&q)
            })
            .cloned()
            .collect();

        let matching_ids: std::collections::HashSet<String> =
            matching_nodes.iter().map(|n| n.id.clone()).collect();

        let matching_edges: Vec<CognitiveEdgeRecord> = all_edges
            .iter()
            .filter(|e| matching_ids.contains(&e.source_id) || matching_ids.contains(&e.target_id))
            .cloned()
            .collect();

        CognitiveGraphProjection {
            schema_version: WEB_SCHEMA_V1,
            graph: CognitiveGraphRecord {
                nodes: matching_nodes,
                edges: matching_edges,
            },
            focus_node_id: req.focus_id,
        }
    }

    /// Retrieve the Canonical Event1 Journal entries.
    #[must_use]
    pub fn get_journal(&self, limit: Option<usize>, offset: Option<usize>) -> EventJournalProjection {
        let all = self.journal.read().expect("read journal");
        let off = offset.unwrap_or(0);
        let lim = limit.unwrap_or(50);
        let total_count = all.len();
        let entries: Vec<EventJournalEntry> = all
            .iter()
            .rev()
            .skip(off)
            .take(lim)
            .cloned()
            .collect();

        EventJournalProjection {
            schema_version: WEB_SCHEMA_V1,
            entries,
            total_count,
        }
    }

    /// Record a new canonical Event1 journal entry.
    pub fn record_event(&self, entry: EventJournalEntry) {
        let mut jnl = self.journal.write().expect("write journal");
        jnl.push(entry);
    }
}
