// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Learning candidates, owner-resolved evidence, promotion, and governance.

use cybou_protocol::governance::TaskScope;
use cybou_protocol::learning::{
    ArtifactStatus, LearnedArtifactLineage, LearningCandidate, LearningLayer, PromotionGate,
};
use cybou_protocol::promotion::{DemonstratedOutcome, evaluate_promotion};
use cybou_web_contracts::{
    ActionRecordProjection, CandidateEvaluationProjection, GovernanceScopesProjection,
    LearnedArtifactsProjection, LearningCandidatesProjection, ProposeLearningCandidateRequest,
    RevokeArtifactRequest, WEB_SCHEMA_V1,
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

/// A mutation either could not find its target, could not read its evidence, or could not be made
/// durable.
#[derive(Debug)]
pub enum LearningError {
    /// The target does not exist.
    NotFound,
    /// The owner of the evidence could not be read.
    ///
    /// Evaluation stops here rather than falling back on demonstrations resolved earlier: a
    /// promotion granted on a memory of evidence is a promotion nobody can check now.
    EvidenceUnavailable,
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
    ///
    /// # Errors
    ///
    /// Fails when the durable store rejects the change.
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

    /// Derive what a candidate has actually demonstrated from canonical Action1 records.
    ///
    /// An episode is one proposal, so two outcomes of the same proposal are one occasion rather
    /// than two. A record only demonstrates something when the effect was established and the
    /// executor's claim and the telemetry agree; a disagreement, or an effect nothing established,
    /// is evidence of nothing and is left out rather than counted as either a success or a failure.
    #[must_use]
    pub fn demonstrations_in(
        candidate: &LearningCandidate,
        records: &[ActionRecordProjection],
    ) -> Vec<DemonstratedOutcome> {
        let mut demonstrations: Vec<DemonstratedOutcome> = Vec::new();
        for record in records {
            if !candidate.scope_admits(&record.operation, &record.target_resource) {
                continue;
            }
            let Some(outcome) = record.outcome.as_ref() else {
                continue;
            };
            if outcome.relief == "not-established" || outcome.agreement == "not-comparable" {
                continue;
            }
            if demonstrations
                .iter()
                .any(|value| value.outcome == outcome.outcome_id)
            {
                continue;
            }
            demonstrations.push(DemonstratedOutcome {
                episode: record.proposal_id,
                outcome: outcome.outcome_id,
                succeeded: outcome.relief == "relieved" && outcome.agreement == "agree",
            });
        }
        demonstrations
    }

    /// Resolve owner-held evidence and apply the promotion gate.
    ///
    /// `records` is what Action1 currently establishes; `None` means Action1 could not be read, and
    /// evaluation refuses rather than promoting against whatever was resolved last time.
    ///
    /// # Errors
    ///
    /// Reports not found when no candidate carries that identity, stops when Action1 cannot be read,
    /// and fails when the durable store rejects the change.
    pub fn evaluate_candidate(
        &self,
        candidate_id: Uuid,
        records: Option<&[ActionRecordProjection]>,
    ) -> Result<CandidateEvaluationProjection, LearningError> {
        let records = records.ok_or(LearningError::EvidenceUnavailable)?;
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
        // Demonstrations are derived from the canonical record, not accumulated here, so evidence
        // that Action1 no longer establishes stops supporting a promotion.
        let outcomes = Self::demonstrations_in(&next.candidates[position], records);
        next.demonstrations.retain(|(id, _)| *id != candidate_id);
        next.demonstrations.push((candidate_id, outcomes.clone()));
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
    ///
    /// # Errors
    ///
    /// Reports not found when no artifact carries that identity, and fails when the durable store
    /// rejects the change — in which case nothing in memory changed either.
    pub fn revoke_artifact(&self, request: &RevokeArtifactRequest) -> Result<(), LearningError> {
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

    fn action(
        operation: &str,
        target: &str,
        relief: &str,
        agreement: &str,
    ) -> ActionRecordProjection {
        let proposal_id = Uuid::new_v4();
        ActionRecordProjection {
            proposal_id,
            decision_id: Uuid::new_v4(),
            cause_id: None,
            proposer: "mind".into(),
            intent: "relieve".into(),
            operation: operation.into(),
            target_resource: target.into(),
            risk_level: "low".into(),
            reversible: true,
            proposed_at: "2026-01-01T00:00:00Z".into(),
            checks: Vec::new(),
            verdict: "granted".into(),
            verdict_reason: None,
            execution_started: None,
            attempt: None,
            outcome: Some(cybou_web_contracts::ActionOutcomeProjection {
                outcome_id: Uuid::new_v4(),
                proposal_id,
                relief: relief.into(),
                agreement: agreement.into(),
                disagreement: None,
                observation_before: None,
                observation_after: None,
                concluded_at: "2026-01-01T00:01:00Z".into(),
            }),
        }
    }

    #[test]
    fn browser_cannot_claim_evidence_or_promote_without_owner_evidence() {
        let hub = LearningHub::with_optional_store(None);
        let candidate = hub.propose_candidate(request()).expect("proposal");
        assert!(candidate.source_evidence.is_empty() && candidate.outcome_evidence.is_empty());
        assert!(
            hub.evaluate_candidate(candidate.candidate_id, Some(&[]))
                .expect("evaluation")
                .promoted
                .is_none()
        );
    }

    #[test]
    fn an_unreadable_evidence_owner_stops_the_evaluation() {
        let hub = LearningHub::with_optional_store(None);
        let candidate = hub.propose_candidate(request()).expect("proposal");
        assert!(matches!(
            hub.evaluate_candidate(candidate.candidate_id, None),
            Err(LearningError::EvidenceUnavailable)
        ));
    }

    #[test]
    fn demonstrations_come_from_canonical_action_records_in_scope() {
        let hub = LearningHub::with_optional_store(None);
        let candidate = hub
            .propose_candidate(ProposeLearningCandidateRequest {
                layer: LearningLayer::Procedural,
                generalization: "restarting relieves it".into(),
                scope: "service.restart".into(),
            })
            .expect("proposal");
        let records = vec![
            action("service.restart", "systemd:a.service", "relieved", "agree"),
            action("service.restart", "systemd:b.service", "relieved", "agree"),
            // Out of scope, so it is not this candidate's evidence.
            action("package.install", "apt:tree", "relieved", "agree"),
            // Nothing was established, so it demonstrates neither success nor failure.
            action(
                "service.restart",
                "systemd:c.service",
                "not-established",
                "agree",
            ),
            // The executor and the telemetry tell different stories.
            action(
                "service.restart",
                "systemd:d.service",
                "relieved",
                "not-comparable",
            ),
        ];

        let evaluation = hub
            .evaluate_candidate(candidate.candidate_id, Some(&records))
            .expect("evaluation");
        let promoted = evaluation.promoted.expect("promotion");
        assert_eq!(promoted.independent_episodes, 2);
        let artifact = evaluation.artifact.expect("artifact");
        assert!(artifact.source_evidence.contains(&records[0].proposal_id));
        assert!(artifact.source_evidence.contains(&records[1].proposal_id));
        assert!(!artifact.source_evidence.contains(&records[2].proposal_id));
        assert!(!artifact.source_evidence.contains(&records[3].proposal_id));
    }

    #[test]
    fn evidence_action1_no_longer_establishes_stops_supporting_a_promotion() {
        let hub = LearningHub::with_optional_store(None);
        let candidate = hub
            .propose_candidate(ProposeLearningCandidateRequest {
                layer: LearningLayer::Procedural,
                generalization: "restarting relieves it".into(),
                scope: "service.restart".into(),
            })
            .expect("proposal");
        let records = vec![
            action("service.restart", "systemd:a.service", "relieved", "agree"),
            action("service.restart", "systemd:b.service", "relieved", "agree"),
        ];
        assert!(
            hub.evaluate_candidate(candidate.candidate_id, Some(&records))
                .expect("evaluation")
                .promoted
                .is_some()
        );

        // Re-evaluated against a record set that no longer contains those episodes, the candidate
        // is refused again rather than living on the demonstrations resolved the first time.
        let refused = hub
            .evaluate_candidate(candidate.candidate_id, Some(&[]))
            .expect("evaluation");
        assert!(refused.promoted.is_none());
        assert!(refused.refused.is_some());
        let stored = hub
            .get_candidates(None)
            .candidates
            .into_iter()
            .find(|value| value.candidate_id == candidate.candidate_id)
            .expect("candidate");
        assert!(stored.source_evidence.is_empty());
        assert!(stored.outcome_evidence.is_empty());
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
