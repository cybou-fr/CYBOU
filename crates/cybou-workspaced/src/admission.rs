// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Who gets into the conscious moment, and what a proposal is not allowed to do to get there.
//!
//! ADR-0014's amendment says it in one line: *relevance discovered by associative retrieval is not
//! permission to displace the current focus.* Without that rule the word *lemon* activates fifty
//! related things and admits all of them — a cognitive denial of service Mind performs on itself,
//! with every individual step looking reasonable.
//!
//! The failure this prevents is easy to miss because a naive workspace still looks correct while
//! suffering it. `accept` keeps the moment at capacity by dropping the oldest, so a flood of
//! proposals leaves the buffer exactly as bounded as before — and empty of everything that
//! mattered. **Bounded in size is not bounded in attention.** A workspace that answered a
//! `NeedSignal` a moment ago and now holds thirty-two associations of the word *lemon* has stayed
//! within every limit it declared and lost the only thing it was for.
//!
//! So proposals are admitted under a different rule than contributions:
//!
//! - **A proposal never evicts a resident.** Something that happened outranks something that came
//!   to mind, whatever their scores. This is the amendment, and it is structural rather than a
//!   threshold: no relevance is high enough, because relevance is not the currency being spent.
//! - **Proposals share a quota, not the capacity.** Even an empty workspace does not become
//!   entirely associative, because a moment made only of what a word suggested has nothing left to
//!   notice an interruption with.
//! - **What was refused is counted.** A caller that sees three admitted out of two thousand knows
//!   it asked for too much. Silently keeping three would have looked identical and told it nothing.
//!
//! Ordering is by relevance, ties by label, so the same proposals against the same moment always
//! produce the same admission — the property everything downstream compares against.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// A concept asking to be noticed.
///
/// Deliberately not a `ConceptNode` or an `ActivatedConcept`: `workspaced` does not read
/// `contextd`'s types, so association cannot acquire the standing of attention by being the same
/// struct. Whoever holds both organs does the conversion, in the open.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionProposal {
    /// The concept being proposed.
    pub label: String,
    /// How strongly whatever proposed it reached it, in [0.0, 1.0].
    pub relevance: f64,
    /// Why it was proposed, carried through from the retrieval that found it.
    pub reason: String,
}

/// The outcome of offering proposals to a moment.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Admission {
    /// What got in, strongest first.
    pub admitted: Vec<AttentionProposal>,
    /// How many were offered.
    pub considered: usize,
    /// How many were turned away because the quota was full.
    pub refused_for_quota: usize,
    /// How many were turned away because nothing actually reached them.
    pub refused_unreached: usize,
    /// How many named a concept another proposal had already named.
    pub refused_duplicate: usize,
    /// Whether every proposal offered was admitted.
    ///
    /// False is the ordinary case for a large activation, and saying so is the point: a caller
    /// reading a short list as the whole list is the failure this field exists to prevent.
    pub complete: bool,
}

/// The share of a workspace that proposals may occupy at once.
///
/// A quarter. The number is arbitrary in the way a threshold is; what is not arbitrary is that it
/// is well below one. A moment made entirely of what a word suggested has nothing left to notice an
/// interruption with, which is the state the quota exists to make unreachable.
const PROPOSAL_SHARE: f64 = 0.25;

/// How many slots proposals may hold in a workspace of this capacity.
///
/// At least one, so a small workspace can still be reminded of something; never all of them, so no
/// workspace can be made entirely of reminders.
#[must_use]
pub fn proposal_quota(capacity: usize) -> usize {
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a workspace capacity is a handful of slots; the quota is a count, not a measurement"
    )]
    let quota = (capacity as f64 * PROPOSAL_SHARE) as usize;
    quota.clamp(1, capacity.saturating_sub(1).max(1))
}

