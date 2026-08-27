// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Read-side projection of Action1 records and self-healing lifecycle (ADR-0022).
//!
//! Exposes what was proposed, authorized, executed, and independently observed by telemetry.
//! Bounded and read-only: the gateway carries the canonical Action1 projection and cannot execute
//! actions itself.

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::state::GatewayState;
use cybou_web_contracts::ActionRecordProjection;

/// Optional query parameters for filtering action projections.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionQuery {
    /// Filter by causal telemetry insight identity.
    pub cause: Option<Uuid>,
}

/// Return action records matching the query or recent records.
///
/// # Errors
///
/// Refuses with `401` when sign-in is required and no session exists.
pub async fn actions_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Query(query): Query<ActionQuery>,
) -> Result<Json<Vec<ActionRecordProjection>>, (StatusCode, Json<crate::state::ErrorBody>)> {
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required());
    }

    let records = match query.cause {
        Some(cause_id) => state.presence.actions_for_cause(cause_id).await,
        None => state.presence.recent_actions().await,
    };
    match records {
        Some(records) => Ok(Json(records)),
        None => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(crate::state::ErrorBody {
                schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
                error: "action1Unavailable",
                retryable: true,
            }),
        )),
    }
}

/// Return recent action lifecycle records.
///
/// # Errors
///
/// Refuses with `401` when sign-in is required and no session exists.
/// Returns `503` when Action1 is unavailable.
pub async fn recent_actions_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ActionRecordProjection>>, (StatusCode, Json<crate::state::ErrorBody>)> {
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required());
    }
    match state.presence.recent_actions().await {
        Some(records) => Ok(Json(records)),
        None => Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(crate::state::ErrorBody {
                schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
                error: "action1Unavailable",
                retryable: true,
            }),
        )),
    }
}
