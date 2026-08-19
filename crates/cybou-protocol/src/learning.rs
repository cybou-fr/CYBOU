// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Layered Lifelong Learning and Learned-Artifact Governance (ADR-0032 & ADR-0033).
//!
//! Provides inspectable candidate extraction, multi-layered skill induction,
//! promotion evaluation, and erasure lineage tracking without conflating learning with authority.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Learning layer hierarchy defined in ADR-0032.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LearningLayer {
    /// L0: Accepted biography and episodic history.
    Episodic,
    /// L1: Validated epistemic state and propositions.
    Epistemic,
    /// L2: Associative context graph and co-occurrence.
    Associative,
    /// L3: Behavioral habits, reference tendencies, and linguistic preferences.
    Behavioral,
    /// L4: Reusable verified procedures and skills.
    Procedural,
    /// L5: Optional statistical or neural parameter adaptation.
    Neural,
}

/// Lifecycle status of a learned cognitive artifact per ADR-0033.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactStatus {
    /// Initial candidate proposed from consolidation or observation.
    Draft,
    /// Undergoing automated evaluation across historical episodes.
    Evaluating,
    /// Successfully evaluated and promoted into active use.
    Promoted,
    /// Superseded by a newer artifact version.
    Deprecated,
    /// Explicitly revoked due to contradiction, user veto, or erasure.
    Revoked,
}

/// A candidate learning proposition derived from episodic evidence.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearningCandidate {
    /// Unique candidate identifier.
    pub candidate_id: Uuid,
    /// Target learning layer.
    pub layer: LearningLayer,
    /// Source evidence message IDs from which the pattern was extracted.
    pub source_evidence: Vec<Uuid>,
    /// Observed outcome message IDs demonstrating success or failure.
    pub outcome_evidence: Vec<Uuid>,
    /// Proposed behavioral or procedural generalization.
    pub generalization: String,
    /// Applicability scope (e.g. "service.postgresql", "dialogue.concise").
    pub scope: String,
    /// Learning extraction algorithm version.
    pub derivation_version: u32,
    /// Candidate creation timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Governance criteria required to promote a learning candidate into active use.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromotionGate {
    /// Minimum distinct episodes required to demonstrate repeatability.
    pub min_independent_episodes: u32,
    /// Minimum empirical success rate across episodes in [0.0, 1.0].
    pub min_success_rate: f64,
    /// Whether explicit evaluation against replay tests succeeded.
    pub evaluation_passed: bool,
}

impl Default for PromotionGate {
    fn default() -> Self {
        Self {
            min_independent_episodes: 3,
            min_success_rate: 0.85,
            evaluation_passed: false,
        }
    }
}

/// Provenance and lineage record for a durable learned artifact.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LearnedArtifactLineage {
    /// Unique artifact identifier.
    pub artifact_id: Uuid,
    /// Learning layer.
    pub layer: LearningLayer,
    /// Current lifecycle status.
    pub status: ArtifactStatus,
    /// Contributing candidate IDs that produced this artifact.
    pub contributing_candidates: Vec<Uuid>,
    /// Underlying evidence IDs across all contributing candidates.
    pub source_evidence: Vec<Uuid>,
    /// Instant when the artifact was promoted.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub promoted_at: Option<OffsetDateTime>,
    /// Erasure epoch under which this artifact was validated.
    pub erasure_epoch: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learning_candidate_creation_and_layer_hierarchy() {
        let ev1 = Uuid::new_v4();
        let ev2 = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();

        let candidate = LearningCandidate {
            candidate_id: Uuid::new_v4(),
            layer: LearningLayer::Procedural,
            source_evidence: vec![ev1],
            outcome_evidence: vec![ev2],
            generalization: "restart nginx when socket returns ECONNREFUSED".into(),
            scope: "service.nginx".into(),
            derivation_version: 1,
            created_at: now,
        };

        assert_eq!(candidate.layer, LearningLayer::Procedural);
        assert_eq!(candidate.source_evidence.len(), 1);
    }
}