/// Decide which proposals may enter a moment that already holds `occupied` contributions.
///
/// Nothing here can remove a resident. The worst a flood of proposals can do is fill the quota and
/// be counted; the moment it arrived into is the moment it leaves behind.
#[must_use]
pub fn admit(proposals: &[AttentionProposal], capacity: usize, occupied: usize) -> Admission {
    let considered = proposals.len();

    let mut ranked: Vec<&AttentionProposal> = proposals
        .iter()
        // Nothing reached it, so nothing is proposing it. A zero-relevance concept admitted on the
        // strength of being in the list would be attention paid for no reason at all.
        .filter(|proposal| proposal.relevance > 0.0)
        .collect();
    let refused_unreached = considered - ranked.len();

    ranked.sort_by(|left, right| {
        right
            .relevance
            .partial_cmp(&left.relevance)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.label.cmp(&right.label))
    });
    // One concept is one concept, however many paths reached it. Kept by the strongest, since the
    // list is already in that order.
    let mut seen: HashSet<&str> = HashSet::new();
    let before_dedup = ranked.len();
    ranked.retain(|proposal| seen.insert(proposal.label.as_str()));
    let refused_duplicate = before_dedup - ranked.len();

    // Two independent ceilings, and the smaller binds. The quota keeps proposals from owning the
    // moment; the free space keeps them from displacing what is in it.
    let room = capacity.saturating_sub(occupied);
    let allowed = proposal_quota(capacity).min(room);

    let admitted: Vec<AttentionProposal> = ranked
        .iter()
        .take(allowed)
        .map(|proposal| (*proposal).clone())
        .collect();
    let refused_for_quota = ranked.len() - admitted.len();

    Admission {
        considered,
        refused_for_quota,
        refused_unreached,
        refused_duplicate,
        // A duplicate is not a refusal a caller needs to act on — it asked for one thing twice and
        // got it once — so it does not make an admission incomplete.
        complete: refused_for_quota == 0 && refused_unreached == 0,
        admitted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proposal(label: &str, relevance: f64) -> AttentionProposal {
        AttentionProposal {
            label: label.to_owned(),
            relevance,
            reason: format!("lemon → {label} (episodic, strength {relevance:.2}) at depth 1"),
        }
    }

    fn flood(count: usize) -> Vec<AttentionProposal> {
        (0..count)
            .map(|index| {
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "a test fixture generating distinct scores"
                )]
                let relevance = 0.5 + (index % 100) as f64 / 1000.0;
                proposal(&format!("concept-{index:04}"), relevance)
            })
            .collect()
    }

    #[test]
    fn thousands_of_associations_cannot_take_over_the_moment() {
        // A11, and the amendment to ADR-0014. The word lemon activating fifty related things must
        // not become fifty things Mind is now paying attention to.
        let admission = admit(&flood(5000), 32, 0);
        assert_eq!(admission.admitted.len(), proposal_quota(32));
        assert_eq!(admission.considered, 5000);
        assert!(!admission.complete);
        assert_eq!(admission.refused_for_quota, 5000 - admission.admitted.len());
    }

    #[test]
    fn a_proposal_never_takes_a_slot_something_that_happened_is_in() {
        // The rule stated structurally rather than as a threshold: no relevance is high enough,
        // because relevance is not the currency being spent.
        let full = admit(&[proposal("honey", 1.0)], 32, 32);
        assert!(full.admitted.is_empty());
        assert!(!full.complete);

        let nearly_full = admit(&[proposal("honey", 1.0), proposal("tea", 0.9)], 32, 31);
        assert_eq!(nearly_full.admitted.len(), 1, "one free slot, one admitted");
        assert_eq!(nearly_full.admitted[0].label, "honey");
    }

    #[test]
    fn no_workspace_becomes_entirely_associative() {
        // A moment made only of what a word suggested has nothing left to notice an interruption
        // with.
        for capacity in 1..=256usize {
            let quota = proposal_quota(capacity);
            assert!(quota >= 1, "capacity {capacity} could not be reminded");
            if capacity > 1 {
                assert!(
                    quota < capacity,
                    "capacity {capacity} could be made entirely of reminders"
                );
            }
            let admission = admit(&flood(1000), capacity, 0);
            assert!(admission.admitted.len() <= quota);
        }
    }

    #[test]
    fn what_was_turned_away_is_counted_rather_than_dropped_quietly() {
        // Three admitted out of two thousand and three admitted out of three look identical unless
        // the difference is reported.
        let admission = admit(&flood(2000), 12, 0);
        assert_eq!(
            admission.considered,
            admission.admitted.len()
                + admission.refused_for_quota
                + admission.refused_unreached
                + admission.refused_duplicate,
            "every proposal offered is accounted for exactly once"
        );
        assert!(!admission.complete);
    }

    #[test]
    fn a_small_activation_that_fits_is_complete() {
        let admission = admit(&[proposal("honey", 0.8)], 32, 0);
        assert_eq!(admission.admitted.len(), 1);
        assert!(admission.complete);
        assert_eq!(admission.refused_for_quota, 0);
    }

    #[test]
    fn a_concept_nothing_reached_is_not_admitted_for_being_in_the_list() {
        let admission = admit(&[proposal("honey", 0.8), proposal("unreached", 0.0)], 32, 0);
        assert_eq!(admission.admitted.len(), 1);
        assert_eq!(admission.refused_unreached, 1);
        assert!(!admission.complete);
    }

    #[test]
    fn the_strongest_proposals_are_the_ones_that_get_in() {
        let admission = admit(
            &[
                proposal("tea", 0.4),
                proposal("yellow", 0.9),
                proposal("honey", 0.8),
            ],
            8,
            6,
        );
        assert_eq!(admission.admitted.len(), 2, "two free slots");
        assert_eq!(admission.admitted[0].label, "yellow");
        assert_eq!(admission.admitted[1].label, "honey");
    }

    #[test]
    fn the_same_proposals_against_the_same_moment_admit_the_same_things() {
        // Ties are broken by label, so nothing downstream compares against a shuffled answer.
        let tied = vec![
            proposal("zinc", 0.7),
            proposal("apple", 0.7),
            proposal("mango", 0.7),
        ];
        let first = admit(&tied, 8, 6);
        for _ in 0..8 {
            assert_eq!(admit(&tied, 8, 6), first);
        }
        assert_eq!(first.admitted[0].label, "apple");
    }

    #[test]
    fn one_concept_proposed_twice_takes_one_slot() {
        let admission = admit(&[proposal("honey", 0.8), proposal("honey", 0.8)], 32, 0);
        assert_eq!(admission.admitted.len(), 1);
        assert_eq!(admission.refused_duplicate, 1);
        // Asking for one thing twice and getting it once is not a partial answer.
        assert!(admission.complete);
    }

    #[test]
    fn admitting_nothing_into_an_empty_moment_is_complete() {
        let admission = admit(&[], 32, 0);
        assert!(admission.admitted.is_empty());
        assert!(admission.complete, "nothing was asked and nothing refused");
    }
}
