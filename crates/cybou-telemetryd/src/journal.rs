// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Putting a finding in the Journal, together with exactly the readings it rests on.
//!
//! Nothing here decides what a finding is. It decides how one enters the Journal, and the whole of
//! that decision is already stated on [`SystemInsight`]: a finding is a `Hypothesis` and not an
//! `Observation`, because what was *observed* is the readings, and that they add up to *the database
//! stopped because the disk filled* is an inference. An inference recorded as an observation is a
//! claim the host cannot support.
//!
//! ## Which readings, and why not all of them
//!
//! ```text
//! every reading the host takes   transient, and not biography
//! the readings a finding cites   why it thinks so, and worth keeping
//! ```
//!
//! Writing the whole stream in would turn the Journal into a metrics database, which is the thing
//! this system has said all along it is not. Writing none of them would leave a finding with nothing
//! to point at — and the Journal will not take a `Hypothesis` that cites nothing, correctly, because
//! an inference nobody can trace back is indistinguishable from one a model made up.
//!
//! So the readings a finding names in `because` go in as `Observation`s, and the finding cites them
//! as its evidence. `why do you think that` is then answerable by following the record rather than
//! by trusting the sentence.
//!
//! ## Why this exists at all
//!
//! Downstream of it, `Action1` records a proposal citing the finding that gave rise to it. Until the
//! finding is in the Journal that citation points at nothing, and the whole authorization lifecycle
//! is refused — which is what was happening, silently, on every host.

use cybou_protocol::admission::Kind;
use cybou_protocol::canonical::CanonicalEnvelope;
use cybou_protocol::telemetry::{InsightEvidence, SystemInsight};
use time::OffsetDateTime;
use uuid::{Uuid, uuid};

/// The organ these contributions come from.
const ORIGIN: &str = "telemetryd";

/// Schema of the envelopes this module writes.
const SCHEMA: u16 = 3;

/// Namespace for reading identities, so the same reading of the same finding is the same row.
const READING_NAMESPACE: Uuid = uuid!("6f4b1f5a-0d3e-4c2a-9f77-2b1c8e5a4d90");

/// Why a finding cannot be written down.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CannotRecord {
    /// The finding names no readings.
    ///
    /// A `Hypothesis` must cite something that exists, and a finding that cited nothing would be an
    /// inference nobody can trace back — which is exactly what the Journal declines to hold, and
    /// exactly what it should decline. Not a failure to work around: a finding worth recording is one
    /// that can say why.
    NoReadings(Uuid),
}

impl core::fmt::Display for CannotRecord {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoReadings(id) => {
                write!(
                    formatter,
                    "finding {id} cites no readings and cannot be traced back"
                )
            }
        }
    }
}

impl core::error::Error for CannotRecord {}

/// The identity one cited reading has in the Journal.
///
/// Derived rather than generated, so a finding re-concluded from the same readings produces the same
/// rows. The alternative is a Journal that grows a fresh copy of the same evidence every time a host
/// notices the same problem again.
#[must_use]
pub fn reading_id(insight: &SystemInsight, reading: &InsightEvidence) -> Uuid {
    let seed = format!(
        "{}|{}|{}",
        insight.insight_id,
        reading.key.label(),
        reading.observed
    );
    Uuid::new_v5(&READING_NAMESPACE, seed.as_bytes())
}

/// One finding and the readings it rests on, in the order they must be written.
///
/// Readings first: the finding cites them, and a contribution may only cite something already there.
///
/// # Errors
///
/// Returns [`CannotRecord::NoReadings`] for a finding that names none.
///
/// # Panics
///
/// Never: encoding these types to CBOR cannot fail, and the expectation is written out rather than
/// swallowed so a future field that could fail is caught by a test.
pub fn contributions(
    insight: &SystemInsight,
    now: OffsetDateTime,
) -> Result<Vec<CanonicalEnvelope>, CannotRecord> {
    if insight.because.is_empty() {
        return Err(CannotRecord::NoReadings(insight.insight_id));
    }

    let mut out = Vec::with_capacity(insight.because.len() + 1);
    let mut evidence = Vec::with_capacity(insight.because.len());

    for reading in &insight.because {
        let id = reading_id(insight, reading);
        evidence.push(id);
        // An Observation, and one of the two kinds that may cite nothing: a reading is something the
        // host measured outside the Journal, which is exactly what that kind is for.
        out.push(envelope(
            id,
            insight.insight_id,
            Uuid::nil(),
            Vec::new(),
            Kind::Observation,
            &payload(reading),
            now,
        ));
    }

    // The finding itself: an inference, citing what it was inferred from.
    out.push(envelope(
        insight.insight_id,
        insight.insight_id,
        Uuid::nil(),
        evidence,
        Kind::Hypothesis,
        &payload(insight),
        now,
    ));
    Ok(out)
}

fn payload<T: serde::Serialize>(value: &T) -> Vec<u8> {
    let mut out = Vec::new();
    ciborium::into_writer(value, &mut out).expect("a telemetry payload encodes");
    out
}

fn envelope(
    message_id: Uuid,
    correlation_id: Uuid,
    causation_id: Uuid,
    evidence: Vec<Uuid>,
    kind: Kind,
    payload: &[u8],
    now: OffsetDateTime,
) -> CanonicalEnvelope {
    CanonicalEnvelope {
        schema_version: SCHEMA,
        message_id,
        correlation_id,
        causation_id,
        origin_organ: ORIGIN.to_owned(),
        origin_node: String::new(),
        kind: kind as u16,
        wall_time_ms: i64::try_from(now.unix_timestamp_nanos() / 1_000_000).unwrap_or(i64::MAX),
        monotonic_time: 0,
        logical_clock: 1,
        confidence: 1.0,
        evidence,
        payload: payload.to_vec(),
        // Node: what this host measured about itself is about this host.
        privacy: 1,
        capability_scope: String::new(),
        sealed: false,
        key_domain_id: Uuid::nil(),
        key_epoch: 0,
        retention_class: 2,
        retention_policy_version: 0,
        retain_until_ms: 0,
        // Operational rather than personal. A disk filling up is not a fact about anybody.
        sensitivity: 0,
    }
}

