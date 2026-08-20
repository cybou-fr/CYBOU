// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The meaning boundary of ADR-0031: language on one side, typed cognitive acts on the other.
//!
//! Nothing here is a language model, and nothing here needs one. Every function is deterministic
//! and total: an utterance it cannot interpret produces no interpretation rather than a guess, and
//! a reference it cannot settle stays unresolved rather than resolving to whichever candidate
//! happened to score highest. Those two refusals are the boundary. Without them the layer would be
//! a way for prose to reach the APIs of Mind with its ambiguity silently discarded on the way.
//!
//! What crosses into Mind is a [`CognitiveAct`]: typed, inspectable, and readable long after
//! whatever produced it has been stopped or replaced.

pub mod realize;
pub mod resolve;

use cybou_protocol::meaning::{CognitiveAct, CognitiveActKind, MeaningInterpretation};
use time::OffsetDateTime;
use uuid::Uuid;

pub use realize::{Language, realize};
pub use resolve::{Candidate, resolve_reference};

/// A surface marker that introduces an act, in one language.
struct Marker {
    /// What the person said to open the utterance.
    opening: &'static str,
    /// The act that opening names.
    kind: CognitiveActKind,
}

/// The vocabulary this build recognises.
///
/// Deliberately small, and deliberately explicit. ADR-0031 asks for enough to support ordinary
/// interaction without pretending to encode every speech act, and an entry here is a claim that
/// this exact opening means this exact act: a claim a person can read and disagree with, which is
/// not true of a weight in a model.
///
/// Longer openings come first, because a shorter one that is a prefix of a longer one would
/// otherwise win and name a different act than the person did.
const MARKERS: &[Marker] = &[
    // English.
    Marker {
        opening: "tell me about",
        kind: CognitiveActKind::Ask,
    },
    Marker {
        opening: "what is",
        kind: CognitiveActKind::Ask,
    },
    Marker {
        opening: "what are",
        kind: CognitiveActKind::Ask,
    },
    Marker {
        opening: "how is",
        kind: CognitiveActKind::Ask,
    },
    Marker {
        opening: "why did",
        kind: CognitiveActKind::Explain,
    },
    Marker {
        opening: "why is",
        kind: CognitiveActKind::Explain,
    },
    Marker {
        opening: "explain",
        kind: CognitiveActKind::Explain,
    },
    Marker {
        opening: "show me",
        kind: CognitiveActKind::Inspect,
    },
    Marker {
        opening: "inspect",
        kind: CognitiveActKind::Inspect,
    },
    Marker {
        opening: "compare",
        kind: CognitiveActKind::Compare,
    },
    Marker {
        opening: "verify",
        kind: CognitiveActKind::Verify,
    },
    Marker {
        opening: "check",
        kind: CognitiveActKind::Verify,
    },
    Marker {
        opening: "please",
        kind: CognitiveActKind::Request,
    },
    Marker {
        opening: "remember that",
        kind: CognitiveActKind::Remember,
    },
    Marker {
        opening: "remember",
        kind: CognitiveActKind::Remember,
    },
    Marker {
        opening: "forget",
        kind: CognitiveActKind::Forget,
    },
    Marker {
        opening: "cancel",
        kind: CognitiveActKind::Cancel,
    },
    Marker {
        opening: "pause",
        kind: CognitiveActKind::Pause,
    },
    Marker {
        opening: "resume",
        kind: CognitiveActKind::Resume,
    },
    Marker {
        opening: "propose",
        kind: CognitiveActKind::Propose,
    },
    Marker {
        opening: "actually,",
        kind: CognitiveActKind::Correct,
    },
    Marker {
        opening: "no,",
        kind: CognitiveActKind::Correct,
    },
    Marker {
        opening: "yes",
        kind: CognitiveActKind::Confirm,
    },
    Marker {
        opening: "confirm",
        kind: CognitiveActKind::Confirm,
    },
    Marker {
        opening: "no",
        kind: CognitiveActKind::Reject,
    },
    // Russian. The same canonical acts reached from a different surface language: ADR-0031 asks
    // for multilingual interaction to be a boundary concern rather than a second cognitive world.
    Marker {
        opening: "расскажи про",
        kind: CognitiveActKind::Ask,
    },
    Marker {
        opening: "что такое",
        kind: CognitiveActKind::Ask,
    },
    Marker {
        opening: "почему",
        kind: CognitiveActKind::Explain,
    },
    Marker {
        opening: "объясни",
        kind: CognitiveActKind::Explain,
    },
    Marker {
        opening: "покажи",
        kind: CognitiveActKind::Inspect,
    },
    Marker {
        opening: "сравни",
        kind: CognitiveActKind::Compare,
    },
    Marker {
        opening: "проверь",
        kind: CognitiveActKind::Verify,
    },
    Marker {
        opening: "пожалуйста",
        kind: CognitiveActKind::Request,
    },
    Marker {
        opening: "запомни",
        kind: CognitiveActKind::Remember,
    },
    Marker {
        opening: "забудь",
        kind: CognitiveActKind::Forget,
    },
    Marker {
        opening: "отмени",
        kind: CognitiveActKind::Cancel,
    },
    Marker {
        opening: "приостанови",
        kind: CognitiveActKind::Pause,
    },
    Marker {
        opening: "продолжи",
        kind: CognitiveActKind::Resume,
    },
    Marker {
        opening: "предложи",
        kind: CognitiveActKind::Propose,
    },
    Marker {
        opening: "нет,",
        kind: CognitiveActKind::Correct,
    },
    Marker {
        opening: "да",
        kind: CognitiveActKind::Confirm,
    },
    Marker {
        opening: "нет",
        kind: CognitiveActKind::Reject,
    },
];

