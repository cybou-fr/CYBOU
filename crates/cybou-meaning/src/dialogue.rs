// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What a conversation is allowed to remember between turns.
//!
//! Without any memory, "it" in the second sentence points at nothing, and a person has to restate
//! the subject every time — which is not a conversation, it is a series of unrelated commands. The
//! obvious repair is to remember what was being talked about and let the next turn's pronoun mean
//! that. The obvious repair is also a machine for guessing, and ADR-0031 C2 exists precisely to
//! stop the guess: *an unresolved reference stays unresolved rather than resolving to whichever
//! candidate scored highest.*
//!
//! So this remembers **referents, not a topic**. Everything mentioned in the recent past is offered
//! to [`crate::resolve::resolve_reference`] as one more candidate, alongside whatever the caller
//! already had. Memory can therefore make an ambiguity visible — two things were mentioned, and now
//! "it" could be either — and it has no way to make one disappear. There is deliberately no
//! accessor for "the current subject", because a type that could answer that question would be
//! asked it, and answering it is the guess.
//!
//! Memory is bounded three ways, and each bound answers a different failure:
//!
//! - **Turns.** A referent from twenty exchanges ago is not what "it" means, and offering it as a
//!   candidate makes every pronoun ambiguous forever.
//! - **Time.** A conversation resumed the next morning is a new conversation, whatever the turn
//!   counter says.
//! - **Erasure.** What ADR-0028 erased leaves here too, in the same act. A referent that survived
//!   in dialogue memory would let a person be offered, by name, a thing they had erased.

use time::{Duration, OffsetDateTime};

use crate::resolve::Candidate;

/// The most referents held at once, however many turns fit inside the bounds.
///
/// A single turn can mention a great many things — a listing, a sweep — and a candidate list the
/// length of a directory is not a clarification a person can answer. Oldest go first.
const MAX_REMEMBERED: usize = 32;

/// Something that was talked about, and when.
#[derive(Clone, Debug, PartialEq)]
struct Mention {
    /// Stable identity of the thing mentioned.
    target_id: String,
    /// How it was named where it lives.
    label: String,
    /// The turn it was mentioned in.
    turn: u64,
    /// When it was mentioned.
    at: OffsetDateTime,
    /// The erasure epoch it belongs to, so erasure can reach it.
    epoch: u64,
}

/// The referents a conversation may still be pointing at.
///
/// Holds no text, no summary and no inferred subject: only what was named, when, and under which
/// erasure epoch.
#[derive(Clone, Debug)]
pub struct Dialogue {
    /// How many turns back a mention stays available.
    turns_remembered: u64,
    /// How long a mention stays available regardless of turn count.
    memory_span: Duration,
    /// The current turn.
    turn: u64,
    /// What has been mentioned, oldest first.
    mentions: Vec<Mention>,
}

impl Dialogue {
    /// Begin a conversation that remembers this far back and no further.
    ///
    /// Both bounds are the caller's, because how long "it" stays meaningful is a property of the
    /// surface a person is talking through, not of this module.
    #[must_use]
    pub const fn new(turns_remembered: u64, memory_span: Duration) -> Self {
        Self {
            turns_remembered,
            memory_span,
            turn: 0,
            mentions: Vec::new(),
        }
    }

    /// The turn a conversation is on.
    #[must_use]
    pub const fn turn(&self) -> u64 {
        self.turn
    }

    /// Start the next turn, dropping whatever has fallen outside the bounds.
    ///
    /// Expiry happens here rather than lazily on read, so that what a person is offered as a
    /// candidate is what the conversation holds, and not a function of when somebody last looked.
    pub fn open_turn(&mut self, now: OffsetDateTime) {
        self.turn = self.turn.saturating_add(1);
        self.expire(now);
    }

    /// Note that something was talked about in the current turn.
    ///
    /// Mentioning the same thing again moves it forward rather than listing it twice: a person who
    /// named a server in three consecutive turns has one thing in mind, not three.
    pub fn mention(&mut self, target_id: &str, label: &str, epoch: u64, now: OffsetDateTime) {
        self.mentions
            .retain(|mention| mention.target_id != target_id);
        self.mentions.push(Mention {
            target_id: target_id.to_owned(),
            label: label.to_owned(),
            turn: self.turn,
            at: now,
            epoch,
        });
        while self.mentions.len() > MAX_REMEMBERED {
            self.mentions.remove(0);
        }
    }

