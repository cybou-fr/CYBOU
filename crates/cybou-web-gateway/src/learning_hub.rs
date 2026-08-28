// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Learning & Governance Hub: Manages layered lifelong learning candidates,
//! deterministic promotion gates, durable artifact lineages, and task-scoped capability governance.

use std::sync::Mutex;
use cybou_protocol::learning::{
    ArtifactStatus, LearnedArtifactLineage, LearningCandidate, LearningLayer, PromotionGate,
};
use cybou_protocol::promotion::{
    DemonstratedOutcome, evaluate_promotion,
};
use cybou_protocol::governance::{
    ActorKind, TaskScope,
};
use cybou_web_contracts::{
    CandidateEvaluationProjection, GovernanceScopesProjection, LearnedArtifactsProjection,
    LearningCandidatesProjection, ProposeLearningCandidateRequest, RevokeArtifactRequest,
    WEB_SCHEMA_V1,
};
use time::OffsetDateTime;
use uuid::Uuid;

/// Server-owned engine for candidate evaluation, artifact lineage, and capability grants.
pub struct LearningHub {
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
    /// Create a new LearningHub initialized with nominal system candidates, artifacts, and scopes.
    #[must_use]
    pub fn new() -> Self {
        let now = OffsetDateTime::now_utc();
        let ev1 = Uuid::new_v4();
        let ev2 = Uuid::new_v4();
        let ev3 = Uuid::new_v4();
        let ep1 = Uuid::new_v4();
        let ep2 = Uuid::new_v4();
        let ep3 = Uuid::new_v4();

        let cand_procedural_id = Uuid::new_v4();
        let cand_behavioral_id = Uuid::new_v4();
        let cand_epistemic_id = Uuid::new_v4();
        let cand_neural_id = Uuid::new_v4();

        let candidates = vec![
            LearningCandidate {
                candidate_id: cand_procedural_id,
                layer: LearningLayer::Procedural,
                source_evidence: vec![ev1, ev2],
                outcome_evidence: vec![ev3],
                generalization: "Auto-reconnect D-Bus daemon when connection drop is detected with ECONNREFUSED".into(),
                scope: "service.dbus".into(),
                derivation_version: 1,
                created_at: now,
            },
            LearningCandidate {
                candidate_id: cand_behavioral_id,
                layer: LearningLayer::Behavioral,
                source_evidence: vec![ev1],
                outcome_evidence: vec![ev2],
                generalization: "Format telemetry as concise spatial badges when viewing Living Canvas overview".into(),
                scope: "canvas.telemetry".into(),
                derivation_version: 1,
                created_at: now,
            },
            LearningCandidate {
                candidate_id: cand_epistemic_id,
                layer: LearningLayer::Epistemic,
                source_evidence: vec![ev2],
                outcome_evidence: vec![ev3],
                generalization: "Classify Landlock confinement as strictly enforced when ABI version >= 3".into(),
                scope: "security.landlock".into(),
                derivation_version: 1,
                created_at: now,
            },
            LearningCandidate {
                candidate_id: cand_neural_id,
                layer: LearningLayer::Neural,
                source_evidence: vec![ev1, ev3],
                outcome_evidence: vec![ev2],
                generalization: "Low-rank LoRA parameter adaptation for fast local Rust AST lint reasoning".into(),
                scope: "agent.opencode".into(),
                derivation_version: 1,
                created_at: now,
            },
        ];

        let demonstrations = vec![
            (
                cand_procedural_id,
                vec![
                    DemonstratedOutcome {
                        episode: ep1,
                        outcome: ev1,
                        succeeded: true,
                    },
                    DemonstratedOutcome {
                        episode: ep2,
                        outcome: ev2,
                        succeeded: true,
                    },
                    DemonstratedOutcome {
                        episode: ep3,
                        outcome: ev3,
                        succeeded: true,
                    },
                ],
            ),
            (
                cand_behavioral_id,
                vec![
                    DemonstratedOutcome {
                        episode: ep1,
                        outcome: ev1,
                        succeeded: true,
                    },
                    DemonstratedOutcome {
                        episode: ep2,
                        outcome: ev2,
                        succeeded: true,
                    },
                ],
            ),
        ];

        let art_id = Uuid::new_v4();
        let artifacts = vec![LearnedArtifactLineage {
            artifact_id: art_id,
            layer: LearningLayer::Procedural,
            status: ArtifactStatus::Promoted,
            contributing_candidates: vec![cand_procedural_id],
            source_evidence: vec![ev1, ev2, ev3],
            promoted_at: Some(now),
            erasure_epoch: 1,
        }];

        let scopes = vec![
            TaskScope {
                actor_id: Uuid::new_v4(),
                kind: ActorKind::Agent,
                intention_id: Some(Uuid::new_v4()),
                capabilities: vec![
                    "fs.read".into(),
                    "fs.write".into(),
                    "terminal.exec".into(),
                    "model.query".into(),
                ],
                tool_grants: vec![
                    "git.status".into(),
                    "git.diff".into(),
                    "cargo.check".into(),
                ],
                network_destinations: vec!["localhost".into(), "127.0.0.1".into()],
                ttl_seconds: 7200,
                max_compute_ms: 300_000,
                delegation_permitted: true,
                granted_at: now,
            },
            TaskScope {
                actor_id: Uuid::new_v4(),
                kind: ActorKind::Worker,
                intention_id: Some(Uuid::new_v4()),
                capabilities: vec!["fs.read".into()],
                tool_grants: vec!["fs.read_file".into()],
                network_destinations: Vec::new(),
                ttl_seconds: 600,
                max_compute_ms: 10_000,
                delegation_permitted: false,
                granted_at: now,
            },
        ];

        Self {
            candidates: Mutex::new(candidates),
            demonstrations: Mutex::new(demonstrations),
            artifacts: Mutex::new(artifacts),
            scopes: Mutex::new(scopes),
        }
    }

