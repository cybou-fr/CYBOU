// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The owner of `org.cybou.Mind.Meaning1`.
//!
//! Interpretation is derived state, not truth, and ADR-0031 says so in those words. That
//! distinction is what decides how it enters the biography: what the person said is an
//! `Observation`, because it happened outside the Journal, and what this organ took it to mean is
//! a `Hypothesis` caused by that observation. Nothing here needed a new contribution kind — a
//! meaning layer that had to widen the frozen vocabulary to fit would have been a sign it was
//! claiming something the rest of Mind has no way to reason about.
//!
//! Recording both is also what makes an act inspectable after the interpreter that produced it is
//! stopped or replaced: the act is in the Journal, next to the sentence it came from.

use cybou_protocol::{
    Kind,
    canonical::CanonicalEnvelope,
    meaning::{CognitiveAct, MeaningInterpretation},
    observation::ObservationV1,
    unix_millis,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

#[cfg(target_os = "linux")]
pub mod service;

/// The two contributions one interpretation produces, in the order they must be submitted.
///
/// They are built together because the second cites the first: a `Hypothesis` is a derived kind
/// and has no standing without the contribution it came from. Building the pair as one value keeps
/// a caller from submitting an interpretation whose utterance never reached the Journal.
pub struct InterpretedContributions {
    /// What the person said, as a root observation.
    pub utterance: CanonicalEnvelope,
    /// What this organ took it to mean, caused by the utterance.
    pub interpretation: CanonicalEnvelope,
}

/// Build the pair of contributions that record an utterance and its interpretation.
///
/// `supersedes` names an earlier interpretation this one corrects. It becomes evidence rather than
/// cause: the cause of an interpretation is always the sentence it interprets, and a correction
/// that pointed at the interpretation it replaced would lose the sentence. The earlier one stays
/// in the Journal untouched, which is the whole of ADR-0031's rule that corrections append.
#[must_use]
pub fn contributions_for(
    interpreted: &MeaningInterpretation,
    now: OffsetDateTime,
    supersedes: Option<Uuid>,
) -> Option<InterpretedContributions> {
    let instant = now.format(&Rfc3339).ok()?;
    let spoken = ObservationV1 {
        source_id: interpreted.primary_act.source.clone(),
        subject: "utterance".to_owned(),
        value: ciborium::Value::Text(interpreted.utterance.clone()),
        acquired_at: instant.clone(),
        // What was said was true of the moment it was said and vouches for nothing after it.
        freshness_until: instant,
        provenance: "spoken to Meaning1".to_owned(),
    };
    let mut spoken_payload = Vec::new();
    ciborium::into_writer(&spoken, &mut spoken_payload).ok()?;

    let mut interpretation_payload = Vec::new();
    ciborium::into_writer(interpreted, &mut interpretation_payload).ok()?;

    let utterance_id = Uuid::new_v4();
    // One episode: the sentence and the reading of it happened together, and sharing a correlation
    // identity is what lets anything downstream see that they belong to each other.
    let episode = Uuid::new_v4();

    let utterance = envelope(
        utterance_id,
        episode,
        Uuid::nil(),
        Kind::Observation,
        spoken_payload,
        1.0,
        Vec::new(),
        now,
    );

    let interpretation = envelope(
        interpreted.primary_act.act_id,
        episode,
        utterance_id,
        Kind::Hypothesis,
        interpretation_payload,
        interpreted.confidence,
        supersedes.into_iter().collect(),
        now,
    );

    Some(InterpretedContributions {
        utterance,
        interpretation,
    })
}

/// The act carried by an interpretation contribution, read back from its payload.
///
/// This is what C4 comes down to: an act is inspectable because it is a row, and reading it needs
/// nothing but the Journal and this function. The interpreter that produced it may be gone.
#[must_use]
pub fn act_in(envelope: &CanonicalEnvelope) -> Option<CognitiveAct> {
    if Kind::from_u16(envelope.kind) != Some(Kind::Hypothesis) {
        return None;
    }
    let interpreted: MeaningInterpretation =
        ciborium::from_reader(envelope.payload.as_slice()).ok()?;
    Some(interpreted.primary_act)
}

#[allow(
    clippy::too_many_arguments,
    reason = "an envelope is a wide value and naming its parts is the point"
)]
fn envelope(
    message_id: Uuid,
    correlation_id: Uuid,
    causation_id: Uuid,
    kind: Kind,
    payload: Vec<u8>,
    confidence: f64,
    evidence: Vec<Uuid>,
    now: OffsetDateTime,
) -> CanonicalEnvelope {
    CanonicalEnvelope {
        schema_version: 4,
        message_id,
        correlation_id,
        causation_id,
        origin_organ: "meaningd".to_owned(),
        origin_node: String::new(),
        kind: kind as u16,
        wall_time_ms: unix_millis(now),
        monotonic_time: 0,
        logical_clock: 1,
        confidence,
        evidence,
        payload,
        // What a person said to their own machine does not leave the node.
        privacy: 1,
        capability_scope: String::new(),
        sealed: false,
        key_domain_id: Uuid::nil(),
        key_epoch: 0,
        retention_class: 2,
        retention_policy_version: 0,
        retain_until_ms: 0,
        // A person's own words are about the person, whatever they happen to be about otherwise.
        // Stamping this Ordinary would put everything anyone ever says onto the public surface.
        sensitivity: 1,
    }
}

