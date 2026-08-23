// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What Mind supplied to a named consumer, and what it held back (ADR-0030).
//!
//! The biography could say what the system did and not who looked at it, which made it traceable in
//! one direction only. These are the types that close that: a record naming the destination, the
//! contributions the delivered items came from, and — the part that decides whether the surface is
//! honest — what was withheld and why.
//!
//! Nothing here carries content. A disclosure record is permanent, so a copy of what was disclosed
//! would put the thing being protected into the one place erasure cannot reach.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The most source contributions one disclosure record carries.
///
/// A delivery can cite thousands. Recording all of them put a set that grows with the biography
/// into a permanent contribution on every delivery, and searching it linearly made a wide delivery
/// quadratic in what it cited. The bound is generous enough that ordinary deliveries are recorded
/// whole, and `provenance_count` says when one was not.
pub const MAX_RECORDED_PROVENANCE: usize = 256;

/// How much of a person's context a consumer may see.
///
/// Locality is not trust. ADR-0030's amendment says so in as many words: once every consequential
/// consumer is local, a policy that only filters remote ones filters nothing that matters. A
/// consumer gains context by being permitted, never by being nearby.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConsumerTrust {
    /// May see only what is ordinary: a stranger reading a public surface.
    Public,
    /// May see what belongs to the person, having established who they are.
    Owner,
}

/// Who received something, and what receiving it means.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    /// Stable name of the consumer, as the consumer is configured rather than as it asks to be called.
    pub id: String,
    /// How much of the person's context this consumer may see.
    pub trust: ConsumerTrust,
    /// Whether what it receives outlives the request.
    ///
    /// The durable record follows this rather than distance. A local model that adapts on what it
    /// received has written it into parameters nobody can surgically unlearn, and under erasure the
    /// delivery record is the only evidence of how that contamination travelled.
    pub retains: bool,
    /// Whether delivery crosses a network or trust boundary.
    ///
    /// Recorded regardless of retention, because irreversibility is its own reason.
    pub external_boundary: bool,
}

impl Destination {
    /// Whether supplying this consumer has to be recorded in the Journal.
    ///
    /// An inspector that renders and forgets needs no contribution. A consumer that keeps what it
    /// was given, or one that is on the other side of a boundary, does.
    #[must_use]
    pub const fn needs_a_record(&self) -> bool {
        self.retains || self.external_boundary
    }
}

/// Why an item was not supplied.
///
/// A closed set. An item quietly dropped for policy reasons and an item that was never relevant
/// look identical unless the interface insists on the difference, and after a language faculty
/// exists nobody would be able to tell which one they were looking at.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WithheldBecause {
    /// More exposing than this consumer's trust permits.
    AboveConsumerTrust,
    /// About the person by construction, whatever its class says.
    BelongsToThePerson,
}

/// One thing that was not supplied, named without being restated.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Withheld {
    /// What the item was about, which is a subject and never a value.
    ///
    /// A subject is the least that still lets a person ask "why was that held back?", and the most
    /// that can be said without the record becoming a copy of what it protects. Where even the
    /// subject would say too much, a consumer sees a count and no subject at all.
    pub subject: Option<String>,
    /// Why it was held back.
    pub because: WithheldBecause,
}

