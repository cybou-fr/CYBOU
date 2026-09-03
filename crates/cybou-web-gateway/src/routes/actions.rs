// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Action1 records, the self-healing lifecycle, and the one answer a person can give (ADR-0022).
//!
//! Exposes what was proposed, authorized, executed, and independently observed by telemetry.
//!
//! The gateway still cannot execute anything and still decides nothing. What it can now do is
//! carry one answer: it is the only party that authenticated the person, so it is the only party
//! that can say who is confirming. Every check on whether that answer authorizes anything belongs
//! to Action1, and the permit that follows one never comes back through this boundary.

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
};
use cybou_web_contracts::ConfirmActionRequest;
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
                detail: None,
            }),
        )),
    }
}

/// Answer a proposal that was waiting on a person.
///
/// The seat is established here and the answer is decided there. This handler supplies exactly
/// two things Action1 cannot know: who is at the keyboard, and that they are entitled to be
/// asked at all. It supplies no operation, no target and no argument — those are on the proposal
/// Action1 already holds, and there is no field on this request for them.
///
/// A public reader holds no seat and is refused before Action1 hears about it. Action1 refusing —
/// because the verdict is no longer the one that asked, because the decision on screen is stale,
/// because a critic objected, or because the proposal is older than the readings behind it — is
/// reported as one refusal, for the reason Action1 gives one.
///
/// # Errors
///
/// Refuses with `403` when the request holds no seat, and `409` when Action1 did not accept the
/// answer or could not be reached.
pub async fn confirm_action_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<ConfirmActionRequest>,
) -> Result<Json<ActionRecordProjection>, (StatusCode, Json<crate::state::ErrorBody>)> {
    let Some(principal) = state.authenticated_principal(&headers) else {
        return Err((
            StatusCode::FORBIDDEN,
            Json(crate::state::ErrorBody {
                schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
                error: "confirmationRequiresASeat",
                retryable: false,
                detail: None,
            }),
        ));
    };

    state
        .presence
        .confirm_action(request.proposal_id, request.decision_id, &principal)
        .await
        .map(Json)
        .ok_or((
            // Not 503. Action1 answering "that is not a proposal awaiting this answer" and Action1
            // being unreachable are both reported here, because the deliberate design of that
            // refusal is that it does not say which of its four checks failed — and a gateway that
            // split it into a retryable and a non-retryable status would say it for Action1.
            StatusCode::CONFLICT,
            Json(crate::state::ErrorBody {
                schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
                error: "confirmationNotAccepted",
                retryable: false,
                detail: None,
            }),
        ))
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
                detail: None,
            }),
        )),
    }
}
