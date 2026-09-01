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

/// Something a plan carries that the prose must not be allowed to drop.
///
/// A closed set, never free text. These are the qualifications ADR-0031 C5 requires a plan to
/// express *before* language realization, and they are exactly the parts a fluent sentence tends to
/// lose: "mostly fine", "as far as I know", "roughly". A renderer that omitted one would turn a
/// hedged answer into a confident one, which is the failure the plan boundary exists to prevent.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Qualification {
    /// An owner this answer depends on was never read.
    ///
    /// Not the same as an owner that answered with nothing. Silence and emptiness are different
    /// facts, and only one of them is evidence.
    NotRead,
    /// The state this answer rests on is outside the freshness its owner declared.
    Stale,
    /// The answer was cut short by a bound rather than by running out of things to say.
    Partial,
    /// Something was withheld from the reader this answer is for.
    Withheld,
    /// The Journal behind this answer has not been verified through its head.
    Unverified,
    /// Competing observations disagree about something here, and nothing has settled which is right.
    ///
    /// Added rather than folded into `Unverified`, which means something else: unverified is a
    /// check that has not finished, disputed is a check that finished and came back contradictory.
    /// A reader told the weaker of the two would not know to go and look.
    Disputed,
    /// Something newer has replaced part of what this rests on.
    Superseded,
}

impl Qualification {
    /// The frozen spelling this qualification is recorded and rendered under.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NotRead => "not-read",
            Self::Stale => "stale",
            Self::Partial => "partial",
            Self::Withheld => "withheld",
            Self::Unverified => "unverified",
            Self::Disputed => "disputed",
            Self::Superseded => "superseded",
        }
    }
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
    /// What the reader must be told alongside the points, whatever wording is chosen.
    ///
    /// Carried in the plan rather than left to the realizer because a qualification is a claim
    /// about the answer's standing, and the realizer is not allowed to make claims (C5, C6).
    #[serde(default)]
    pub qualifications: Vec<Qualification>,
}

/// Complete answer owned by Meaning1 after an utterance was admitted to Event1.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeaningResponse {
    /// Typed interpretation recorded by the owner.
    pub interpretation: MeaningInterpretation,
    /// Owner-built response plan.
    pub response_plan: ResponsePlan,
    /// Deterministic realization of that plan.
    pub realization: String,
}

/// Bounded dialogue state held by Meaning1, never by a transport boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DialogueMemory {
    /// Current dialogue turn.
    pub current_turn: u64,
    /// Referents still inside the owner's retention window.
    pub remembered_referents: Vec<String>,
    /// Maximum turns retained by this owner.
    pub turns_bound: u64,
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