/// A record that context was supplied to a consumer.
///
/// Kind 17 in the frozen vocabulary, and a root kind: a disclosure happened outside the Journal, so
/// it cites nothing and is caused by nothing.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextDisclosedV1 {
    /// Who received it.
    pub destination: Destination,
    /// The distinct contributions the supplied items were derived from, up to
    /// [`MAX_RECORDED_PROVENANCE`].
    ///
    /// Provenance rather than content: what was disclosed can be reconstructed from the Journal by
    /// anyone entitled to read it, and cannot be read out of this record by anyone who is not.
    ///
    /// A set of sources, and not on the same scale as `item_count`: one supplied belief can cite
    /// hundreds of contributions, so this is routinely far the longer of the two. Until 2026-08-22
    /// the paragraph below claimed the opposite relationship, and a surface built on that reading
    /// reported ten items supplied and three thousand accounted for on a live deployment.
    ///
    /// Bounded since 2026-08-23. Unbounded, it grew with the biography and put a set that large
    /// into a permanent record on every delivery. Where it is a sample rather than the whole set,
    /// `provenance_count` is larger than its length — the record says so itself rather than
    /// needing a flag, and a permanent record that silently omitted would be worse than a large
    /// one.
    pub items: Vec<Uuid>,
    /// How many distinct contributions there were, whether or not `items` could carry them all.
    ///
    /// `None` on a record written before this field existed. Not zero: a record that cannot say
    /// how many sources there were and a record saying there were none are different facts, and
    /// defaulting to zero would turn the first into the second everywhere it was read.
    #[serde(default)]
    pub provenance_count: Option<u32>,
    /// How many items were supplied, including any whose provenance could not be established.
    ///
    /// Neither bounds the other, and the length of `items` is not the count this should be read
    /// against. What answers "supplied five, can account for four" is a count of items that named
    /// at least one contribution, which this record does not carry; the gateway keeps it for the
    /// surface that shows it.
    pub item_count: u32,
    /// What was held back, and why.
    pub withheld: Vec<Withheld>,
    /// UTC RFC 3339 instant the delivery happened.
    pub disclosed_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn destination(retains: bool, external: bool) -> Destination {
        Destination {
            id: "living-canvas:public".into(),
            trust: ConsumerTrust::Public,
            retains,
            external_boundary: external,
        }
    }

    #[test]
    fn a_consumer_that_renders_and_forgets_locally_needs_no_record() {
        assert!(!destination(false, false).needs_a_record());
    }

    #[test]
    fn keeping_what_it_was_given_is_what_makes_a_record_necessary() {
        assert!(destination(true, false).needs_a_record());
    }

    #[test]
    fn crossing_a_boundary_is_recorded_whether_or_not_anything_keeps_it() {
        // Irreversibility is its own reason: once it has left, no later decision can call it back.
        assert!(destination(false, true).needs_a_record());
    }

    #[test]
    fn a_record_written_before_this_field_existed_says_it_cannot_say() {
        // Records already in Journals do not have `provenanceCount`. Decoding one must not report
        // that it cited nothing: a record that cannot say how many sources there were and a record
        // saying there were none are different facts, and a zero default would turn every existing
        // record into the second one on the day this field shipped.
        //
        // The fixture is built as a map without the field rather than by re-encoding the struct,
        // because re-encoding could only ever produce whatever this build writes today.
        let mut older = Vec::new();
        ciborium::into_writer(
            &ciborium::Value::Map(vec![
                (
                    ciborium::Value::Text("destination".into()),
                    ciborium::Value::Map(vec![
                        (
                            ciborium::Value::Text("id".into()),
                            ciborium::Value::Text("living-canvas:public".into()),
                        ),
                        (
                            ciborium::Value::Text("trust".into()),
                            ciborium::Value::Text("public".into()),
                        ),
                        (
                            ciborium::Value::Text("retains".into()),
                            ciborium::Value::Bool(false),
                        ),
                        (
                            ciborium::Value::Text("externalBoundary".into()),
                            ciborium::Value::Bool(true),
                        ),
                    ]),
                ),
                (
                    ciborium::Value::Text("items".into()),
                    ciborium::Value::Array(Vec::new()),
                ),
                (
                    ciborium::Value::Text("itemCount".into()),
                    ciborium::Value::Integer(7.into()),
                ),
                (
                    ciborium::Value::Text("withheld".into()),
                    ciborium::Value::Array(Vec::new()),
                ),
                (
                    ciborium::Value::Text("disclosedAt".into()),
                    ciborium::Value::Text("2026-08-21T00:00:00Z".into()),
                ),
            ]),
            &mut older,
        )
        .expect("the older shape encodes");

        let decoded: ContextDisclosedV1 =
            ciborium::from_reader(older.as_slice()).expect("an older record still decodes");
        assert_eq!(decoded.item_count, 7);
        assert_eq!(
            decoded.provenance_count, None,
            "an older record was read as having cited nothing"
        );
    }

    #[test]
    fn a_record_that_carried_every_source_it_cited_says_a_number_matching_its_own_list() {
        // The two readings a consumer has to be able to tell apart, side by side: a complete record
        // where the count equals the length, and a sampled one where it exceeds it.
        let complete = ContextDisclosedV1 {
            provenance_count: Some(2),
            destination: destination(false, true),
            items: vec![Uuid::from_u128(1), Uuid::from_u128(2)],
            item_count: 1,
            withheld: Vec::new(),
            disclosed_at: "2026-08-23T00:00:00Z".into(),
        };
        assert_eq!(complete.provenance_count, Some(2));
        assert_eq!(complete.items.len(), 2);

        let sampled = ContextDisclosedV1 {
            provenance_count: Some(3000),
            items: vec![Uuid::from_u128(1)],
            ..complete
        };
        assert!(
            sampled.provenance_count.unwrap_or_default()
                > u32::try_from(sampled.items.len()).unwrap_or(u32::MAX),
            "a sampled record could not be told from a complete one"
        );
    }

    #[test]
    fn a_record_carries_provenance_and_never_content() {
        let record = ContextDisclosedV1 {
            provenance_count: Some(1),
            destination: destination(false, true),
            items: vec![Uuid::from_u128(1)],
            item_count: 2,
            withheld: vec![Withheld {
                subject: Some("utterance".into()),
                because: WithheldBecause::AboveConsumerTrust,
            }],
            disclosed_at: "2026-08-21T00:00:00Z".into(),
        };
        let mut encoded = Vec::new();
        ciborium::into_writer(&record, &mut encoded).expect("encode");
        let decoded: ContextDisclosedV1 =
            ciborium::from_reader(encoded.as_slice()).expect("decode");
        assert_eq!(decoded, record);

        // The count and the provenance are allowed to disagree, and saying so is the point: this
        // record accounts for one of the two items it supplied.
        assert_eq!(decoded.items.len(), 1);
        assert_eq!(decoded.item_count, 2);
    }
}
