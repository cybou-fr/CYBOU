// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The whole path an observation travels before someone reads it.
//!
//! ```text
//! perception → contribution → epistemic belief → redaction → projection a reader receives
//! ```
//!
//! Four boundaries, each tested on its own, and none of them holding the composition — the same
//! condition that hid three defects in the path a word travels. Here the thing that can be lost is
//! worse than a hedge: it is the value a person did not agree to publish. A sensitivity that
//! survives perception, survives the Journal envelope, survives belief derivation and is dropped at
//! the redaction step produces a page that looks exactly like a page with nothing to hide.
//!
//! The assertion that matters is not that the filter returned the right count. It is that **the
//! withheld value does not appear anywhere in the bytes a stranger receives** — serialised, whole,
//! searched as text. A test that checks the list length passes on a projection that leaked the
//! value in a field nobody thought to look at.

use cybou_epistemicd::EpistemicCore;
use cybou_perception::types::{ObservedValue, SystemObservation};
use cybou_protocol::disclosure::WithheldBecause;
use cybou_protocol::{Kind, canonical::CanonicalEnvelope};
use cybou_web_gateway::redact::{Ledger, Verdict, verdict};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

/// What an ordinary reader of a public surface is permitted.
const ORDINARY: u8 = 0;
/// What belongs to the person, and to nobody who merely arrived.
const THE_PERSONS: u8 = 1;

/// The value that must never reach a stranger, in any field, in any encoding.
const THE_SECRET: &str = "at-a-clinic";

fn at() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_787_000_000).expect("a fixed instant")
}

/// Turn a perception into the contribution `perceptiond` would submit.
fn contribution(observation: SystemObservation, sensitivity: u8) -> CanonicalEnvelope {
    let acquired = observation.acquired_at;
    let payload_value = observation
        .into_protocol()
        .expect("the frozen timestamp format applies");
    let mut payload = Vec::new();
    ciborium::into_writer(&payload_value, &mut payload).expect("an observation encodes");

    CanonicalEnvelope {
        schema_version: 4,
        message_id: Uuid::from_u128(u128::from(sensitivity) + 100),
        correlation_id: Uuid::from_u128(1),
        causation_id: Uuid::nil(),
        origin_organ: "perceptiond".to_owned(),
        origin_node: String::new(),
        kind: Kind::Observation as u16,
        wall_time_ms: acquired.unix_timestamp() * 1000,
        monotonic_time: 0,
        logical_clock: 1,
        confidence: 1.0,
        evidence: Vec::new(),
        payload,
        privacy: 1,
        capability_scope: String::new(),
        sealed: false,
        key_domain_id: Uuid::nil(),
        key_epoch: 0,
        retention_class: 2,
        retention_policy_version: 0,
        retain_until_ms: 0,
        sensitivity,
    }
}

fn observation(subject: &'static str, value: &str) -> SystemObservation {
    SystemObservation {
        source_id: "test",
        subject,
        value: ObservedValue::Text(value.to_owned()),
        acquired_at: at(),
        freshness_until: at() + Duration::hours(1),
        provenance: "a walkthrough".to_owned(),
    }
}

/// Perceive two things, one of them the person's, and derive beliefs from both.
fn mind_that_knows_something_private() -> EpistemicCore {
    let epistemic = EpistemicCore::new();
    epistemic.ingest_envelope(
        &contribution(observation("kernel.version", "6.12.0"), ORDINARY),
        1,
    );
    epistemic.ingest_envelope(
        &contribution(observation("whereabouts", THE_SECRET), THE_PERSONS),
        2,
    );
    epistemic
}

/// Everything a reader permitted `permitted` would be handed, as the text they would receive.
fn what_a_reader_receives(epistemic: &EpistemicCore, permitted: u8) -> (String, Ledger) {
    let ledger = Ledger::new();
    ledger.begin();
    let supplied: Vec<_> = epistemic
        .projection()
        .into_iter()
        .filter(|belief| {
            ledger.decide(
                belief.sensitivity,
                permitted,
                || Some(belief.subject.clone()),
                &belief.evidence,
            )
        })
        .collect();
    (
        serde_json::to_string(&supplied).expect("beliefs serialise"),
        ledger,
    )
}

#[test]
fn the_value_a_stranger_may_not_see_appears_nowhere_in_what_they_receive() {
    // The assertion the path exists for. Not "the list was the right length" — a projection that
    // leaked the value into a field nobody thought to check would pass that and fail this.
    let epistemic = mind_that_knows_something_private();
    let (received, _) = what_a_reader_receives(&epistemic, ORDINARY);

    assert!(
        !received.contains(THE_SECRET),
        "the value crossed the boundary: {received}"
    );
    assert!(
        received.contains("6.12.0"),
        "nothing crossed at all, so the test above proved nothing: {received}"
    );
}

