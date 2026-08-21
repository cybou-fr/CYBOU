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
    /// The contributions the supplied items were derived from.
    ///
    /// Provenance rather than content: what was disclosed can be reconstructed from the Journal by
    /// anyone entitled to read it, and cannot be read out of this record by anyone who is not.
    pub items: Vec<Uuid>,
    /// How many items were supplied, including any whose provenance could not be established.
    ///
    /// Separate from the length of `items` on purpose. A projection that lost track of where one of
    /// its rows came from should say it supplied five things and can account for four, rather than
    /// quietly claim it supplied four.
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
    fn a_record_carries_provenance_and_never_content() {
        let record = ContextDisclosedV1 {
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
