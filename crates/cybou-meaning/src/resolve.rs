// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Reference resolution that keeps its ambiguity instead of spending it.
//!
//! "that server", "it", "the previous one" are resolved against candidates the rest of Mind
//! already holds — active concepts, recent contributions, open intentions. What matters is not the
//! ranking, which is ordinary string work, but what happens when the ranking does not settle the
//! question: the reference stays unresolved and says which candidates it was torn between.

use cybou_protocol::meaning::{CognitiveActKind, ReferenceCandidate, ReferenceResolution};

use crate::is_mutating;

/// Something the reference could be pointing at.
///
/// Supplied by the caller, because this module does not get to decide what exists. ADR-0031 keeps
/// retrieval with ADR-0029 and delivery with ADR-0030; a resolver that went looking for its own
/// candidates would be reaching past both.
pub struct Candidate {
    /// Stable identity of the entity.
    pub target_id: String,
    /// How the entity is named where it lives.
    pub label: String,
}

/// How far ahead the best candidate must be before a reference counts as settled.
///
/// A margin rather than an absolute score: two candidates matching equally well is exactly the
/// case a person needs to be asked about, and no threshold on the leader alone can see it.
const DECIDING_MARGIN: f64 = 0.25;

/// Resolve one surface reference against the candidates a caller offers.
///
/// A mutating act never resolves on a margin alone. Asking about the wrong server wastes a
/// sentence; cancelling the wrong run cancels something, and "the highest score won" is not a
/// reason a person would accept for it afterwards.
#[must_use]
pub fn resolve_reference(
    surface_form: &str,
    candidates: &[Candidate],
    act: CognitiveActKind,
) -> ReferenceResolution {
    let needle = surface_form.trim().to_lowercase();
    let mut scored: Vec<ReferenceCandidate> = candidates
        .iter()
        .map(|candidate| ReferenceCandidate {
            target_id: candidate.target_id.clone(),
            label: candidate.label.clone(),
            score: overlap(&needle, &candidate.label.to_lowercase()),
        })
        .filter(|candidate| candidate.score > 0.0)
        .collect();
    scored.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            // Two candidates that score identically are ordered by identity so the same inputs
            // always produce the same list. A resolution that shuffled between runs could not be
            // compared with the one a person was shown.
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.target_id.cmp(&right.target_id))
    });

    let decided = match scored.as_slice() {
        [] => false,
        [_only] => true,
        [best, next, ..] => best.score - next.score >= DECIDING_MARGIN,
    };

    // A pronoun carries no descriptive content, so nothing it could be compared against would be
    // evidence. Resolving it would be choosing for the person rather than reading them.
    let describes_something = !needle.is_empty() && !is_pronoun(&needle);
    let resolved = decided && describes_something && !is_mutating(act);

    let selected_target_id = if resolved {
        scored.first().map(|candidate| candidate.target_id.clone())
    } else {
        None
    };

    ReferenceResolution {
        surface_form: surface_form.trim().to_owned(),
        candidates: scored,
        resolved,
        selected_target_id,
    }
}

/// Whether a surface form is a bare pronoun in either supported language.
fn is_pronoun(needle: &str) -> bool {
    matches!(
        needle,
        "it" | "that" | "this" | "the previous one" | "оно" | "это" | "то" | "он" | "она"
    )
}

/// How much of the reference the label accounts for, in [0.0, 1.0].
///
/// Word overlap, and nothing cleverer. It is stated plainly rather than tuned, because the number
/// it produces is only ever used to decide whether two candidates are too close to separate, and a
/// score nobody can explain would make that decision unexplainable too.
fn overlap(needle: &str, label: &str) -> f64 {
    let words: Vec<&str> = needle
        .split_whitespace()
        .filter(|word| !is_noise(word))
        .collect();
    if words.is_empty() {
        return 0.0;
    }
    let matched = words.iter().filter(|word| label.contains(**word)).count();
    #[allow(
        clippy::cast_precision_loss,
        reason = "a reference is a handful of words; neither count reaches the limits of f64"
    )]
    let score = matched as f64 / words.len() as f64;
    score
}

/// Words that appear in almost any reference and separate nothing.
fn is_noise(word: &str) -> bool {
    matches!(
        word,
        "the" | "a" | "an" | "that" | "this" | "тот" | "эта" | "этот"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidates() -> Vec<Candidate> {
        vec![
            Candidate {
                target_id: "srv-1".into(),
                label: "debian primary server".into(),
            },
            Candidate {
                target_id: "srv-2".into(),
                label: "debian staging server".into(),
            },
            Candidate {
                target_id: "pg-1".into(),
                label: "postgres".into(),
            },
        ]
    }

    #[test]
    fn a_reference_two_candidates_answer_equally_well_stays_unresolved() {
        // C2, the ordinary case: the ranking cannot separate them, so the question is still open.
        let resolved =
            resolve_reference("that debian server", &candidates(), CognitiveActKind::Ask);
        assert!(!resolved.resolved);
        assert!(resolved.selected_target_id.is_none());
        // The ambiguity is kept rather than thrown away, so an interface can show what it was torn
        // between and a person can settle it in one word.
        assert_eq!(resolved.candidates.len(), 2);
        assert!(
            resolved
                .candidates
                .iter()
                .any(|candidate| candidate.target_id == "srv-2")
        );
    }

    #[test]
    fn a_reference_only_one_candidate_answers_resolves() {
        let resolved = resolve_reference("postgres", &candidates(), CognitiveActKind::Ask);
        assert!(resolved.resolved);
        assert_eq!(resolved.selected_target_id.as_deref(), Some("pg-1"));
    }

    #[test]
    fn a_mutating_act_never_picks_a_referent_on_a_margin() {
        // C2, the case that matters: the same reference that resolves for a question does not
        // resolve for something that would change state.
        let asked = resolve_reference("postgres", &candidates(), CognitiveActKind::Ask);
        assert!(asked.resolved);

        let cancelled = resolve_reference("postgres", &candidates(), CognitiveActKind::Cancel);
        assert!(
            !cancelled.resolved,
            "a mutating act must be confirmed by the person, not decided by a score"
        );
        assert!(cancelled.selected_target_id.is_none());
        // It still reports what it found, so the interface can offer it for confirmation.
        assert_eq!(
            cancelled.candidates.first().map(|c| c.target_id.as_str()),
            Some("pg-1")
        );
    }

    #[test]
    fn a_bare_pronoun_resolves_to_nothing_however_few_candidates_there_are() {
        let single = vec![Candidate {
            target_id: "srv-1".into(),
            label: "debian primary server".into(),
        }];
        let resolved = resolve_reference("it", &single, CognitiveActKind::Ask);
        assert!(!resolved.resolved);
        assert!(resolved.candidates.is_empty());
    }

    #[test]
    fn a_reference_nothing_answers_resolves_to_nothing() {
        let resolved = resolve_reference("the kettle", &candidates(), CognitiveActKind::Ask);
        assert!(!resolved.resolved);
        assert!(resolved.candidates.is_empty());
        assert!(resolved.selected_target_id.is_none());
    }

    #[test]
    fn the_same_inputs_always_produce_the_same_ranking() {
        let first = resolve_reference("debian server", &candidates(), CognitiveActKind::Ask);
        let second = resolve_reference("debian server", &candidates(), CognitiveActKind::Ask);
        let ids = |resolution: &ReferenceResolution| {
            resolution
                .candidates
                .iter()
                .map(|candidate| candidate.target_id.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&first), ids(&second));
    }
}
