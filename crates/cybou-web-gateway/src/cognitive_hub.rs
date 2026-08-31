// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Grounded provider and governor for the Canonical Event1 Journal & Deep Cognitive Graph (Milestone 7).

use cybou_protocol::cognitive::{
    CognitiveEdgeRecord, CognitiveEdgeType, CognitiveGraphRecord, CognitiveNodeRecord,
    CognitiveNodeType, EventJournalEntry,
};
use cybou_protocol::epistemic::EpistemicStatus;
use cybou_protocol::subject::SubjectRef;
use cybou_protocol::system::{ProcessRecord, ServiceRecord};
use cybou_web_contracts::{
    CognitiveGraphProjection, CognitiveQueryRequest, EventJournalProjection, MindProjection,
    WEB_SCHEMA_V1,
};
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use time::OffsetDateTime;

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
    /// Build a new `CognitiveHub` with honest initial state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(Vec::new()),
            edges: RwLock::new(Vec::new()),
            journal: RwLock::new(Vec::new()),
        }
    }

    /// Retrieve the full or focused Cognitive Graph from custom in-memory entries.
    #[must_use]
    pub fn get_graph(&self, focus_node_id: Option<String>) -> CognitiveGraphProjection {
        let nodes = self.nodes.read().unwrap_or_else(|e| e.into_inner()).clone();
        let edges = self.edges.read().unwrap_or_else(|e| e.into_inner()).clone();
        CognitiveGraphProjection {
            schema_version: WEB_SCHEMA_V1,
            graph: CognitiveGraphRecord { nodes, edges },
            focus_node_id,
        }
    }

    /// Build a grounded live graph synthesizing real observed host services, processes, and Mind state.
    #[must_use]
    pub fn build_grounded_graph(
        &self,
        services: &[ServiceRecord],
        processes: &[ProcessRecord],
        mind: Option<&MindProjection>,
        focus_node_id: Option<String>,
    ) -> CognitiveGraphProjection {
        let now_str = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();

        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut node_ids = HashSet::new();

        // 1. Host Root Path nodes
        let host_roots = [
            ("/home/demo", "Host Home (/home/demo)"),
            ("/etc/cybou", "Config Boundary (/etc/cybou)"),
        ];
        for (path, label) in host_roots {
            let id = format!("path:{path}");
            node_ids.insert(id.clone());
            nodes.push(CognitiveNodeRecord {
                id,
                label: label.to_owned(),
                node_type: CognitiveNodeType::HostPath {
                    path: path.to_owned(),
                },
                epistemic_status: EpistemicStatus::Observed,
                confidence: 1.0,
                subject: None,
                created_at: now_str.clone(),
                updated_at: now_str.clone(),
                metadata: HashMap::new(),
            });
        }

        // 2. Real Systemd Services
        for svc in services {
            let id = format!("service:{}", svc.name);
            node_ids.insert(id.clone());
            let mut meta = HashMap::new();
            meta.insert("substate".to_owned(), svc.substate.clone());
            meta.insert(
                "active".to_owned(),
                (svc.state == cybou_protocol::system::ServiceState::Active).to_string(),
            );
            if let Some(pid) = svc.main_pid {
                meta.insert("pid".to_owned(), pid.to_string());
            }

            nodes.push(CognitiveNodeRecord {
                id: id.clone(),
                label: svc.name.clone(),
                node_type: CognitiveNodeType::Service {
                    name: svc.name.clone(),
                    state: svc.state.label().to_string(),
                },
                epistemic_status: EpistemicStatus::Observed,
                confidence: 1.0,
                subject: Some(SubjectRef::Service {
                    name: svc.name.clone(),
                    node_id: None,
                }),
                created_at: now_str.clone(),
                updated_at: now_str.clone(),
                metadata: meta,
            });
        }

        // 3. Top Host Processes (from live /proc)
        for proc in processes.iter().take(12) {
            let id = format!("process:{}", proc.pid);
            node_ids.insert(id.clone());
            let mut meta = HashMap::new();
            meta.insert("user".to_owned(), proc.user.clone());
            meta.insert("memoryBytes".to_owned(), proc.memory_bytes.to_string());
            meta.insert("cmdline".to_owned(), proc.cmdline.clone());

            nodes.push(CognitiveNodeRecord {
                id: id.clone(),
                label: format!("{} (pid {})", proc.name, proc.pid),
                node_type: CognitiveNodeType::Process {
                    pid: proc.pid,
                    name: proc.name.clone(),
                },
                epistemic_status: EpistemicStatus::Observed,
                confidence: 1.0,
                subject: Some(SubjectRef::Process {
                    pid: proc.pid,
                    name: proc.name.clone(),
                }),
                created_at: now_str.clone(),
                updated_at: now_str.clone(),
                metadata: meta,
            });
        }

        // 4. Epistemic Beliefs from Mind
        if let Some(mind_proj) = mind {
            for belief in &mind_proj.beliefs.beliefs {
                let id = format!("belief:{}", belief.subject);
                node_ids.insert(id.clone());
                let mut meta = HashMap::new();
                meta.insert("value".to_owned(), belief.value.clone());
                meta.insert("status".to_owned(), belief.status.clone());

                nodes.push(CognitiveNodeRecord {
                    id,
                    label: format!("{}: {}", belief.subject, belief.value),
                    node_type: CognitiveNodeType::Finding {
                        cause_id: belief.subject.clone(),
                        severity: "info".to_owned(),
                        title: format!("{}: {}", belief.subject, belief.value),
                    },
                    epistemic_status: if belief.status == "observed" {
                        EpistemicStatus::Observed
                    } else {
                        EpistemicStatus::Stale
                    },
                    confidence: belief.confidence,
                    subject: None,
                    created_at: belief.last_corroborated_at.clone(),
                    updated_at: belief.last_corroborated_at.clone(),
                    metadata: meta,
                });
            }
        }

        // 5. Connect known architectural daemon edges
        let daemon_relations = [
            (
                "service:cybou-web-gateway.service",
                "service:cybou-presenced.service",
                CognitiveEdgeType::Observes,
                "Web gateway observes Mind presence",
            ),
            (
                "service:cybou-presenced.service",
                "service:cybou-eventd.service",
                CognitiveEdgeType::Observes,
                "Presence observes canonical Event1 stream",
            ),
            (
                "service:cybou-actiond.service",
                "service:cybou-eventd.service",
                CognitiveEdgeType::Observes,
                "Action governor observes Event1 outcomes",
            ),
            (
                "service:cybou-actiond.service",
                "service:cybou-executord.service",
                CognitiveEdgeType::Governs,
                "Action governor authorizes typed execution permits",
            ),
            (
                "service:cybou-agentd.service",
                "path:/home/demo",
                CognitiveEdgeType::Governs,
                "Agent daemon confines workspaces under /home/demo",
            ),
            (
                "service:cybou-host-filesd@demo.service",
                "path:/home/demo",
                CognitiveEdgeType::Governs,
                "Host files daemon serves bounded per-user home socket",
            ),
        ];

        let mut edge_seq = 1;
        for (src, tgt, edge_type, desc) in daemon_relations {
            if node_ids.contains(src) && node_ids.contains(tgt) {
                edges.push(CognitiveEdgeRecord {
                    id: format!("edge-{edge_seq}"),
                    source_id: src.to_owned(),
                    target_id: tgt.to_owned(),
                    edge_type,
                    weight: 1.0,
                    description: desc.to_owned(),
                });
                edge_seq += 1;
            }
        }

        // Connect belief nodes to eventd
        let eventd_id = "service:cybou-eventd.service";
        if node_ids.contains(eventd_id) {
            for node in &nodes {
                if node.id.starts_with("belief:") {
                    edges.push(CognitiveEdgeRecord {
                        id: format!("edge-{edge_seq}"),
                        source_id: node.id.clone(),
                        target_id: eventd_id.to_owned(),
                        edge_type: CognitiveEdgeType::DerivesFrom,
                        weight: 0.9,
                        description: "Epistemic belief derived from canonical event journal"
                            .to_owned(),
                    });
                    edge_seq += 1;
                }
            }
        }

        // Merge any custom in-memory nodes/edges
        let custom_nodes = self.nodes.read().unwrap_or_else(|e| e.into_inner());
        let custom_edges = self.edges.read().unwrap_or_else(|e| e.into_inner());
        for cn in custom_nodes.iter() {
            if !node_ids.contains(&cn.id) {
                nodes.push(cn.clone());
            }
        }
        for ce in custom_edges.iter() {
            edges.push(ce.clone());
        }

        // Focus filtering if requested
        if let Some(ref focus) = focus_node_id {
            let focus_lower = focus.to_lowercase();
            let focus_matches: HashSet<String> = nodes
                .iter()
                .filter(|n| {
                    n.id.to_lowercase().contains(&focus_lower)
                        || n.label.to_lowercase().contains(&focus_lower)
                })
                .map(|n| n.id.clone())
                .collect();

            if !focus_matches.is_empty() {
                let relevant_edges: Vec<CognitiveEdgeRecord> = edges
                    .iter()
                    .filter(|e| {
                        focus_matches.contains(&e.source_id) || focus_matches.contains(&e.target_id)
                    })
                    .cloned()
                    .collect();

                let mut connected_node_ids = focus_matches;
                for e in &relevant_edges {
                    connected_node_ids.insert(e.source_id.clone());
                    connected_node_ids.insert(e.target_id.clone());
                }

                nodes.retain(|n| connected_node_ids.contains(&n.id));
                edges = relevant_edges;
            }
        }

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
        let all_nodes = self.nodes.read().unwrap_or_else(|e| e.into_inner());
        let all_edges = self.edges.read().unwrap_or_else(|e| e.into_inner());

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

        let matching_ids: HashSet<String> = matching_nodes.iter().map(|n| n.id.clone()).collect();

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
    pub fn get_journal(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> EventJournalProjection {
        let all = self.journal.read().unwrap_or_else(|e| e.into_inner());
        let off = offset.unwrap_or(0);
        let lim = limit.unwrap_or(50);
        let total_count = all.len();
        let entries: Vec<EventJournalEntry> =
            all.iter().rev().skip(off).take(lim).cloned().collect();

        EventJournalProjection {
            schema_version: WEB_SCHEMA_V1,
            entries,
            total_count,
        }
    }

    /// Record a real canonical Event1 journal entry from observed events.
    pub fn record_event(&self, entry: EventJournalEntry) {
        let mut jnl = self.journal.write().unwrap_or_else(|e| e.into_inner());
        jnl.push(entry);
    }
}
