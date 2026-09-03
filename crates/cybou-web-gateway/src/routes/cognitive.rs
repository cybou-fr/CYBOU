// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! HTTP endpoints for the Canonical Event1 Journal & Deep Cognitive Graph (Milestone 7).

use axum::{
    Json,
    extract::{Query, State},
};
use cybou_protocol::cognitive::{
    CognitiveEdgeRecord, CognitiveGraphRecord, CognitiveNodeRecord, EventJournalEntry,
};
use cybou_web_contracts::{
    CognitiveGraphProjection, CognitiveQueryRequest, EventJournalProjection, WEB_SCHEMA_V1,
};
use serde::Deserialize;
use std::collections::HashSet;

use crate::state::{GatewayError, GatewayState};

/// Query parameters for Cognitive Graph.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphQuery {
    /// Focus subject identifier or domain prefix.
    pub focus: Option<String>,
}

/// Query parameters for Event1 Journal pagination.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalQuery {
    /// Maximum number of events to return.
    pub limit: Option<usize>,
    /// Pagination offset.
    pub offset: Option<usize>,
}

/// GET `/api/v1/cognitive/graph`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn get_cognitive_graph(
    State(state): State<GatewayState>,
    Query(query): Query<GraphQuery>,
) -> Result<Json<CognitiveGraphProjection>, GatewayError> {
    let services = state.system.list_services();
    let procs = state.system.list_processes();
    let mind = state.presence.mind().await.ok();

    Ok(Json(state.cognitive.build_grounded_graph(
        &services.services,
        &procs.processes,
        mind.as_ref(),
        query.focus,
    )))
}

/// POST `/api/v1/cognitive/query`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn query_cognitive_graph(
    State(state): State<GatewayState>,
    Json(request): Json<CognitiveQueryRequest>,
) -> Result<Json<CognitiveGraphProjection>, GatewayError> {
    let services = state.system.list_services();
    let procs = state.system.list_processes();
    let mind = state.presence.mind().await.ok();
    let full = state.cognitive.build_grounded_graph(
        &services.services,
        &procs.processes,
        mind.as_ref(),
        request.focus_id.clone(),
    );

    let q = request.query.trim().to_lowercase();
    if q.is_empty() {
        return Ok(Json(full));
    }

    let matching_nodes: Vec<CognitiveNodeRecord> = full
        .graph
        .nodes
        .into_iter()
        .filter(|n| {
            n.id.to_lowercase().contains(&q)
                || n.label.to_lowercase().contains(&q)
                || n.node_type.category_name().to_lowercase().contains(&q)
        })
        .collect();

    let matching_ids: HashSet<String> = matching_nodes.iter().map(|n| n.id.clone()).collect();
    let matching_edges: Vec<CognitiveEdgeRecord> = full
        .graph
        .edges
        .into_iter()
        .filter(|e| matching_ids.contains(&e.source_id) || matching_ids.contains(&e.target_id))
        .collect();

    Ok(Json(CognitiveGraphProjection {
        schema_version: WEB_SCHEMA_V1,
        graph: CognitiveGraphRecord {
            nodes: matching_nodes,
            edges: matching_edges,
        },
        focus_node_id: request.focus_id,
    }))
}

/// GET `/api/v1/cognitive/journal`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn get_event_journal(
    State(state): State<GatewayState>,
    Query(query): Query<JournalQuery>,
) -> Result<Json<EventJournalProjection>, GatewayError> {
    let mind = state.presence.mind().await?;
    let journal = &mind.journal;
    let total_count = journal
        .contribution_count
        .map_or(journal.recent.len(), |count| {
            usize::try_from(count).unwrap_or(usize::MAX)
        });
    let entries = journal
        .recent
        .iter()
        .rev()
        .skip(query.offset.unwrap_or(0))
        .take(query.limit.unwrap_or(50))
        .map(|contribution| EventJournalEntry {
            event_id: contribution.message_id.clone(),
            causation_id: None,
            correlation_id: contribution.message_id.clone(),
            origin_organ: contribution.origin_organ.clone(),
            event_type: contribution.kind.clone(),
            summary: format!(
                "{} contribution from {}",
                contribution.kind, contribution.origin_organ
            ),
            payload_preview: "Canonical Event1 Journal Contribution".to_owned(),
            timestamp: contribution.recorded_at.clone(),
            subject: None,
            epistemic_status: cybou_protocol::epistemic::EpistemicStatus::Observed,
        })
        .collect();

    Ok(Json(EventJournalProjection {
        schema_version: WEB_SCHEMA_V1,
        entries,
        total_count,
    }))
}

#[cfg(test)]
mod tests {
    use crate::cognitive_hub::CognitiveHub;
    use cybou_web_contracts::CognitiveQueryRequest;

    #[test]
    fn cognitive_hub_starts_with_no_unowned_graph_state() {
        let hub = CognitiveHub::new();

        let graph = hub.get_graph(None);
        assert!(graph.graph.nodes.is_empty());

        let queried = hub.query_graph(CognitiveQueryRequest {
            query: "OpenCode".to_owned(),
            node_types: None,
            focus_id: None,
            max_depth: None,
        });
        assert!(queried.graph.nodes.is_empty());

        // Test grounded graph construction
        let grounded = hub.build_grounded_graph(&[], &[], None, None);
        assert!(grounded.graph.nodes.is_empty());
    }
}
