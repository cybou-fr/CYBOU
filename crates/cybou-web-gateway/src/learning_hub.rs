// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Learning & Governance Hub: Manages layered lifelong learning candidates,
//! deterministic promotion gates, durable artifact lineages, and task-scoped capability governance.

use cybou_protocol::governance::TaskScope;
use cybou_protocol::learning::{
    ArtifactStatus, LearnedArtifactLineage, LearningCandidate, LearningLayer, PromotionGate,
};
use cybou_protocol::promotion::{DemonstratedOutcome, evaluate_promotion};
use cybou_web_contracts::{
    CandidateEvaluationProjection, GovernanceScopesProjection, LearnedArtifactsProjection,
    LearningCandidatesProjection, ProposeLearningCandidateRequest, RevokeArtifactRequest,
    WEB_SCHEMA_V1,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct LearningStore {
    #[serde(default)]
    candidates: Vec<LearningCandidate>,
    #[serde(default)]
    demonstrations: Vec<(Uuid, Vec<DemonstratedOutcome>)>,
    #[serde(default)]
    artifacts: Vec<LearnedArtifactLineage>,
    #[serde(default)]
    scopes: Vec<TaskScope>,
}

/// Server-owned engine for candidate evaluation, artifact lineage, and capability grants.
pub struct LearningHub {
    store_path: Option<PathBuf>,
    candidates: Mutex<Vec<LearningCandidate>>,
    demonstrations: Mutex<Vec<(Uuid, Vec<DemonstratedOutcome>)>>,
    artifacts: Mutex<Vec<LearnedArtifactLineage>>,
    scopes: Mutex<Vec<TaskScope>>,
}

impl Default for LearningHub {
    fn default() -> Self {
        Self::new()
    }
}

impl LearningHub {
    /// Create a new `LearningHub` initialized with default store path if available.
    #[must_use]
    pub fn new() -> Self {
        let store_path = Self::default_store_path();
        Self::with_optional_store(store_path)
    }

    /// Determine default store path from environment or Linux path.
    #[must_use]
    pub fn default_store_path() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("CYBOU_LEARNING_STORE") {
            return Some(PathBuf::from(path));
        }
        #[cfg(target_os = "linux")]
        {
            let candidate = PathBuf::from("/var/lib/cybou/learning-store.json");
            if candidate.parent().is_some_and(std::path::Path::exists) {
                return Some(candidate);
            }
        }
        None
    }

    /// Construct `LearningHub` with an optional backing store path.
    #[must_use]
    pub fn with_optional_store(store_path: Option<PathBuf>) -> Self {
        let mut loaded = LearningStore::default();
        if let Some(ref path) = store_path
            && let Ok(bytes) = std::fs::read(path)
            && let Ok(parsed) = serde_json::from_slice::<LearningStore>(&bytes)
        {
            loaded = parsed;
        }

        Self {
            store_path,
            candidates: Mutex::new(loaded.candidates),
            demonstrations: Mutex::new(loaded.demonstrations),
            artifacts: Mutex::new(loaded.artifacts),
            scopes: Mutex::new(loaded.scopes),
        }
    }

    fn persist(&self) {
        let Some(ref path) = self.store_path else {
            return;
        };

        let store = LearningStore {
            candidates: self
                .candidates
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
            demonstrations: self
                .demonstrations
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
            artifacts: self
                .artifacts
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
            scopes: self
                .scopes
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
        };

        if let Ok(json_bytes) = serde_json::to_vec_pretty(&store) {
            let tmp_path = path.with_extension("tmp");
            if std::fs::write(&tmp_path, json_bytes).is_ok() {
                let _ = std::fs::rename(tmp_path, path);
            }
        }
    }

    /// Retrieve active learning candidates filtered by optional layer.
    pub fn get_candidates(
        &self,
        layer_filter: Option<LearningLayer>,
    ) -> LearningCandidatesProjection {
        let candidates = self
            .candidates
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let filtered: Vec<LearningCandidate> = candidates
            .iter()
            .filter(|c| layer_filter.is_none_or(|l| c.layer == l))
            .cloned()
            .collect();
        let total_count = filtered.len();

        LearningCandidatesProjection {
            schema_version: WEB_SCHEMA_V1,
            candidates: filtered,
            total_count,
        }
    }

    /// Propose a new learning candidate.
    pub fn propose_candidate(&self, req: ProposeLearningCandidateRequest) -> LearningCandidate {
        let now = OffsetDateTime::now_utc();
        let candidate = LearningCandidate {
            candidate_id: Uuid::new_v4(),
            layer: req.layer,
            source_evidence: req.source_evidence,
            outcome_evidence: req.outcome_evidence,
            generalization: req.generalization,
            scope: req.scope,
            derivation_version: 1,
            created_at: now,
        };

        {
            let mut candidates = self
                .candidates
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            candidates.push(candidate.clone());
        }
        self.persist();
        candidate
    }

    /// Evaluate a candidate against demonstrated outcomes and promotion criteria.
    pub fn evaluate_candidate(
        &self,
        candidate_id: Uuid,
        supplied_outcomes: Option<Vec<DemonstratedOutcome>>,
    ) -> Result<CandidateEvaluationProjection, String> {
        let candidates = self
            .candidates
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let candidate = candidates
            .iter()
            .find(|c| c.candidate_id == candidate_id)
            .cloned()
            .ok_or_else(|| format!("Candidate {candidate_id} not found"))?;
        drop(candidates);

        let outcomes = if let Some(outs) = supplied_outcomes {
            outs
        } else {
            let demos = self
                .demonstrations
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            demos
                .iter()
                .find(|(id, _)| *id == candidate_id)
                .map(|(_, list)| list.clone())
                .unwrap_or_default()
        };

        let gate = PromotionGate {
            min_independent_episodes: 2,
            min_success_rate: 0.80,
            evaluation_passed: true,
        };

        let result = match evaluate_promotion(&candidate, &outcomes, &gate) {
            Ok(promoted) => {
                let now = OffsetDateTime::now_utc();
                let artifact_id = Uuid::new_v4();
                let mut all_evidence = candidate.source_evidence.clone();
                all_evidence.extend(candidate.outcome_evidence.clone());

                let artifact = LearnedArtifactLineage {
                    artifact_id,
                    layer: candidate.layer,
                    status: ArtifactStatus::Promoted,
                    contributing_candidates: vec![candidate.candidate_id],
                    source_evidence: all_evidence,
                    promoted_at: Some(now),
                    erasure_epoch: 1,
                };

                {
                    let mut artifacts = self
                        .artifacts
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    artifacts.push(artifact.clone());
                }

                Ok(CandidateEvaluationProjection {
                    schema_version: WEB_SCHEMA_V1,
                    candidate_id,
                    promoted: Some(promoted),
                    refused: None,
                    artifact: Some(artifact),
                })
            }
            Err(refused) => Ok(CandidateEvaluationProjection {
                schema_version: WEB_SCHEMA_V1,
                candidate_id,
                promoted: None,
                refused: Some(refused),
                artifact: None,
            }),
        };

        self.persist();
        result
    }

    /// Retrieve list of promoted durable artifacts.
    pub fn get_artifacts(&self) -> LearnedArtifactsProjection {
        let artifacts = self
            .artifacts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let total_count = artifacts.len();

        LearnedArtifactsProjection {
            schema_version: WEB_SCHEMA_V1,
            artifacts: artifacts.clone(),
            total_count,
        }
    }

    /// Revoke or deprecate a promoted artifact.
    pub fn revoke_artifact(&self, req: RevokeArtifactRequest) -> bool {
        let mut artifacts = self
            .artifacts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let revoked = if let Some(art) = artifacts
            .iter_mut()
            .find(|a| a.artifact_id == req.artifact_id)
        {
            art.status = ArtifactStatus::Revoked;
            true
        } else {
            false
        };
        drop(artifacts);
        if revoked {
            self.persist();
        }
        revoked
    }

    /// Retrieve active task-scoped capability grants and governance scopes.
    pub fn get_scopes(&self) -> GovernanceScopesProjection {
        let scopes = self
            .scopes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        GovernanceScopesProjection {
            schema_version: WEB_SCHEMA_V1,
            scopes: scopes.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learning_hub_persists_and_reloads() {
        let tmp_dir = std::env::temp_dir().join(format!("cybou_test_learning_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&tmp_dir);
        let store_path = tmp_dir.join("learning.json");

        let hub = LearningHub::with_optional_store(Some(store_path.clone()));
        let candidate = hub.propose_candidate(ProposeLearningCandidateRequest {
            layer: LearningLayer::Behavioral,
            source_evidence: vec![],
            outcome_evidence: vec![],
            generalization: "Test generalization".to_owned(),
            scope: "Test scope".to_owned(),
        });

        assert_eq!(hub.get_candidates(None).candidates.len(), 1);

        // Reload
        let reloaded = LearningHub::with_optional_store(Some(store_path));
        let candidates = reloaded.get_candidates(None).candidates;
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].candidate_id, candidate.candidate_id);

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
