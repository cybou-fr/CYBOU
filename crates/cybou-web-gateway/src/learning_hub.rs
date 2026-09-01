// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Learning candidates, owner-resolved evidence, promotion, and governance.

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
use std::{io::Write, path::PathBuf, sync::Mutex};
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

/// A mutation either could not find its target or could not be made durable.
#[derive(Debug)]
pub enum LearningError {
    /// The target does not exist.
    NotFound,
    /// The backing store rejected the state.
    Persistence(std::io::Error),
}

/// Server-owned learning state and evidence resolver.
pub struct LearningHub {
    store_path: Option<PathBuf>,
    store: Mutex<LearningStore>,
}

impl Default for LearningHub {
    fn default() -> Self {
        Self::new()
    }
}

impl LearningHub {
    /// Create a hub with the default store.
    #[must_use]
    pub fn new() -> Self {
        Self::with_optional_store(Self::default_store_path())
    }

    /// Determine the configured durable store.
    #[must_use]
    pub fn default_store_path() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("CYBOU_LEARNING_STORE") {
            return Some(path.into());
        }
        #[cfg(target_os = "linux")]
        {
            let path = PathBuf::from("/var/lib/cybou/learning-store.json");
            if path.parent().is_some_and(std::path::Path::exists) {
                return Some(path);
            }
        }
        None
    }

    /// Construct a hub with an optional store.
    #[must_use]
    pub fn with_optional_store(store_path: Option<PathBuf>) -> Self {
        let store = store_path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self {
            store_path,
            store: Mutex::new(store),
        }
    }

    fn persist(&self, store: &LearningStore) -> Result<(), LearningError> {
        let Some(path) = self.store_path.as_ref() else {
            return Ok(());
        };
        let bytes = serde_json::to_vec_pretty(store)
            .map_err(|e| LearningError::Persistence(std::io::Error::other(e)))?;
        let temporary = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
        let result = (|| {
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            std::fs::rename(&temporary, path)?;
            Ok::<(), std::io::Error>(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result.map_err(LearningError::Persistence)
    }

    /// Retrieve candidates, optionally filtered by layer.
    pub fn get_candidates(&self, layer: Option<LearningLayer>) -> LearningCandidatesProjection {
        let store = self
            .store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let candidates: Vec<_> = store
            .candidates
            .iter()
            .filter(|c| layer.is_none_or(|l| c.layer == l))
            .cloned()
            .collect();
        LearningCandidatesProjection {
            schema_version: WEB_SCHEMA_V1,
            total_count: candidates.len(),
            candidates,
        }
    }

    /// Propose candidate content; evidence is owner-resolved during evaluation.
    pub fn propose_candidate(
        &self,
        request: ProposeLearningCandidateRequest,
    ) -> Result<LearningCandidate, LearningError> {
        let candidate = LearningCandidate {
            candidate_id: Uuid::new_v4(),
            layer: request.layer,
            source_evidence: Vec::new(),
            outcome_evidence: Vec::new(),
            generalization: request.generalization,
            scope: request.scope,
            derivation_version: 1,
            created_at: OffsetDateTime::now_utc(),
        };
        let mut current = self
            .store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut next = current.clone();
        next.candidates.push(candidate.clone());
        self.persist(&next)?;
        *current = next;
        Ok(candidate)
    }

    /// Resolve owner-held evidence and apply the promotion gate.
    pub fn evaluate_candidate(
        &self,
        candidate_id: Uuid,
    ) -> Result<CandidateEvaluationProjection, LearningError> {
        let mut current = self
            .store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut next = current.clone();
        let position = next
            .candidates
            .iter()
            .position(|c| c.candidate_id == candidate_id)
            .ok_or(LearningError::NotFound)?;
        let outcomes = next
            .demonstrations
            .iter()
            .find(|(id, _)| *id == candidate_id)
            .map(|(_, values)| values.clone())
            .unwrap_or_default();
        let candidate = &mut next.candidates[position];
        candidate.source_evidence = outcomes.iter().map(|v| v.episode).collect();
        candidate.source_evidence.sort_unstable();
        candidate.source_evidence.dedup();
        candidate.outcome_evidence = outcomes.iter().map(|v| v.outcome).collect();
        candidate.outcome_evidence.sort_unstable();
        candidate.outcome_evidence.dedup();
        let candidate = candidate.clone();
        let gate = PromotionGate {
            min_independent_episodes: 2,
            min_success_rate: 0.80,
            evaluation_passed: true,
        };
        let projection = match evaluate_promotion(&candidate, &outcomes, &gate) {
            Ok(promoted) => {
                let mut evidence = candidate.source_evidence.clone();
                evidence.extend(candidate.outcome_evidence.iter().copied());
                evidence.sort_unstable();
                evidence.dedup();
                let artifact = LearnedArtifactLineage {
                    artifact_id: Uuid::new_v4(),
                    layer: candidate.layer,
                    status: ArtifactStatus::Promoted,
                    contributing_candidates: vec![candidate_id],
                    source_evidence: evidence,
                    promoted_at: Some(OffsetDateTime::now_utc()),
                    erasure_epoch: 1,
                };
                next.artifacts.push(artifact.clone());
                CandidateEvaluationProjection {
                    schema_version: WEB_SCHEMA_V1,
                    candidate_id,
                    promoted: Some(promoted),
                    refused: None,
                    artifact: Some(artifact),
                }
            }
            Err(refused) => CandidateEvaluationProjection {
                schema_version: WEB_SCHEMA_V1,
                candidate_id,
                promoted: None,
                refused: Some(refused),
                artifact: None,
            },
        };
        self.persist(&next)?;
        *current = next;
        Ok(projection)
    }

    /// Retrieve durable artifacts.
    pub fn get_artifacts(&self) -> LearnedArtifactsProjection {
        let store = self
            .store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        LearnedArtifactsProjection {
            schema_version: WEB_SCHEMA_V1,
            total_count: store.artifacts.len(),
            artifacts: store.artifacts.clone(),
        }
    }

    /// Revoke an artifact, committing disk before visible memory.
    pub fn revoke_artifact(&self, request: RevokeArtifactRequest) -> Result<(), LearningError> {
        let mut current = self
            .store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut next = current.clone();
        next.artifacts
            .iter_mut()
            .find(|a| a.artifact_id == request.artifact_id)
            .ok_or(LearningError::NotFound)?
            .status = ArtifactStatus::Revoked;
        self.persist(&next)?;
        *current = next;
        Ok(())
    }

    /// Retrieve task-scoped grants.
    pub fn get_scopes(&self) -> GovernanceScopesProjection {
        let store = self
            .store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        GovernanceScopesProjection {
            schema_version: WEB_SCHEMA_V1,
            scopes: store.scopes.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> ProposeLearningCandidateRequest {
        ProposeLearningCandidateRequest {
            layer: LearningLayer::Behavioral,
            generalization: "Test".into(),
            scope: "scope".into(),
        }
    }

    #[test]
    fn persists_and_reloads() {
        let directory = std::env::temp_dir().join(format!("cybou_learning_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("temporary directory");
        let path = directory.join("learning.json");
        let candidate = LearningHub::with_optional_store(Some(path.clone()))
            .propose_candidate(request())
            .expect("durable proposal");
        assert_eq!(
            LearningHub::with_optional_store(Some(path))
                .get_candidates(None)
                .candidates[0]
                .candidate_id,
            candidate.candidate_id
        );
        std::fs::remove_dir_all(directory).expect("remove temporary directory");
    }

    #[test]
    fn browser_cannot_claim_evidence_or_promote_without_owner_evidence() {
        let hub = LearningHub::with_optional_store(None);
        let candidate = hub.propose_candidate(request()).expect("proposal");
        assert!(candidate.source_evidence.is_empty() && candidate.outcome_evidence.is_empty());
        assert!(
            hub.evaluate_candidate(candidate.candidate_id)
                .expect("evaluation")
                .promoted
                .is_none()
        );
    }

    #[test]
    fn promoted_lineage_is_resolved_from_owner_demonstrations() {
        let hub = LearningHub::with_optional_store(None);
        let candidate = hub.propose_candidate(request()).expect("proposal");
        let episode_a = Uuid::new_v4();
        let episode_b = Uuid::new_v4();
        let outcome_a = Uuid::new_v4();
        let outcome_b = Uuid::new_v4();
        hub.store
            .lock()
            .expect("learning store")
            .demonstrations
            .push((
                candidate.candidate_id,
                vec![
                    DemonstratedOutcome {
                        episode: episode_a,
                        outcome: outcome_a,
                        succeeded: true,
                    },
                    DemonstratedOutcome {
                        episode: episode_b,
                        outcome: outcome_b,
                        succeeded: true,
                    },
                ],
            ));

        let evaluation = hub
            .evaluate_candidate(candidate.candidate_id)
            .expect("evaluation");
        let artifact = evaluation.artifact.expect("promoted artifact");
        assert!(artifact.source_evidence.contains(&episode_a));
        assert!(artifact.source_evidence.contains(&episode_b));
        assert!(artifact.source_evidence.contains(&outcome_a));
        assert!(artifact.source_evidence.contains(&outcome_b));
    }

    #[test]
    fn failed_persistence_does_not_publish_memory_mutation() {
        let directory = std::env::temp_dir().join(format!("cybou_learning_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("temporary directory");
        let hub = LearningHub::with_optional_store(Some(directory.clone()));
        assert!(matches!(
            hub.propose_candidate(request()),
            Err(LearningError::Persistence(_))
        ));
        assert!(hub.get_candidates(None).candidates.is_empty());
        std::fs::remove_dir_all(directory).expect("remove temporary directory");
    }
}