    /// The remembered referents, most recently mentioned first.
    ///
    /// Offered to the resolver as candidates. Nothing here is a selection: if two are returned and
    /// the reference is a pronoun, the resolver's answer is still that it did not settle — now with
    /// both possibilities named, which is what makes the question askable.
    #[must_use]
    pub fn candidates(&self, now: OffsetDateTime) -> Vec<Candidate> {
        let mut live: Vec<&Mention> = self
            .mentions
            .iter()
            .filter(|mention| self.within_bounds(mention, now))
            .collect();
        live.reverse();
        live.into_iter()
            .map(|mention| Candidate {
                target_id: mention.target_id.clone(),
                label: mention.label.clone(),
            })
            .collect()
    }

    /// Forget everything belonging to an erased epoch.
    ///
    /// ADR-0028 erases a thing; this is the part of that erasure that lives in a conversation. A
    /// referent left behind here would let the system offer, by name, something a person had
    /// already had removed — and offering it by name is the disclosure the erasure was for.
    pub fn forget_epoch(&mut self, epoch: u64) {
        self.mentions.retain(|mention| mention.epoch != epoch);
    }

    /// Forget everything. The conversation is over.
    pub fn clear(&mut self) {
        self.mentions.clear();
    }

    /// Drop mentions that have fallen outside either bound.
    fn expire(&mut self, now: OffsetDateTime) {
        let turn = self.turn;
        let turns_remembered = self.turns_remembered;
        let memory_span = self.memory_span;
        self.mentions.retain(|mention| {
            turn.saturating_sub(mention.turn) <= turns_remembered && now - mention.at <= memory_span
        });
    }

    /// Whether a mention is still inside both bounds.
    fn within_bounds(&self, mention: &Mention, now: OffsetDateTime) -> bool {
        self.turn.saturating_sub(mention.turn) <= self.turns_remembered
            && now - mention.at <= self.memory_span
    }
}

#[cfg(test)]
mod tests {
    use cybou_protocol::meaning::CognitiveActKind;

    use super::*;
    use crate::resolve::resolve_reference;

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    fn conversation() -> Dialogue {
        Dialogue::new(3, Duration::minutes(10))
    }

    #[test]
    fn memory_makes_an_ambiguity_visible_and_never_makes_one_disappear() {
        // The rule the module exists for. Two things were mentioned; "it" could be either; the
        // resolver still refuses. What memory added is that the person can now be asked *which*.
        let mut dialogue = conversation();
        dialogue.open_turn(at(0));
        dialogue.mention("srv-1", "debian primary server", 1, at(0));
        dialogue.mention("srv-2", "debian staging server", 1, at(1));

        dialogue.open_turn(at(2));
        let resolution =
            resolve_reference("it", &dialogue.candidates(at(2)), CognitiveActKind::Ask);
        assert!(!resolution.resolved, "a pronoun was resolved from memory");
        assert!(resolution.selected_target_id.is_none());
    }

    #[test]
    fn a_referent_falls_out_after_the_turns_it_was_given() {
        let mut dialogue = conversation();
        dialogue.open_turn(at(0));
        dialogue.mention("srv-1", "debian primary server", 1, at(0));

        for turn in 1..=3 {
            dialogue.open_turn(at(turn));
            assert_eq!(dialogue.candidates(at(turn)).len(), 1, "turn {turn}");
        }
        dialogue.open_turn(at(4));
        assert!(
            dialogue.candidates(at(4)).is_empty(),
            "a referent outlived the turns it was given"
        );
    }

    #[test]
    fn a_conversation_resumed_much_later_is_a_new_conversation() {
        // The turn counter says one exchange has passed. The clock says the morning did.
        let mut dialogue = conversation();
        dialogue.open_turn(at(0));
        dialogue.mention("srv-1", "debian primary server", 1, at(0));

        dialogue.open_turn(at(60 * 60 * 9));
        assert!(dialogue.candidates(at(60 * 60 * 9)).is_empty());
    }

