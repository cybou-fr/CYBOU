// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Meaning & Dialogue HTTP endpoints.

use axum::{
    Json,
    extract::State,
    http::HeaderMap,
    http::StatusCode,
};
use cybou_web_contracts::{
    DialogueMemoryProjection, MeaningInterpretProjection, MeaningInterpretRequest,
};

use crate::state::GatewayState;

/// Interpret an utterance into a typed cognitive act and produce a qualified response.
pub async fn interpret_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<MeaningInterpretRequest>,
) -> Result<
    Json<MeaningInterpretProjection>,
    (StatusCode, Json<crate::state::ErrorBody>),
> {
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required());
    }

    let projection = state.meaning.process_utterance(&request);
    Ok(Json(projection))
}

/// Retrieve dialogue memory state.
pub async fn dialogue_memory_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<
    Json<DialogueMemoryProjection>,
    (StatusCode, Json<crate::state::ErrorBody>),
> {
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required());
    }

    let projection = state.meaning.dialogue_memory();
    Ok(Json(projection))
}