#[cfg(test)]
mod tests {
    use cybou_protocol::telemetry::{
        Deviation, EvidenceStrength, Finding, MetricKey, Subject, SystemInsight,
    };

    use super::*;

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    fn reading(observed: f64) -> InsightEvidence {
        InsightEvidence {
            key: MetricKey::host(Subject::RootFilesystemUsed),
            observed,
            deviation: None,
        }
    }

    fn finding(readings: Vec<InsightEvidence>) -> SystemInsight {
        SystemInsight {
            insight_id: SystemInsight::derive_id(Finding::StorageExhaustion, None, at(0)),
            finding: Finding::StorageExhaustion,
            about: None,
            because: readings,
            strength: EvidenceStrength::Strong,
            concluded_at: at(0),
            since: at(0),
        }
    }

    #[test]
    fn a_finding_is_an_inference_and_its_readings_are_what_was_observed() {
        // The distinction the protocol already states. Recording the inference as an observation
        // would be the host claiming it saw something it worked out.
        let written = contributions(&finding(vec![reading(97.0)]), at(1)).expect("it cites one");

        assert_eq!(written.len(), 2);
        assert_eq!(written[0].kind, Kind::Observation as u16);
        assert_eq!(written[1].kind, Kind::Hypothesis as u16);
    }

    #[test]
    fn the_readings_come_first_because_the_finding_cites_them() {
        // A contribution may only cite something already there, so the order is not cosmetic.
        let insight = finding(vec![reading(97.0), reading(98.0)]);
        let written = contributions(&insight, at(1)).expect("it cites two");

        let ids: Vec<Uuid> = written[..2].iter().map(|e| e.message_id).collect();
        let cited = &written[2].evidence;
        assert_eq!(cited, &ids);
        assert_eq!(written[2].message_id, insight.insight_id);
    }

    #[test]
    fn only_the_readings_a_finding_cites_are_written_down() {
        // The whole stream is transient and is not biography. What a finding rests on is why it
        // thinks so, and that is worth keeping.
        let one = contributions(&finding(vec![reading(97.0)]), at(1)).expect("one");
        let three = contributions(
            &finding(vec![reading(97.0), reading(98.0), reading(99.0)]),
            at(1),
        )
        .expect("three");

        assert_eq!(one.len(), 2);
        assert_eq!(three.len(), 4);
    }

    #[test]
    fn the_same_finding_from_the_same_readings_is_the_same_rows() {
        // Derived rather than generated, so a host that notices the same problem again does not grow
        // a fresh copy of the same evidence each time.
        let insight = finding(vec![reading(97.0)]);
        let first = contributions(&insight, at(1)).expect("written");
        let second = contributions(&insight, at(600)).expect("written again");

        let ids = |written: &[CanonicalEnvelope]| -> Vec<Uuid> {
            written.iter().map(|e| e.message_id).collect()
        };
        assert_eq!(ids(&first), ids(&second));
    }

    #[test]
    fn readings_that_differ_are_different_rows() {
        let one = contributions(&finding(vec![reading(97.0)]), at(1)).expect("written");
        let other = contributions(&finding(vec![reading(98.0)]), at(1)).expect("written");

        assert_ne!(one[0].message_id, other[0].message_id);
        assert_eq!(
            one[1].message_id, other[1].message_id,
            "the same finding keeps its identity; the readings under it do not"
        );
    }

    #[test]
    fn a_finding_that_cites_nothing_is_refused_rather_than_written() {
        // The Journal would refuse it, and rightly: an inference nobody can trace back is
        // indistinguishable from one a model made up. Refusing here says so, instead of submitting
        // something certain to be rejected and calling the result best effort.
        let uncited = finding(Vec::new());
        assert_eq!(
            contributions(&uncited, at(1)),
            Err(CannotRecord::NoReadings(uncited.insight_id))
        );
    }

    #[test]
    fn a_reading_and_a_finding_belong_to_one_episode() {
        // So a reader following a finding arrives at what it rests on without knowing which organ
        // wrote either.
        let insight = finding(vec![reading(97.0), reading(98.0)]);
        let written = contributions(&insight, at(1)).expect("written");

        assert!(
            written
                .iter()
                .all(|envelope| envelope.correlation_id == insight.insight_id)
        );
        assert!(
            written
                .iter()
                .all(|envelope| envelope.causation_id.is_nil()),
            "nothing here follows from an earlier contribution; the finding rests on evidence"
        );
    }

    #[test]
    fn a_deviation_travels_with_the_reading_it_belongs_to() {
        // The reading is kept whole. A record of "97" without "and ordinary here is 40" is a number
        // somebody has to go and interpret again later.
        let mut with = reading(97.0);
        with.deviation = Some(Deviation {
            ordinary: 40.0,
            spread: 3.0,
            observed: 97.0,
            spreads_away: 19.0,
        });
        let written = contributions(&finding(vec![with.clone()]), at(1)).expect("written");

        let decoded: InsightEvidence =
            ciborium::from_reader(written[0].payload.as_slice()).expect("decodes");
        assert_eq!(decoded, with);
    }
}