    /// Retrieve active learning candidates filtered by optional layer.
    pub fn get_candidates(&self, layer_filter: Option<LearningLayer>) -> LearningCandidatesProjection {
        let candidates = self.candidates.lock().unwrap_or_else(|e| e.into_inner());
        let filtered: Vec<LearningCandidate> = candidates
            .iter()
            .filter(|c| layer_filter.map_or(true, |l| c.layer == l))
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

        let mut candidates = self.candidates.lock().unwrap_or_else(|e| e.into_inner());
        candidates.push(candidate.clone());
        candidate
    }

    /// Evaluate a candidate against demonstrated outcomes and promotion criteria.
    pub fn evaluate_candidate(
        &self,
        candidate_id: Uuid,
        supplied_outcomes: Option<Vec<DemonstratedOutcome>>,
    ) -> Result<CandidateEvaluationProjection, String> {
        let candidates = self.candidates.lock().unwrap_or_else(|e| e.into_inner());
        let candidate = candidates
            .iter()
            .find(|c| c.candidate_id == candidate_id)
            .cloned()
            .ok_or_else(|| format!("Candidate {candidate_id} not found"))?;
        drop(candidates);

        let outcomes = if let Some(outs) = supplied_outcomes {
            outs
        } else {
            let demos = self.demonstrations.lock().unwrap_or_else(|e| e.into_inner());
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

        match evaluate_promotion(&candidate, &outcomes, &gate) {
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

                let mut artifacts = self.artifacts.lock().unwrap_or_else(|e| e.into_inner());
                artifacts.push(artifact.clone());

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
        }
    }

    /// Retrieve list of promoted durable artifacts.
    pub fn get_artifacts(&self) -> LearnedArtifactsProjection {
        let artifacts = self.artifacts.lock().unwrap_or_else(|e| e.into_inner());
        let total_count = artifacts.len();

        LearnedArtifactsProjection {
            schema_version: WEB_SCHEMA_V1,
            artifacts: artifacts.clone(),
            total_count,
        }
    }

    /// Revoke or deprecate a promoted artifact.
    pub fn revoke_artifact(&self, req: RevokeArtifactRequest) -> bool {
        let mut artifacts = self.artifacts.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(art) = artifacts.iter_mut().find(|a| a.artifact_id == req.artifact_id) {
            art.status = ArtifactStatus::Revoked;
            true
        } else {
            false
        }
    }

    /// Retrieve active task-scoped capability grants and governance scopes.
    pub fn get_scopes(&self) -> GovernanceScopesProjection {
        let scopes = self.scopes.lock().unwrap_or_else(|e| e.into_inner());
        GovernanceScopesProjection {
            schema_version: WEB_SCHEMA_V1,
            scopes: scopes.clone(),
        }
    }
}