#[test]
fn the_person_is_told_what_belongs_to_them() {
    // The control for the control. Without it, a redactor that dropped everything would satisfy
    // every assertion above, and the surface would be private and useless.
    let epistemic = mind_that_knows_something_private();
    let (received, ledger) = what_a_reader_receives(&epistemic, THE_PERSONS);

    assert!(received.contains(THE_SECRET));
    assert_eq!(ledger.delivered().item_count, 2);
    assert!(ledger.delivered().withheld.is_empty());
}

#[test]
fn a_sensitivity_set_at_perception_is_still_in_force_at_the_last_boundary() {
    // Four boundaries. The perception declared the class, the contribution carried it, the belief
    // inherited it, and the redactor acted on it. Any one of them resetting it to zero produces a
    // page that looks exactly like a page with nothing to hide.
    let epistemic = mind_that_knows_something_private();
    let private = epistemic
        .query("whereabouts")
        .expect("the belief was derived");
    assert_eq!(
        private.sensitivity, THE_PERSONS,
        "the class was lost between perception and belief"
    );
    assert_eq!(
        verdict(private.sensitivity, ORDINARY),
        Verdict::Withhold(WithheldBecause::AboveConsumerTrust)
    );
}

#[test]
fn a_stranger_is_told_that_something_was_kept_and_never_what_it_was() {
    // ADR-0030 B6 across the whole path: an item quietly dropped and an item that was never
    // relevant must not look identical. The record names the subject so the person can ask; the
    // route is what decides that a stranger sees the count and not the name.
    let epistemic = mind_that_knows_something_private();
    let (_, ledger) = what_a_reader_receives(&epistemic, ORDINARY);
    let delivered = ledger.delivered();

    assert_eq!(delivered.item_count, 1);
    assert_eq!(delivered.withheld.len(), 1);
    assert_eq!(
        delivered.withheld[0].because,
        WithheldBecause::AboveConsumerTrust
    );
    assert_eq!(
        delivered.withheld[0].subject.as_deref(),
        Some("whereabouts"),
        "the record cannot answer 'why was that kept from me?' without the subject"
    );
}

#[test]
fn what_is_accounted_for_never_exceeds_what_was_supplied() {
    // The one defect this surface has actually shipped, walked rather than unit-tested: ten items
    // supplied and three thousand reported as accounted for, because a set of source contributions
    // was read as a count of items.
    let epistemic = mind_that_knows_something_private();
    for permitted in [ORDINARY, THE_PERSONS] {
        let (_, ledger) = what_a_reader_receives(&epistemic, permitted);
        let delivered = ledger.delivered();
        assert!(
            delivered.accounted_for <= delivered.item_count,
            "accounted for {} of {} supplied",
            delivered.accounted_for,
            delivered.item_count
        );
    }
}

#[test]
fn a_belief_the_person_later_disputes_is_still_theirs_alone() {
    // A second observation contradicting the first makes the belief `Disputed`. What must not
    // happen is the dispute resetting the class along with the value — a contested private fact is
    // still a private fact, and this is the path where "it changed, so re-derive it" loses things.
    let epistemic = mind_that_knows_something_private();
    epistemic.ingest_envelope(
        &contribution(observation("whereabouts", "somewhere-else"), ORDINARY),
        3,
    );

    let private = epistemic.query("whereabouts").expect("still derived");
    assert_eq!(
        private.sensitivity, THE_PERSONS,
        "a contradicting observation downgraded the class"
    );

    let (received, _) = what_a_reader_receives(&epistemic, ORDINARY);
    assert!(!received.contains(THE_SECRET), "{received}");
    assert!(!received.contains("somewhere-else"), "{received}");
}

#[test]
fn nothing_perceived_produces_nothing_supplied_rather_than_nothing_withheld() {
    // An empty Mind and a fully redacted one must not produce the same account. One supplied
    // nothing because there was nothing; the other supplied nothing because everything was kept.
    let empty = EpistemicCore::new();
    let (_, empty_ledger) = what_a_reader_receives(&empty, ORDINARY);

    let epistemic = EpistemicCore::new();
    epistemic.ingest_envelope(
        &contribution(observation("whereabouts", THE_SECRET), THE_PERSONS),
        1,
    );
    let (_, redacted_ledger) = what_a_reader_receives(&epistemic, ORDINARY);

    assert_eq!(empty_ledger.delivered().item_count, 0);
    assert_eq!(redacted_ledger.delivered().item_count, 0);
    assert!(empty_ledger.delivered().withheld.is_empty());
    assert_eq!(redacted_ledger.delivered().withheld.len(), 1);
}
