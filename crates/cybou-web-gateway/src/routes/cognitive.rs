// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! HTTP endpoints for the Canonical Event1 Journal & Deep Cognitive Graph (Milestone 7).

use axum::{
    Json,
    extract::{Query, State},
};
use cybou_web_contracts::{
    CognitiveGraphProjection, CognitiveQueryRequest, EventJournalProjection,
};
use serde::Deserialize;

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
pub async fn get_cognitive_graph(
    State(state): State<GatewayState>,
    Query(query): Query<GraphQuery>,
) -> Result<Json<CognitiveGraphProjection>, GatewayError> {
    Ok(Json(state.cognitive.get_graph(query.focus)))
}

/// POST `/api/v1/cognitive/query`
pub async fn query_cognitive_graph(
    State(state): State<GatewayState>,
    Json(request): Json<CognitiveQueryRequest>,
) -> Result<Json<CognitiveGraphProjection>, GatewayError> {
    Ok(Json(state.cognitive.query_graph(request)))
}

/// GET `/api/v1/cognitive/journal`
pub async fn get_event_journal(
    State(state): State<GatewayState>,
    Query(query): Query<JournalQuery>,
) -> Result<Json<EventJournalProjection>, GatewayError> {
    Ok(Json(state.cognitive.get_journal(query.limit, query.offset)))
}

#[cfg(test)]
mod tests {
    use crate::cognitive_hub::CognitiveHub;
    use cybou_protocol::cognitive::EventJournalEntry;
    use cybou_protocol::epistemic::EpistemicStatus;
    use cybou_web_contracts::CognitiveQueryRequest;

    #[test]
    fn cognitive_hub_serves_graph_and_journal() {
        let hub = CognitiveHub::new();

        let graph = hub.get_graph(None);
        assert!(graph.graph.nodes.is_empty());

        hub.record_event(EventJournalEntry {
            event_id: "evt-001".to_owned(),
            causation_id: None,
            correlation_id: "corr-001".to_owned(),
            origin_organ: "systemd".to_owned(),
            event_type: "UnitActive".to_owned(),
            summary: "Service started".to_owned(),
            payload_preview: "{}".to_owned(),
            timestamp: "2026-08-29T12:00:00Z".to_owned(),
            subject: None,
            epistemic_status: EpistemicStatus::Observed,
        });

        let journal = hub.get_journal(Some(10), None);
        assert_eq!(journal.total_count, 1);
        assert_eq!(journal.entries[0].event_id, "evt-001");

        let queried = hub.query_graph(CognitiveQueryRequest {
            query: "OpenCode".to_owned(),
            node_types: None,
            focus_id: None,
            max_depth: None,
        });
        assert!(queried.graph.nodes.is_empty());
    }
}
