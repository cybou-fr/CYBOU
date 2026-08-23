// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What was proposed for attention, and what became of it — in a vocabulary every layer shares.
//!
//! Here for the same reason [`crate::epistemic::EpistemicStatus`] is: a decision only the organ
//! that made it can name gets dropped at the first boundary it crosses. An admission that said
//! "three got in" and could not say "out of two thousand" would let whatever renders the moment
//! present a fragment as the whole of it, and the loss would be invisible from the outside.
//!
//! Naming the types here moves no authority. `workspaced` remains the only thing that decides what
//! enters the conscious moment; everyone else may read what it decided.

use serde::{Deserialize, Serialize};

use crate::epistemic::EpistemicStatus;

/// A concept asking to be noticed.
///
/// Deliberately not a concept node or an activated concept. Association does not acquire the
/// standing of attention by being the same struct — whoever holds both organs does the conversion,
/// in the open, and that conversion is where a reader can see association becoming a proposal.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionProposal {
    /// The concept being proposed.
    pub label: String,
    /// How strongly whatever proposed it reached it, in [0.0, 1.0].
    pub relevance: f64,
    /// Why it was proposed, carried through from the retrieval that found it.
    pub reason: String,
    /// How the epistemic owner stood on it, carried through from the same retrieval.
    ///
    /// ADR-0029 A4 does not stop at the retrieval boundary. A disputed concept that reached
    /// attention with its standing stripped would be presented as settled by whatever draws the
    /// moment, and nothing downstream would have any way to know it had been contested.
    #[serde(default)]
    pub epistemic_status: EpistemicStatus,
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
    /// Whether what was offered was itself everything there was to offer.
    ///
    /// Kept apart from `complete` because the two are different facts and a reader needs to tell
    /// them apart: a quota turning proposals away is attention being busy, and a budget cutting a
    /// walk short is the retrieval never having finished. Folding them into one flag would report
    /// "there is more" without saying where the more is, and the two have different remedies.
    ///
    /// This was not here when admission was written, and the gap was invisible from inside
    /// `workspaced`: a truncated activation offering one concept looks exactly like a graph that
    /// holds one concept. It took walking the whole path to see it.
    #[serde(default = "everything_was_offered")]
    pub upstream_complete: bool,
}

/// The default for [`Admission::upstream_complete`] when a record predates the field.
const fn everything_was_offered() -> bool {
    true
}
