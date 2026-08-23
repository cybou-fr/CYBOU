// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! How sure Mind is of something, in a vocabulary every layer shares.
//!
//! This lives in the protocol rather than in `epistemicd` for one reason: a standing that only the
//! organ that derived it can name is a standing that gets dropped at the first boundary it crosses.
//! ADR-0029 A4 asks that a disputed state still be disputed after retrieval, and a retrieval that
//! has no word for "disputed" cannot carry one — it would hand back the value and nothing else,
//! and the loss would look exactly like there having been nothing to lose.
//!
//! Naming it here does not move the authority. `epistemicd` remains the only thing that decides
//! what a subject's status *is*; everyone else may only carry what it decided, unchanged.

use serde::{Deserialize, Serialize};

/// Epistemic validity status of a proposition.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EpistemicStatus {
    /// Actively corroborated by recent observations.
    Observed,
    /// Previously observed but beyond freshness horizon without corroboration.
    Stale,
    /// Contradicted by competing observations with conflicting values.
    Disputed,
    /// Explicitly superseded by a newer belief revision.
    Superseded,
    /// Not yet observed or unresolvable.
    ///
    /// The default, deliberately. Something that arrives without a standing has not been
    /// established to be settled; treating silence as corroboration is how an unread projection
    /// comes to read as a healthy one.
    #[default]
    Unknown,
}

impl EpistemicStatus {
    /// Whether this standing is one a reader must be told about before acting on the value.
    ///
    /// `Observed` is the only status that does not qualify what rests on it. Everything else is a
    /// reason a person might decide differently, which is the definition being encoded.
    #[must_use]
    pub const fn qualifies(self) -> bool {
        !matches!(self, Self::Observed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_corroborated_standing_leaves_a_claim_unqualified() {
        assert!(!EpistemicStatus::Observed.qualifies());
        for qualified in [
            EpistemicStatus::Stale,
            EpistemicStatus::Disputed,
            EpistemicStatus::Superseded,
            EpistemicStatus::Unknown,
        ] {
            assert!(qualified.qualifies(), "{qualified:?}");
        }
    }

    #[test]
    fn something_arriving_without_a_standing_is_not_treated_as_settled() {
        assert_eq!(EpistemicStatus::default(), EpistemicStatus::Unknown);
        assert!(EpistemicStatus::default().qualifies());
    }
}
