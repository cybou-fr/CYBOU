// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Joining plans, without letting the join lose what either of them said.
//!
//! One question often needs two answers, and the two are built by different parts of Mind. The
//! temptation is to concatenate the prose, which is where the loss happens: two sentences run
//! together keep the claims and quietly shed the hedges, because a hedge reads as belonging to the
//! sentence beside it rather than to the whole.
//!
//! Composition happens here instead, on plans, where the hedges are typed. The rule that matters is
//! one line long: **a qualification on any part qualifies the whole.** Joining an answer that was
//! read from a stale projection with one that was not produces an answer that is stale, because a
//! reader cannot tell which half a hedge applied to and should not have to.
//!
//! A composition that cannot be made honestly is refused rather than fudged. Two plans with
//! different intents are not one plan: saying "status" over claims made to confirm an action would
//! reframe them, and reframing is the thing a plan boundary exists to prevent.

use cybou_protocol::meaning::{Qualification, ResponsePlan};
use uuid::Uuid;

/// Why two plans could not be made into one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositionRefused {
    /// The parts were built for different communicative intents.
    ///
    /// Not an error to work around. An answer composed of claims made under two intents has no
    /// single intent, and picking one of them would present half the claims under a purpose their
    /// author did not have.
    DifferentIntents {
        /// The intent the composition started with.
        first: String,
        /// The intent that did not match it.
        second: String,
    },
    /// There was nothing to compose.
    ///
    /// An empty composition is not an empty answer: it is the absence of one, and returning a plan
    /// that says nothing would let a caller present "no parts" as "nothing to report".
    NothingToCompose,
}

impl core::fmt::Display for CompositionRefused {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DifferentIntents { first, second } => write!(
                formatter,
                "plans for '{first}' and '{second}' are not one plan"
            ),
            Self::NothingToCompose => write!(formatter, "there were no plans to compose"),
        }
    }
}

impl core::error::Error for CompositionRefused {}