    #[test]
    fn what_was_erased_leaves_the_conversation_too() {
        // Left behind, it would let the system offer by name a thing that had been erased — which
        // is the disclosure the erasure was for.
        let mut dialogue = conversation();
        dialogue.open_turn(at(0));
        dialogue.mention("srv-1", "debian primary server", 7, at(0));
        dialogue.mention("srv-2", "debian staging server", 8, at(0));

        dialogue.forget_epoch(7);
        let remembered = dialogue.candidates(at(1));
        assert_eq!(remembered.len(), 1);
        assert_eq!(remembered[0].target_id, "srv-2");
    }

    #[test]
    fn the_thing_most_recently_talked_about_is_offered_first() {
        // Order, not selection: the resolver still decides, and still refuses when it cannot.
        let mut dialogue = conversation();
        dialogue.open_turn(at(0));
        dialogue.mention("srv-1", "debian primary server", 1, at(0));
        dialogue.mention("srv-2", "debian staging server", 1, at(1));

        let offered = dialogue.candidates(at(2));
        assert_eq!(offered[0].target_id, "srv-2");
        assert_eq!(offered[1].target_id, "srv-1");
    }

    #[test]
    fn mentioning_the_same_thing_again_is_one_thing_not_two() {
        let mut dialogue = conversation();
        dialogue.open_turn(at(0));
        dialogue.mention("srv-1", "debian primary server", 1, at(0));
        dialogue.open_turn(at(1));
        dialogue.mention("srv-1", "debian primary server", 1, at(1));

        assert_eq!(dialogue.candidates(at(1)).len(), 1);
        // And it is fresh again: it was just talked about.
        dialogue.open_turn(at(2));
        dialogue.open_turn(at(3));
        dialogue.open_turn(at(4));
        assert_eq!(dialogue.candidates(at(4)).len(), 1);
    }

    #[test]
    fn a_turn_that_names_a_great_many_things_does_not_become_the_candidate_list() {
        // A candidate list the length of a directory is not a clarification a person can answer.
        let mut dialogue = conversation();
        dialogue.open_turn(at(0));
        for index in 0..200 {
            dialogue.mention(&format!("f-{index}"), &format!("file {index}"), 1, at(0));
        }
        assert_eq!(dialogue.candidates(at(0)).len(), MAX_REMEMBERED);
        // The ones kept are the ones most recently named.
        assert_eq!(dialogue.candidates(at(0))[0].target_id, "f-199");
    }

    #[test]
    fn memory_lets_a_description_resolve_that_could_not_have_before() {
        // What memory is for. The caller offered nothing; the conversation had named one thing;
        // a description that matches it settles — because it describes, not because it was recent.
        let mut dialogue = conversation();
        dialogue.open_turn(at(0));
        dialogue.mention("srv-1", "debian primary server", 1, at(0));

        dialogue.open_turn(at(1));
        let resolution = resolve_reference(
            "the primary server",
            &dialogue.candidates(at(1)),
            CognitiveActKind::Ask,
        );
        assert!(resolution.resolved);
        assert_eq!(resolution.selected_target_id.as_deref(), Some("srv-1"));
    }

    #[test]
    fn recency_alone_never_settles_a_reference_that_would_change_something() {
        // Memory changes which candidates exist. It does not change the rule that a mutating act
        // is not resolved on a score.
        let mut dialogue = conversation();
        dialogue.open_turn(at(0));
        dialogue.mention("run-1", "the nightly consolidation run", 1, at(0));

        let resolution = resolve_reference(
            "the nightly consolidation run",
            &dialogue.candidates(at(0)),
            CognitiveActKind::Cancel,
        );
        assert!(!resolution.resolved);
    }

    #[test]
    fn a_conversation_that_is_over_remembers_nothing() {
        let mut dialogue = conversation();
        dialogue.open_turn(at(0));
        dialogue.mention("srv-1", "debian primary server", 1, at(0));
        dialogue.clear();
        assert!(dialogue.candidates(at(0)).is_empty());
        assert_eq!(dialogue.turn(), 1);
    }
}