/// Whether an act, once authorized elsewhere, would change something.
///
/// The distinction exists so [`resolve::resolve_reference`] can refuse to pick a referent for one.
/// Asking about the wrong thing wastes a sentence; cancelling the wrong thing cancels something.
#[must_use]
pub const fn is_mutating(kind: CognitiveActKind) -> bool {
    matches!(
        kind,
        CognitiveActKind::Request
            | CognitiveActKind::Cancel
            | CognitiveActKind::Pause
            | CognitiveActKind::Resume
            | CognitiveActKind::Forget
            | CognitiveActKind::Remember
            | CognitiveActKind::Correct
    )
}

/// Interpret an utterance into a typed act, or refuse to.
///
/// Returns `None` when nothing in the vocabulary matches. That is the honest answer: an utterance
/// this build cannot read is not an `Ask` about the whole sentence, and answering as though it
/// were would put a shape on something nobody established. A caller receiving `None` has a person
/// to go back to, which is a better position than one holding a confident wrong act.
#[must_use]
pub fn interpret(
    utterance: &str,
    source: &str,
    now: OffsetDateTime,
) -> Option<MeaningInterpretation> {
    let trimmed = utterance.trim();
    if trimmed.is_empty() {
        return None;
    }
    let normalized = trimmed.to_lowercase();

    let matched = MARKERS
        .iter()
        .find(|marker| normalized.starts_with(marker.opening))?;

    let subject = normalized[matched.opening.len()..]
        .trim()
        .trim_end_matches(['.', '?', '!'])
        .trim()
        .to_owned();
    if subject.is_empty()
        && !matches!(
            matched.kind,
            CognitiveActKind::Confirm | CognitiveActKind::Reject
        )
    {
        // An opening on its own names no subject. An act whose subject is the empty string would
        // reach Mind asking about nothing in particular, and be indistinguishable from one that
        // named something.
        return None;
    }

    // Two markers matching means the utterance opens with something this vocabulary reads two
    // ways. Whichever is chosen, the other reading was possible, and a caller has to be able to
    // see that before acting on a mutating act.
    let ambiguous = MARKERS
        .iter()
        .filter(|marker| normalized.starts_with(marker.opening))
        .count()
        > 1;

    Some(MeaningInterpretation {
        utterance: trimmed.to_owned(),
        primary_act: CognitiveAct {
            act_id: Uuid::new_v4(),
            kind: matched.kind,
            subject,
            parameters: Vec::new(),
            source: source.to_owned(),
            evidence: Vec::new(),
        },
        references: Vec::new(),
        // A deterministic match is certain about which marker it matched, and says nothing about
        // whether the person meant what the marker means. The second question is not this
        // function's to answer, and a lower number here would be a guess dressed as modesty.
        confidence: if ambiguous { 0.5 } else { 1.0 },
        ambiguous,
        derived_at: now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000).expect("a fixed instant")
    }

    #[test]
    fn an_utterance_becomes_a_typed_act_rather_than_prose() {
        // C1: what crosses into Mind is the act, not the sentence.
        let interpreted = interpret("Explain why the journal is degraded", "person", at())
            .expect("a recognised opening");
        assert_eq!(interpreted.primary_act.kind, CognitiveActKind::Explain);
        assert_eq!(
            interpreted.primary_act.subject,
            "why the journal is degraded"
        );
        assert_eq!(interpreted.primary_act.source, "person");
        // The original is kept beside the act, not replaced by it: an interpretation that discards
        // what it was derived from cannot be argued with afterwards.
        assert_eq!(interpreted.utterance, "Explain why the journal is degraded");
    }

    #[test]
    fn two_surface_languages_reach_the_same_canonical_act() {
        // C7: the same canonical act from different surface forms.
        let english = interpret("Compare the last two sessions", "person", at()).expect("english");
        let russian = interpret("Сравни последние две сессии", "person", at()).expect("russian");
        assert_eq!(english.primary_act.kind, russian.primary_act.kind);
        assert_eq!(english.primary_act.kind, CognitiveActKind::Compare);
        // Different sentences, so different subjects: what is canonical is the act, not the words.
        assert_ne!(english.primary_act.subject, russian.primary_act.subject);
    }

    #[test]
    fn an_utterance_this_build_cannot_read_produces_nothing() {
        // The other half of C8: refusing is available without a model, and is what keeps the
        // boundary from inventing meaning when it has none.
        assert!(interpret("the kettle is boiling over there somewhere", "person", at()).is_none());
        assert!(interpret("   ", "person", at()).is_none());
        // An opening with nothing after it names no subject.
        assert!(interpret("explain", "person", at()).is_none());
    }

    #[test]
    fn an_opening_this_vocabulary_reads_two_ways_is_marked_ambiguous() {
        let corrected = interpret("No, the disk was fine", "person", at()).expect("an opening");
        assert!(
            corrected.ambiguous,
            "this opening is read as both a Correct and a Reject, and a caller has to see that"
        );
        let plain = interpret("Verify the chain", "person", at()).expect("an opening");
        assert!(!plain.ambiguous);
    }

    #[test]
    fn acts_that_would_change_something_are_distinguished_from_acts_that_would_not() {
        assert!(is_mutating(CognitiveActKind::Cancel));
        assert!(is_mutating(CognitiveActKind::Forget));
        assert!(!is_mutating(CognitiveActKind::Ask));
        assert!(!is_mutating(CognitiveActKind::Inspect));
    }
}