/// Join plans that share an intent into one.
///
/// Key points keep their order, because the order a plan puts its claims in is part of what it
/// says. Evidence and qualifications are unions: a contribution cited by two parts is one
/// contribution, and a hedge raised by one part holds for the result.
///
/// # Errors
///
/// Returns [`CompositionRefused`] when the parts do not share an intent, or when there are none.
pub fn compose(parts: &[ResponsePlan], plan_id: Uuid) -> Result<ResponsePlan, CompositionRefused> {
    let Some(first) = parts.first() else {
        return Err(CompositionRefused::NothingToCompose);
    };
    if let Some(mismatch) = parts.iter().find(|part| part.intent != first.intent) {
        return Err(CompositionRefused::DifferentIntents {
            first: first.intent.clone(),
            second: mismatch.intent.clone(),
        });
    }

    let mut key_points: Vec<String> = Vec::new();
    let mut referenced_evidence: Vec<Uuid> = Vec::new();
    let mut qualifications: Vec<Qualification> = Vec::new();

    for part in parts {
        for point in &part.key_points {
            // Identical text is one claim. Two organs reporting the same fact should say it once;
            // saying it twice would read as two findings that happen to agree.
            if !key_points.contains(point) {
                key_points.push(point.clone());
            }
        }
        for evidence in &part.referenced_evidence {
            if !referenced_evidence.contains(evidence) {
                referenced_evidence.push(*evidence);
            }
        }
        for qualification in &part.qualifications {
            if !qualifications.contains(qualification) {
                qualifications.push(*qualification);
            }
        }
    }

    Ok(ResponsePlan {
        plan_id,
        intent: first.intent.clone(),
        key_points,
        referenced_evidence,
        qualifications,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(intent: &str, point: &str) -> ResponsePlan {
        ResponsePlan {
            plan_id: Uuid::from_u128(1),
            intent: intent.to_owned(),
            key_points: vec![point.to_owned()],
            referenced_evidence: Vec::new(),
            qualifications: Vec::new(),
        }
    }

    #[test]
    fn a_hedge_on_one_part_holds_for_the_whole() {
        // The rule the module exists for. A reader cannot tell which half a hedge applied to, so
        // the answer as a whole carries it.
        let mut stale = plan("inform_status", "The disk is fine.");
        stale.qualifications = vec![Qualification::Stale];
        let confident = plan("inform_status", "Two commitments are open.");

        let joined = compose(&[stale, confident], Uuid::from_u128(9)).expect("one intent");
        assert_eq!(joined.qualifications, vec![Qualification::Stale]);
    }

    #[test]
    fn plans_for_different_intents_are_not_one_plan() {
        // Picking an intent would present half the claims under a purpose their author did not
        // have.
        let status = plan("inform_status", "All is available.");
        let confirm = plan("confirm_action", "This will delete the record.");

        let refusal = compose(&[status, confirm], Uuid::from_u128(9)).expect_err("a refusal");
        assert_eq!(
            refusal,
            CompositionRefused::DifferentIntents {
                first: "inform_status".to_owned(),
                second: "confirm_action".to_owned(),
            }
        );
    }

    #[test]
    fn nothing_to_compose_is_refused_rather_than_answered_emptily() {
        // An empty composition is the absence of an answer, not an answer that says nothing.
        let refusal = compose(&[], Uuid::from_u128(9)).expect_err("a refusal");
        assert_eq!(refusal, CompositionRefused::NothingToCompose);
    }

    #[test]
    fn the_order_a_plan_puts_its_claims_in_survives() {
        let first = plan("inform_status", "First.");
        let second = plan("inform_status", "Second.");
        let joined = compose(&[first, second], Uuid::from_u128(9)).expect("one intent");
        assert_eq!(joined.key_points, vec!["First.", "Second."]);
    }

    #[test]
    fn the_same_fact_from_two_parts_is_stated_once() {
        let a = plan("inform_status", "The chain verifies.");
        let b = plan("inform_status", "The chain verifies.");
        let joined = compose(&[a, b], Uuid::from_u128(9)).expect("one intent");
        assert_eq!(joined.key_points.len(), 1);
    }

    #[test]
    fn evidence_is_a_union_and_a_contribution_is_cited_once() {
        let shared = Uuid::from_u128(42);
        let mut a = plan("inform_status", "A.");
        a.referenced_evidence = vec![shared, Uuid::from_u128(1)];
        let mut b = plan("inform_status", "B.");
        b.referenced_evidence = vec![shared, Uuid::from_u128(2)];

        let joined = compose(&[a, b], Uuid::from_u128(9)).expect("one intent");
        assert_eq!(
            joined.referenced_evidence,
            vec![shared, Uuid::from_u128(1), Uuid::from_u128(2)]
        );
    }

    #[test]
    fn every_distinct_hedge_survives_the_join() {
        let mut a = plan("inform_status", "A.");
        a.qualifications = vec![Qualification::Stale, Qualification::Withheld];
        let mut b = plan("inform_status", "B.");
        b.qualifications = vec![Qualification::Withheld, Qualification::NotRead];

        let joined = compose(&[a, b], Uuid::from_u128(9)).expect("one intent");
        for expected in [
            Qualification::Stale,
            Qualification::Withheld,
            Qualification::NotRead,
        ] {
            assert!(joined.qualifications.contains(&expected), "{expected:?}");
        }
        assert_eq!(joined.qualifications.len(), 3, "a hedge was stated twice");
    }

    #[test]
    fn composing_one_plan_changes_nothing_but_its_identity() {
        let mut only = plan("inform_status", "Alone.");
        only.qualifications = vec![Qualification::Partial];
        only.referenced_evidence = vec![Uuid::from_u128(5)];

        let joined = compose(std::slice::from_ref(&only), Uuid::from_u128(9)).expect("one intent");
        assert_eq!(joined.key_points, only.key_points);
        assert_eq!(joined.qualifications, only.qualifications);
        assert_eq!(joined.referenced_evidence, only.referenced_evidence);
        assert_eq!(joined.intent, only.intent);
        assert_eq!(joined.plan_id, Uuid::from_u128(9));
    }
}
