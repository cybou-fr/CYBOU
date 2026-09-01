// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Meaning & Dialogue HTTP endpoints.

use axum::{Json, extract::State, http::HeaderMap};
use cybou_web_contracts::{
    DialogueMemoryProjection, MeaningInterpretProjection, MeaningInterpretRequest,
};

use crate::state::{GatewayError, GatewayState};

/// Interpret an utterance into a typed cognitive act and produce a qualified response.
pub async fn interpret_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<MeaningInterpretRequest>,
) -> Result<Json<MeaningInterpretProjection>, GatewayError> {
    let source = state
        .authenticated_principal(&headers)
        .ok_or(GatewayError::Refused)?;

    let projection = state.meaning.process_utterance(&request, &source).await?;
    Ok(Json(projection))
}

/// Retrieve dialogue memory state.
pub async fn dialogue_memory_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<DialogueMemoryProjection>, GatewayError> {
    let source = state
        .authenticated_principal(&headers)
        .ok_or(GatewayError::Refused)?;

    let projection = state.meaning.dialogue_memory(&source).await?;
    Ok(Json(projection))
}