#[cfg(test)]
mod tests {
    use cybou_meaning::interpret;

    use super::*;

    fn at() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000).expect("a fixed instant")
    }

    #[test]
    fn an_interpretation_cites_the_sentence_it_interprets() {
        let interpreted = interpret("Verify the chain", "person", at()).expect("an opening");
        let pair = contributions_for(&interpreted, at(), None).expect("two contributions");

        assert_eq!(pair.utterance.kind, Kind::Observation as u16);
        assert_eq!(pair.interpretation.kind, Kind::Hypothesis as u16);
        // A Hypothesis is a derived kind and the Journal refuses one that cites nothing.
        assert_eq!(pair.interpretation.causation_id, pair.utterance.message_id);
        assert_eq!(
            pair.interpretation.correlation_id,
            pair.utterance.correlation_id
        );
    }

    #[test]
    fn what_a_person_said_is_never_stamped_ordinary() {
        let interpreted =
            interpret("Remember that the backup runs at nine", "person", at()).expect("an opening");
        let pair = contributions_for(&interpreted, at(), None).expect("two contributions");
        assert_eq!(pair.utterance.sensitivity, 1);
        assert_eq!(pair.interpretation.sensitivity, 1);
        assert_eq!(pair.utterance.privacy, 1);
    }

    #[test]
    fn a_correction_appends_and_names_what_it_supersedes() {
        // C3: the earlier interpretation is cited, not replaced, and the correction is still
        // caused by its own sentence rather than by the reading it disagrees with.
        let first = interpret("Verify the chain", "person", at()).expect("an opening");
        let first_pair = contributions_for(&first, at(), None).expect("two contributions");

        let correction = interpret("No, the disk was fine", "person", at()).expect("an opening");
        let corrected = contributions_for(&correction, at(), Some(first.primary_act.act_id))
            .expect("two contributions");

        assert_eq!(
            corrected.interpretation.evidence,
            vec![first.primary_act.act_id]
        );
        assert_eq!(
            corrected.interpretation.causation_id,
            corrected.utterance.message_id
        );
        // Nothing about the first pair changed: it is still exactly what was recorded.
        assert_eq!(
            first_pair.interpretation.message_id,
            first.primary_act.act_id
        );
        assert!(first_pair.interpretation.evidence.is_empty());
    }

    #[test]
    fn an_act_can_be_read_back_from_the_contribution_alone() {
        // C4: no interpreter, no dialogue state, no organ — just the row.
        let interpreted =
            interpret("Compare the last two sessions", "person", at()).expect("an opening");
        let pair = contributions_for(&interpreted, at(), None).expect("two contributions");

        let recovered = act_in(&pair.interpretation).expect("the act is in the payload");
        assert_eq!(recovered.act_id, interpreted.primary_act.act_id);
        assert_eq!(recovered.kind, interpreted.primary_act.kind);
        assert_eq!(recovered.subject, interpreted.primary_act.subject);

        // The utterance row is not an act, and must not be read as one.
        assert!(act_in(&pair.utterance).is_none());
    }
}
