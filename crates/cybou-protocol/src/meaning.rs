// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Structured Meaning and Cognitive Acts (ADR-0031).
//!
//! Separates raw language utterances from typed cognitive acts, reference resolution,
//! and response planning without allowing a generative model to act as cognitive authority.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Typed speech/cognitive act families per ADR-0031.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CognitiveActKind {
    /// Inquire about system or world state.
    Ask,
    /// Assert or report a factual claim.
    Inform,
    /// Solicit an operation or action from the system.
    Request,
    /// Correct a prior statement or interpretation.
    Correct,
    /// Explicitly confirm an action or interpretation.
    Confirm,
    /// Explicitly reject an action or proposal.
    Reject,
    /// Request inspection of an entity, process, or history.
    Inspect,
    /// Request an explanation of reasoning or causal link.
    Explain,
    /// Compare two or more entities or states.
    Compare,
    /// Verify a condition or invariant against reality.
    Verify,
    /// Resume a paused or suspended activity.
    Resume,
    /// Temporarily pause an activity.
    Pause,
    /// Cancel an active task or intention.
    Cancel,
    /// Explicitly store an episodic or declarative memory.
    Remember,
    /// Explicitly request erasure or forgetting.
    Forget,
    /// Propose a plan or hypothesis without executing.
    Propose,
}

/// A candidate entity considered during reference resolution.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceCandidate {
    /// Target entity or object identifier.
    pub target_id: String,
    /// Descriptive label.
    pub label: String,
    /// Resolution confidence score in [0.0, 1.0].
    pub score: f64,
}

/// Result of resolving a natural language reference ("it", "the server", "yesterday").
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceResolution {
    /// The surface reference string.
    pub surface_form: String,
    /// Candidates ranked by contextual relevance.
    pub candidates: Vec<ReferenceCandidate>,
    /// Whether the reference is unambiguously resolved above the certainty threshold.
    pub resolved: bool,
    /// The selected referent ID if resolved.
    pub selected_target_id: Option<String>,
}

/// A structured cognitive act crossing the meaning boundary.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveAct {
    /// Unique act identifier.
    pub act_id: Uuid,
    /// Act family kind.
    pub kind: CognitiveActKind,
    /// Primary subject or topic.
    pub subject: String,
    /// Extracted parameters and named attributes.
    pub parameters: Vec<(String, String)>,
    /// Originating source ("person", "organ.healthd", etc.).
    pub source: String,
    /// Evidence IDs supporting this act.
    pub evidence: Vec<Uuid>,
}

/// A full interpretation of an observed utterance.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeaningInterpretation {
    /// The original observed utterance.
    pub utterance: String,
    /// Interpreted primary cognitive act.
    pub primary_act: CognitiveAct,
    /// Resolved references within the utterance.
    pub references: Vec<ReferenceResolution>,
    /// Overall interpretation confidence in [0.0, 1.0].
    pub confidence: f64,
    /// Whether multiple conflicting interpretations exist (requires clarification).
    pub ambiguous: bool,
    /// Timestamp when interpretation was produced.
    #[serde(with = "time::serde::rfc3339")]
    pub derived_at: OffsetDateTime,
}

/// Abstract response plan formulated by Mind before language realization.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsePlan {
    /// Plan identifier.
    pub plan_id: Uuid,
    /// Communicative intent ("`inform_status`", "`clarify_ambiguity`", "`confirm_action`").
    pub intent: String,
    /// Key information points to convey.
    pub key_points: Vec<String>,
    /// Factual epistemic propositions referenced.
    pub referenced_evidence: Vec<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_resolution_ambiguity_flag() {
        let res = ReferenceResolution {
            surface_form: "that server".into(),
            candidates: vec![
                ReferenceCandidate {
                    target_id: "srv-1".into(),
                    label: "Debian primary".into(),
                    score: 0.48,
                },
                ReferenceCandidate {
                    target_id: "srv-2".into(),
                    label: "Debian staging".into(),
                    score: 0.45,
                },
            ],
            resolved: false,
            selected_target_id: None,
        };

        assert!(!res.resolved);
        assert!(res.selected_target_id.is_none());
        assert_eq!(res.candidates.len(), 2);
    }
}
