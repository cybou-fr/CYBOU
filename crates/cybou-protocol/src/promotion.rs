// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What a candidate has to have done before it becomes something Mind acts on.
//!
//! ADR-0032 gives a `PromotionGate` with three criteria and nothing that evaluates it. A gate
//! nothing evaluates is a comment, and the thing it was written to stop — a pattern noticed once
//! becoming a rule Mind applies — happens exactly as it would have without it.
//!
//! Two failures are worth naming, because both look like passing the gate.
//!
//! **The unit of evidence is the episode, not the message.** One episode that produced three
//! messages is one demonstration, not three. Counting messages would let a single lucky occasion
//! satisfy "three independent episodes" while nothing was ever repeated, and repeatability is the
//! entire content of that criterion. The same trap sits in the success rate: an episode that went
//! well ten times and one that failed once is eleven-twelfths successful by message and half
//! successful by episode. So an episode counts as a success only if everything observed in it
//! succeeded, and the rate is over episodes.
//!
//! **Association is not promoted.** ADR-0029 A5: association alone never creates durable knowledge.
//! An associative candidate is refused here not because its evidence is weak but because the
//! associative layer is a cache — it is recomputed from the Journal, and promoting into it would
//! create a durable artifact that the next erasure epoch rebuilds from nothing. `contextd` may
//! offer associations, co-occurrence and activation paths as *inputs* to a candidate. Nothing about
//! having offered them makes the candidate promotable.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::learning::{LearningCandidate, LearningLayer, PromotionGate};

/// One thing that happened when the candidate's generalization was in play.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemonstratedOutcome {
    /// The episode it happened in.
    ///
    /// This is what makes a demonstration independent of another. Without it, promotion counts
    /// messages, and a candidate can look repeated without ever having been repeated.
    pub episode: Uuid,
    /// The contribution recording it.
    pub outcome: Uuid,
    /// Whether it went the way the generalization said it would.
    pub succeeded: bool,
}

/// What a promotion was granted on.
///
/// The numbers are carried out rather than discarded, so a promotion can be argued with later by
/// someone who was not there when it was granted.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Promoted {
    /// The candidate that passed.
    pub candidate_id: Uuid,
    /// The layer it was promoted into.
    pub layer: LearningLayer,
    /// How many distinct episodes demonstrated it.
    pub independent_episodes: u32,
    /// The share of those episodes that went as predicted.
    pub success_rate: f64,
}

/// Why a candidate was not promoted.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PromotionRefused {
    /// The candidate targets the associative layer.
    ///
    /// ADR-0029 A5. Not a comment on the evidence: the associative layer is derived state that an
    /// erasure epoch rebuilds, so a durable artifact promoted into it would be a durable claim on
    /// something that is recomputed from scratch.
    AssociationIsNotPromoted,
    /// Nothing was ever observed to happen.
    NothingDemonstrated,
    /// It happened, but not in enough separate episodes to be called repeatable.
    TooFewIndependentEpisodes {
        /// Distinct episodes observed.
        distinct: u32,
        /// Distinct episodes the gate asks for.
        required: u32,
    },
    /// It was repeated often enough and did not work often enough.
    SuccessRateBelowGate {
        /// The share of episodes that went as predicted.
        observed: f64,
        /// The share the gate asks for.
        required: f64,
    },
    /// Replay evaluation has not been run, or did not pass.
    ///
    /// Separate from the counts on purpose: a candidate that has never been evaluated and one that
    /// was evaluated and failed are both unpromotable, and neither is "not enough evidence yet".
    EvaluationNotPassed,
}

impl core::fmt::Display for PromotionRefused {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::AssociationIsNotPromoted => write!(
                formatter,
                "an associative candidate is recomputed, not promoted"
            ),
            Self::NothingDemonstrated => {
                write!(formatter, "nothing was observed to happen")
            }
            Self::TooFewIndependentEpisodes { distinct, required } => write!(
                formatter,
                "{distinct} independent episode(s), {required} required"
            ),
            Self::SuccessRateBelowGate { observed, required } => write!(
                formatter,
                "{observed:.2} of episodes succeeded, {required:.2} required"
            ),
            Self::EvaluationNotPassed => {
                write!(formatter, "replay evaluation has not passed")
            }
        }
    }
}

impl core::error::Error for PromotionRefused {}

