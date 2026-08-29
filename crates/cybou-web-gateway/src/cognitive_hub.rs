// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Grounded provider and governor for the Canonical Event1 Journal & Deep Cognitive Graph (Milestone 7).

use std::collections::HashSet;
use std::sync::RwLock;
use cybou_protocol::cognitive::{
    CognitiveEdgeRecord, CognitiveGraphRecord, CognitiveNodeRecord, EventJournalEntry,
};
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
    /// Build a new `CognitiveHub` with honest initial state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(Vec::new()),
            edges: RwLock::new(Vec::new()),
            journal: RwLock::new(Vec::new()),
        }
    }

    /// Retrieve the full or focused Cognitive Graph.
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

        let matching_ids: HashSet<String> =
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
        let all = self.journal.read().unwrap_or_else(|e| e.into_inner());
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

    /// Record a real canonical Event1 journal entry from observed events.
    pub fn record_event(&self, entry: EventJournalEntry) {
        let mut jnl = self.journal.write().unwrap_or_else(|e| e.into_inner());
        jnl.push(entry);
    }
}
