// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Grounded provider and governor for the Canonical Event1 Journal & Deep Cognitive Graph (Milestone 7).

use cybou_protocol::cognitive::{
    CognitiveEdgeRecord, CognitiveEdgeType, CognitiveGraphRecord, CognitiveNodeRecord,
    CognitiveNodeType, CognitiveProvenance,
};
use cybou_protocol::epistemic::EpistemicStatus;
use cybou_protocol::subject::SubjectRef;
use cybou_protocol::system::{ProcessRecord, ServiceRecord};
use cybou_web_contracts::{
    CognitiveGraphProjection, CognitiveQueryRequest, MindProjection, WEB_SCHEMA_V1,
};
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use time::OffsetDateTime;

/// Gateway-side projection builder for the deep cross-subsystem Cognitive Graph.
pub struct CognitiveHub {
    nodes: RwLock<Vec<CognitiveNodeRecord>>,
    edges: RwLock<Vec<CognitiveEdgeRecord>>,
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
        }
    }

    /// Retrieve the full or focused Cognitive Graph from custom in-memory entries.
    #[must_use]
    pub fn get_graph(&self, focus_node_id: Option<String>) -> CognitiveGraphProjection {
        let nodes = self
            .nodes
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let edges = self
            .edges
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
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

        // 1. Real Systemd Services
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
                provenance: CognitiveProvenance::Observed,
                evidence_ids: Vec::new(),
                observed_at: Some(now_str.clone()),
                subject: Some(SubjectRef::Service {
                    name: svc.name.clone(),
                    node_id: None,
                }),
                created_at: now_str.clone(),
                updated_at: now_str.clone(),
                metadata: meta,
            });
        }

        // 2. Top Host Processes (from live /proc)
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
                provenance: CognitiveProvenance::Observed,
                evidence_ids: Vec::new(),
                observed_at: Some(now_str.clone()),
                subject: Some(SubjectRef::Process {
                    pid: proc.pid,
                    name: proc.name.clone(),
                }),
                created_at: now_str.clone(),
                updated_at: now_str.clone(),
                metadata: meta,
            });
        }

        // 3. Epistemic Beliefs from Mind
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
                    provenance: CognitiveProvenance::Derived,
                    evidence_ids: Vec::new(),
                    observed_at: Some(belief.last_corroborated_at.clone()),
                    subject: None,
                    created_at: belief.last_corroborated_at.clone(),
                    updated_at: belief.last_corroborated_at.clone(),
                    metadata: meta,
                });
            }
        }

        // 4. Connect declared architectural daemon edges only when both runtime nodes were observed.
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
                    provenance: CognitiveProvenance::Architectural,
                    evidence_ids: Vec::new(),
                    observed_at: None,
                    description: desc.to_owned(),
                });
                edge_seq += 1;
            }
        }

        // Merge any custom in-memory nodes/edges
        let custom_nodes = self
            .nodes
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let custom_edges = self
            .edges
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    ///
    /// Every declared parameter is honoured, so the control a person sees means what it says:
    /// a term (or an explicit focus) picks the starting nodes, `node_types` constrains which
    /// categories may appear at all, and `max_depth` bounds a typed breadth-first walk outwards
    /// from those starting nodes. Edges are returned only between nodes the projection contains.
    #[must_use]
    pub fn query_graph(&self, req: CognitiveQueryRequest) -> CognitiveGraphProjection {
        let q = req.query.trim().to_lowercase();
        let all_nodes = self
            .nodes
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let all_edges = self
            .edges
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let permitted: Option<Vec<String>> = req.node_types.as_ref().map(|types| {
            types
                .iter()
                .map(|value| value.trim().to_lowercase())
                .collect()
        });
        let type_permitted = |node: &CognitiveNodeRecord| {
            permitted.as_ref().is_none_or(|types| {
                let category = node.node_type.category_name().to_lowercase();
                types.iter().any(|value| *value == category)
            })
        };

        let focus = req
            .focus_id
            .as_ref()
            .map(|value| value.trim().to_lowercase())
            .filter(|value| !value.is_empty());
        let matches_term = |node: &CognitiveNodeRecord| {
            if let Some(focus) = focus.as_ref() {
                return node.id.to_lowercase() == *focus
                    || node.id.to_lowercase().contains(focus)
                    || node.label.to_lowercase().contains(focus);
            }
            q.is_empty()
                || node.id.to_lowercase().contains(&q)
                || node.label.to_lowercase().contains(&q)
                || node.node_type.category_name().to_lowercase().contains(&q)
        };

        let mut selected: HashSet<String> = all_nodes
            .iter()
            .filter(|node| type_permitted(node) && matches_term(node))
            .map(|node| node.id.clone())
            .collect();

        // Typed breadth-first expansion: a neighbour joins only if its own category is permitted,
        // so a type constraint is not silently widened by traversal.
        let mut frontier: Vec<String> = selected.iter().cloned().collect();
        for _ in 0..req.max_depth.unwrap_or(0) {
            if frontier.is_empty() {
                break;
            }
            let mut next = Vec::new();
            for edge in all_edges.iter() {
                let neighbour = if frontier.contains(&edge.source_id) {
                    Some(&edge.target_id)
                } else if frontier.contains(&edge.target_id) {
                    Some(&edge.source_id)
                } else {
                    None
                };
                let Some(neighbour) = neighbour else { continue };
                if selected.contains(neighbour) {
                    continue;
                }
                let permitted_neighbour = all_nodes
                    .iter()
                    .find(|node| node.id == *neighbour)
                    .is_some_and(type_permitted);
                if permitted_neighbour {
                    selected.insert(neighbour.clone());
                    next.push(neighbour.clone());
                }
            }
            frontier = next;
        }

        let nodes: Vec<CognitiveNodeRecord> = all_nodes
            .iter()
            .filter(|node| selected.contains(&node.id))
            .cloned()
            .collect();
        // An edge to a node this projection does not contain would draw a relation to nothing.
        let edges: Vec<CognitiveEdgeRecord> = all_edges
            .iter()
            .filter(|edge| {
                selected.contains(&edge.source_id) && selected.contains(&edge.target_id)
            })
            .cloned()
            .collect();

        CognitiveGraphProjection {
            schema_version: WEB_SCHEMA_V1,
            graph: CognitiveGraphRecord { nodes, edges },
            focus_node_id: req.focus_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CognitiveHub;
    use cybou_protocol::{
        cognitive::{
            CognitiveEdgeRecord, CognitiveEdgeType, CognitiveNodeRecord, CognitiveNodeType,
            CognitiveProvenance,
        },
        epistemic::EpistemicStatus,
    };
    use cybou_web_contracts::CognitiveQueryRequest;
    use std::collections::HashMap;

    fn node(id: &str, label: &str, node_type: CognitiveNodeType) -> CognitiveNodeRecord {
        CognitiveNodeRecord {
            id: id.to_owned(),
            label: label.to_owned(),
            node_type,
            epistemic_status: EpistemicStatus::Observed,
            confidence: 1.0,
            provenance: CognitiveProvenance::Observed,
            evidence_ids: Vec::new(),
            observed_at: None,
            subject: None,
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            updated_at: "2026-01-01T00:00:00Z".to_owned(),
            metadata: HashMap::new(),
        }
    }

    fn edge(id: &str, source: &str, target: &str) -> CognitiveEdgeRecord {
        CognitiveEdgeRecord {
            id: id.to_owned(),
            source_id: source.to_owned(),
            target_id: target.to_owned(),
            edge_type: CognitiveEdgeType::Observes,
            weight: 1.0,
            provenance: CognitiveProvenance::Observed,
            evidence_ids: Vec::new(),
            observed_at: None,
            description: String::new(),
        }
    }

    fn populated() -> CognitiveHub {
        let hub = CognitiveHub::new();
        {
            let mut nodes = hub.nodes.write().expect("write nodes");
            nodes.push(node(
                "node:service:gateway",
                "cybou-web-gateway",
                CognitiveNodeType::Service {
                    name: "cybou-web-gateway".to_owned(),
                    state: "active".to_owned(),
                },
            ));
            nodes.push(node(
                "node:process:41",
                "gateway worker",
                CognitiveNodeType::Process {
                    pid: 41,
                    name: "cybou-web-gateway".to_owned(),
                },
            ));
            nodes.push(node(
                "node:service:agentd",
                "cybou-agentd",
                CognitiveNodeType::Service {
                    name: "cybou-agentd".to_owned(),
                    state: "active".to_owned(),
                },
            ));
            let mut edges = hub.edges.write().expect("write edges");
            edges.push(edge("edge:1", "node:service:gateway", "node:process:41"));
            edges.push(edge("edge:2", "node:process:41", "node:service:agentd"));
        }
        hub
    }

    #[test]
    fn a_depth_of_zero_returns_only_what_the_term_itself_matched() {
        let queried = populated().query_graph(CognitiveQueryRequest {
            query: "cybou-web-gateway".to_owned(),
            node_types: None,
            focus_id: None,
            max_depth: None,
        });
        let ids: Vec<&str> = queried
            .graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect();
        assert_eq!(ids, vec!["node:service:gateway"]);
        // Its neighbour was not selected, so no relation is drawn to a node this projection does
        // not contain.
        assert!(queried.graph.edges.is_empty());
    }

    #[test]
    fn traversal_walks_exactly_as_far_as_the_requested_depth() {
        let hub = populated();
        let one = hub.query_graph(CognitiveQueryRequest {
            query: "node:service:gateway".to_owned(),
            node_types: None,
            focus_id: None,
            max_depth: Some(1),
        });
        assert_eq!(one.graph.nodes.len(), 2);
        let two = hub.query_graph(CognitiveQueryRequest {
            query: "node:service:gateway".to_owned(),
            node_types: None,
            focus_id: None,
            max_depth: Some(2),
        });
        assert_eq!(two.graph.nodes.len(), 3);
    }

    #[test]
    fn a_type_constraint_is_not_widened_by_traversal() {
        let queried = populated().query_graph(CognitiveQueryRequest {
            query: String::new(),
            node_types: Some(vec!["Service".to_owned()]),
            focus_id: None,
            max_depth: Some(3),
        });
        assert!(
            queried
                .graph
                .nodes
                .iter()
                .all(|node| node.node_type.category_name() == "Service")
        );
        // The two services are only related through a process the constraint excludes, so no edge
        // may be drawn between them.
        assert!(queried.graph.edges.is_empty());
    }

    #[test]
    fn an_explicit_focus_selects_the_starting_node() {
        let queried = populated().query_graph(CognitiveQueryRequest {
            query: "cybou".to_owned(),
            node_types: None,
            focus_id: Some("node:service:agentd".to_owned()),
            max_depth: None,
        });
        let ids: Vec<&str> = queried
            .graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect();
        assert_eq!(ids, vec!["node:service:agentd"]);
        assert_eq!(queried.focus_node_id.as_deref(), Some("node:service:agentd"));
    }
}