/// Decide whether a candidate has earned promotion.
///
/// Deterministic and total: the same candidate, outcomes and gate always give the same answer, and
/// every answer is either a promotion carrying the numbers behind it or a refusal naming which
/// criterion was not met. There is no "probably ready".
///
/// # Errors
///
/// Returns [`PromotionRefused`] naming the first criterion the candidate does not meet, checked in
/// the order a reader would check them: what it is, whether anything happened, whether it happened
/// more than once, whether it worked, and whether it was evaluated.
pub fn evaluate_promotion(
    candidate: &LearningCandidate,
    outcomes: &[DemonstratedOutcome],
    gate: &PromotionGate,
) -> Result<Promoted, PromotionRefused> {
    if candidate.layer == LearningLayer::Associative {
        return Err(PromotionRefused::AssociationIsNotPromoted);
    }
    if outcomes.is_empty() {
        return Err(PromotionRefused::NothingDemonstrated);
    }

    // An episode succeeded only if everything observed in it succeeded. The alternative — counting
    // successful messages — lets one episode that went well many times outvote several that did
    // not, and calls the result a success rate over independent demonstrations.
    let mut episodes: Vec<(Uuid, bool)> = Vec::new();
    for observed in outcomes {
        if let Some(entry) = episodes
            .iter_mut()
            .find(|(episode, _)| *episode == observed.episode)
        {
            entry.1 &= observed.succeeded;
        } else {
            episodes.push((observed.episode, observed.succeeded));
        }
    }

    let distinct = u32::try_from(episodes.len()).unwrap_or(u32::MAX);
    if distinct < gate.min_independent_episodes {
        return Err(PromotionRefused::TooFewIndependentEpisodes {
            distinct,
            required: gate.min_independent_episodes,
        });
    }

    let succeeded = episodes.iter().filter(|(_, ok)| *ok).count();
    #[allow(
        clippy::cast_precision_loss,
        reason = "episode counts are small; a rate is compared against a threshold, not stored"
    )]
    let success_rate = succeeded as f64 / episodes.len() as f64;
    if success_rate < gate.min_success_rate {
        return Err(PromotionRefused::SuccessRateBelowGate {
            observed: success_rate,
            required: gate.min_success_rate,
        });
    }

    if !gate.evaluation_passed {
        return Err(PromotionRefused::EvaluationNotPassed);
    }

    Ok(Promoted {
        candidate_id: candidate.candidate_id,
        layer: candidate.layer,
        independent_episodes: distinct,
        success_rate,
    })
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use super::*;

    fn at() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000).expect("a fixed instant")
    }

    fn candidate(layer: LearningLayer) -> LearningCandidate {
        LearningCandidate {
            candidate_id: Uuid::from_u128(1),
            layer,
            source_evidence: vec![Uuid::from_u128(10)],
            outcome_evidence: vec![Uuid::from_u128(11)],
            generalization: "restart nginx when the socket refuses".to_owned(),
            scope: "service.nginx".to_owned(),
            derivation_version: 1,
            created_at: at(),
        }
    }

    fn outcome(episode: u128, outcome: u128, succeeded: bool) -> DemonstratedOutcome {
        DemonstratedOutcome {
            episode: Uuid::from_u128(episode),
            outcome: Uuid::from_u128(outcome),
            succeeded,
        }
    }

    fn evaluated() -> PromotionGate {
        PromotionGate {
            evaluation_passed: true,
            ..PromotionGate::default()
        }
    }

    #[test]
    fn association_alone_never_becomes_durable_knowledge() {
        // ADR-0029 A5. Not a comment on the evidence — the evidence here is overwhelming and the
        // answer is still no, because the associative layer is recomputed rather than kept.
        let overwhelming: Vec<_> = (0..50u128)
            .map(|index| outcome(index, 1000 + index, true))
            .collect();
        let refusal = evaluate_promotion(
            &candidate(LearningLayer::Associative),
            &overwhelming,
            &evaluated(),
        )
        .expect_err("a refusal");
        assert_eq!(refusal, PromotionRefused::AssociationIsNotPromoted);
    }

    #[test]
    fn one_episode_that_happened_three_times_is_one_demonstration() {
        // The trap the episode unit exists for. Counting messages, this passes with room to spare
        // and nothing was ever repeated.
        let same_episode = [
            outcome(1, 100, true),
            outcome(1, 101, true),
            outcome(1, 102, true),
        ];
        let refusal = evaluate_promotion(
            &candidate(LearningLayer::Procedural),
            &same_episode,
            &evaluated(),
        )
        .expect_err("a refusal");
        assert_eq!(
            refusal,
            PromotionRefused::TooFewIndependentEpisodes {
                distinct: 1,
                required: 3,
            }
        );
    }

    #[test]
    fn one_episode_that_went_well_repeatedly_does_not_outvote_the_ones_that_did_not() {
        // The same trap in the rate. By message this is ten of twelve, comfortably above the gate.
        // By episode it is one of three, and one of three is what actually happened.
        let mut observed: Vec<_> = (0..10u128)
            .map(|index| outcome(1, 100 + index, true))
            .collect();
        observed.push(outcome(2, 200, false));
        observed.push(outcome(3, 300, false));

        let refusal = evaluate_promotion(
            &candidate(LearningLayer::Procedural),
            &observed,
            &evaluated(),
        )
        .expect_err("a refusal");
        match refusal {
            PromotionRefused::SuccessRateBelowGate { observed, .. } => {
                assert!(
                    (observed - 1.0 / 3.0).abs() < 1e-9,
                    "the rate was counted over messages: {observed}"
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn an_episode_that_failed_once_is_a_failed_episode() {
        // Partial success within one occasion is not success. A procedure that worked twice and
        // then broke something did not work that time.
        let observed = [
            outcome(1, 100, true),
            outcome(1, 101, false),
            outcome(2, 200, true),
            outcome(3, 300, true),
        ];
        let refusal = evaluate_promotion(
            &candidate(LearningLayer::Procedural),
            &observed,
            &evaluated(),
        )
        .expect_err("a refusal");
        match refusal {
            PromotionRefused::SuccessRateBelowGate { observed, .. } => {
                assert!((observed - 2.0 / 3.0).abs() < 1e-9, "{observed}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_candidate_that_earned_it_is_promoted_with_the_numbers_it_earned_it_on() {
        let observed = [
            outcome(1, 100, true),
            outcome(2, 200, true),
            outcome(3, 300, true),
        ];
        let promoted = evaluate_promotion(
            &candidate(LearningLayer::Procedural),
            &observed,
            &evaluated(),
        )
        .expect("three independent episodes, all successful, evaluation passed");
        assert_eq!(promoted.independent_episodes, 3);
        assert!((promoted.success_rate - 1.0).abs() < f64::EPSILON);
        assert_eq!(promoted.layer, LearningLayer::Procedural);
        assert_eq!(promoted.candidate_id, Uuid::from_u128(1));
    }

    #[test]
    fn nothing_observed_is_refused_rather_than_counted_as_nothing_going_wrong() {
        // A candidate nobody ever tried has a perfect record in the sense that matters least.
        let refusal = evaluate_promotion(&candidate(LearningLayer::Procedural), &[], &evaluated())
            .expect_err("a refusal");
        assert_eq!(refusal, PromotionRefused::NothingDemonstrated);
    }

    #[test]
    fn evidence_does_not_substitute_for_evaluation() {
        // The default gate has never run its replay evaluation, and a candidate that has never
        // been evaluated is not the same as one with too little evidence.
        let observed = [
            outcome(1, 100, true),
            outcome(2, 200, true),
            outcome(3, 300, true),
        ];
        let refusal = evaluate_promotion(
            &candidate(LearningLayer::Procedural),
            &observed,
            &PromotionGate::default(),
        )
        .expect_err("a refusal");
        assert_eq!(refusal, PromotionRefused::EvaluationNotPassed);
    }

    #[test]
    fn the_same_candidate_and_outcomes_always_get_the_same_answer() {
        let observed = [
            outcome(3, 300, true),
            outcome(1, 100, true),
            outcome(2, 200, false),
        ];
        let first = evaluate_promotion(
            &candidate(LearningLayer::Behavioral),
            &observed,
            &evaluated(),
        );
        for _ in 0..8 {
            assert_eq!(
                evaluate_promotion(
                    &candidate(LearningLayer::Behavioral),
                    &observed,
                    &evaluated()
                ),
                first
            );
        }
    }

    #[test]
    fn every_layer_that_is_not_associative_is_reachable_by_earning_it() {
        // The refusal is about the associative layer specifically, not a blanket ban that would
        // make the whole gate unpassable and look like caution.
        let observed = [
            outcome(1, 100, true),
            outcome(2, 200, true),
            outcome(3, 300, true),
        ];
        for layer in [
            LearningLayer::Episodic,
            LearningLayer::Epistemic,
            LearningLayer::Behavioral,
            LearningLayer::Procedural,
            LearningLayer::Neural,
        ] {
            assert!(
                evaluate_promotion(&candidate(layer), &observed, &evaluated()).is_ok(),
                "{layer:?} could not be promoted even when it was earned"
            );
        }
    }
}
