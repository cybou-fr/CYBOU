// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Lifelong Learning & Governance HTTP endpoints.

use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
    http::StatusCode,
};
use cybou_protocol::learning::LearningLayer;
use cybou_web_contracts::{
    CandidateEvaluationProjection, GovernanceScopesProjection, LearnedArtifactsProjection,
    LearningCandidatesProjection, ProposeLearningCandidateRequest, RevokeArtifactRequest,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::learning_hub::LearningError;
use crate::state::GatewayState;

fn mutation_error(
    error: LearningError,
    not_found: &'static str,
) -> (StatusCode, Json<crate::state::ErrorBody>) {
    let (status, code, retryable) = match error {
        LearningError::NotFound => (StatusCode::NOT_FOUND, not_found, false),
        LearningError::Persistence(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "learningPersistenceFailed",
            true,
        ),
    };
    (
        status,
        Json(crate::state::ErrorBody {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            error: code,
            retryable,
        }),
    )
}

/// Query parameters for filtering learning candidates.
#[derive(Debug, Deserialize)]
pub struct CandidateFilterQuery {
    /// Optional learning layer name filter.
    pub layer: Option<String>,
}

/// Retrieve learning candidates.
pub async fn get_candidates_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Query(query): Query<CandidateFilterQuery>,
) -> Result<Json<LearningCandidatesProjection>, (StatusCode, Json<crate::state::ErrorBody>)> {
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required());
    }

    let layer_filter = query.layer.as_deref().and_then(|l| match l {
        "procedural" => Some(LearningLayer::Procedural),
        "behavioral" => Some(LearningLayer::Behavioral),
        "epistemic" => Some(LearningLayer::Epistemic),
        "associative" => Some(LearningLayer::Associative),
        "neural" => Some(LearningLayer::Neural),
        "episodic" => Some(LearningLayer::Episodic),
        _ => None,
    });

    let projection = state.learning.get_candidates(layer_filter);
    Ok(Json(projection))
}

/// Propose a new learning candidate.
pub async fn propose_candidate_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<ProposeLearningCandidateRequest>,
) -> Result<
    (
        StatusCode,
        Json<cybou_protocol::learning::LearningCandidate>,
    ),
    (StatusCode, Json<crate::state::ErrorBody>),
> {
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required());
    }

    let candidate = state
        .learning
        .propose_candidate(request)
        .map_err(|error| mutation_error(error, "candidateNotFound"))?;
    Ok((StatusCode::CREATED, Json(candidate)))
}

/// Evaluate a candidate against demonstrated episodic outcomes and promotion criteria.
pub async fn evaluate_candidate_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Path(candidate_id): Path<Uuid>,
) -> Result<Json<CandidateEvaluationProjection>, (StatusCode, Json<crate::state::ErrorBody>)> {
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required());
    }

    match state.learning.evaluate_candidate(candidate_id) {
        Ok(projection) => Ok(Json(projection)),
        Err(error) => Err(mutation_error(error, "candidateNotFound")),
    }
}

/// Retrieve promoted durable artifacts and lineages.
pub async fn get_artifacts_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<LearnedArtifactsProjection>, (StatusCode, Json<crate::state::ErrorBody>)> {
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required());
    }

    let projection = state.learning.get_artifacts();
    Ok(Json(projection))
}

/// Revoke or deprecate a promoted artifact.
pub async fn revoke_artifact_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Path(artifact_id): Path<Uuid>,
    Json(request): Json<RevokeArtifactRequest>,
) -> Result<StatusCode, (StatusCode, Json<crate::state::ErrorBody>)> {
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required());
    }

    let mut req = request;
    req.artifact_id = artifact_id;

    state
        .learning
        .revoke_artifact(req)
        .map_err(|error| mutation_error(error, "artifactNotFound"))?;
    Ok(StatusCode::NO_CONTENT)
}

/// Retrieve active task scopes and capability grants.
pub async fn get_governance_scopes_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<GovernanceScopesProjection>, (StatusCode, Json<crate::state::ErrorBody>)> {
    if !state.may_read_mind(&headers) {
        return Err(GatewayState::sign_in_required());
    }

    let projection = state.learning.get_scopes();
    Ok(Json(projection))
}
