// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Recording that context was supplied to someone (ADR-0030 B4, B6).
//!
//! The biography could say what the system did and not who looked at it. This is the other
//! direction: who received something, what it was derived from, and what was held back from them.
//!
//! What is recorded is a delivery, not a request. A reader watching the event stream receives the
//! same projection every few seconds, and a contribution per response would fill the Journal with
//! thousands of identical rows that answer no question anyone would ask. A delivery is a *change*
//! in what a consumer is being supplied — the first one, and every one after that differs. That is
//! the granularity at which "what did this consumer see?" has an answer, and the granularity at
//! which the answer stays readable.

use std::sync::Mutex;

use cybou_protocol::{
    Kind,
    canonical::CanonicalEnvelope,
    disclosure::{ContextDisclosedV1, Destination},
    unix_millis,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::Delivered;

/// How many deliveries are remembered per consumer.
///
/// A person asking *what was I supplied last week* needs more than the last one, and a gateway
/// holding every delivery it ever made is a gateway that runs out of memory on a long uptime. Only
/// changes are recorded — a reader receiving the same projection every few seconds produces no new
/// entry — so sixteen covers a real span rather than sixteen seconds.
///
/// The record that matters durably is the `ContextDisclosed` contribution in the Journal. This is a
/// window onto the recent part of it, held here so the surface answers without a Journal query.
pub const DELIVERIES_REMEMBERED: usize = 16;

/// How many consumers are remembered at all.
///
/// A destination is `living-canvas:<username>`, so this is bounded by the people who sign in and
/// not by traffic. Bounded anyway: an in-memory map keyed by anything derived from a request is one
/// change away from being keyed by something unbounded, and finding that out on a long-running
/// deployment is finding it out the expensive way.
const CONSUMERS_REMEMBERED: usize = 64;

/// One delivery, and when it was recorded.
#[derive(Clone, Debug, PartialEq)]
pub struct Recorded {
    /// What crossed and what did not.
    pub delivered: Delivered,
    /// When this was recorded.
    pub at: OffsetDateTime,
}

/// Builds disclosure records, and knows what it has already recorded for each consumer.
pub struct Disclosures {
    /// What has been recorded as supplied to each destination, oldest first.
    ///
    /// The last of each is compared rather than counted: the question is whether this consumer is
    /// now seeing something different, not how many times it has asked. The ones before it are what
    /// lets a person see what they were supplied last week and not only what they are being
    /// supplied now.
    history: Mutex<Vec<(String, Vec<Recorded>)>>,
}

impl Default for Disclosures {
    fn default() -> Self {
        Self::new()
    }
}

impl Disclosures {
    /// A recorder that has not yet seen any delivery.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            history: Mutex::new(Vec::new()),
        }
    }

    /// What this consumer was last recorded as being supplied, if anything ever was.
    ///
    /// `None` and an empty delivery are different answers and the caller must be able to tell them
    /// apart: one says nothing has been supplied to this consumer, the other says something was
    /// supplied and it was empty.
    #[must_use]
    pub fn last_for(&self, destination_id: &str) -> Option<Delivered> {
        let history = self.history.lock().ok()?;
        history
            .iter()
            .find(|(id, _)| id == destination_id)
            .and_then(|(_, recorded)| recorded.last())
            .map(|recorded| recorded.delivered.clone())
    }

    /// Every delivery to this consumer that is still remembered, newest first.
    ///
    /// Bounded by [`DELIVERIES_REMEMBERED`]. A caller that received the bound back has not been
    /// told there were only that many — the Journal holds the durable record, and this is a window
    /// onto its recent end.
    #[must_use]
    pub fn history_for(&self, destination_id: &str) -> Vec<Recorded> {
        let Ok(history) = self.history.lock() else {
            return Vec::new();
        };
        history
            .iter()
            .find(|(id, _)| id == destination_id)
            .map(|(_, recorded)| recorded.iter().rev().cloned().collect())
            .unwrap_or_default()
    }

    /// The contribution to record for this delivery, or `None` when there is nothing new to say.
    ///
    /// `None` covers two cases that are the same fact: a consumer that keeps nothing and stays on
    /// this side of every boundary needs no record, and a consumer receiving exactly what it was
    /// last recorded as receiving has not been supplied anything since.
    pub fn record_for(
        &self,
        destination: &Destination,
        delivered: &Delivered,
        now: OffsetDateTime,
    ) -> Option<CanonicalEnvelope> {
        if !destination.needs_a_record() {
            return None;
        }

        {
            let mut history = self.history.lock().ok()?;
            if let Some((_, recorded)) = history.iter().find(|(id, _)| id == &destination.id)
                && recorded
                    .last()
                    .is_some_and(|previous| &previous.delivered == delivered)
            {
                return None;
            }
            // Moved to the end as well as appended to, so the consumer evicted when the bound binds
            // is the one nobody has supplied anything to for longest.
            let mut recorded = history
                .iter()
                .position(|(id, _)| id == &destination.id)
                .map(|index| history.remove(index).1)
                .unwrap_or_default();
            recorded.push(Recorded {
                delivered: delivered.clone(),
                at: now,
            });
            while recorded.len() > DELIVERIES_REMEMBERED {
                recorded.remove(0);
            }
            history.push((destination.id.clone(), recorded));
            while history.len() > CONSUMERS_REMEMBERED {
                history.remove(0);
            }
        }

        let record = ContextDisclosedV1 {
            destination: destination.clone(),
            items: delivered.items.clone(),
            provenance_count: Some(delivered.provenance_count),
            item_count: delivered.item_count,
            withheld: delivered.withheld.clone(),
            disclosed_at: now.format(&Rfc3339).ok()?,
        };
        let mut payload = Vec::new();
        ciborium::into_writer(&record, &mut payload).ok()?;

        Some(CanonicalEnvelope {
            schema_version: 4,
            message_id: Uuid::new_v4(),
            correlation_id: Uuid::new_v4(),
            // A disclosure happened outside the Journal, so it cites nothing. `ContextDisclosed` is
            // a root kind for exactly this reason.
            causation_id: Uuid::nil(),
            origin_organ: "web-gateway".to_owned(),
            origin_node: String::new(),
            kind: Kind::ContextDisclosed as u16,
            wall_time_ms: unix_millis(now),
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
            // That somebody read something is about the person whose biography it is, even though
            // the record carries none of what was read.
            sensitivity: 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use cybou_protocol::disclosure::{ConsumerTrust, Withheld, WithheldBecause};

    use super::*;

    fn a_delivery(items: u32) -> Delivered {
        Delivered {
            items: Vec::new(),
            provenance_count: items,
            item_count: items,
            accounted_for: items,
            withheld: Vec::new(),
        }
    }

    #[test]
    fn a_person_can_see_what_they_were_supplied_before_now() {
        // The surface answered "what am I being given" and nothing else, which makes it a status
        // light rather than a record. What was supplied last week is the question a record is for.
        let disclosures = Disclosures::new();
        let destination = Destination {
            id: "living-canvas:owner".to_owned(),
            trust: ConsumerTrust::Owner,
            retains: false,
            external_boundary: true,
        };
        for (minute, items) in [(1i64, 3u32), (2, 5), (3, 4)] {
            disclosures.record_for(&destination, &a_delivery(items), at(minute));
        }

        let history = disclosures.history_for(&destination.id);
        let counts: Vec<u32> = history
            .iter()
            .map(|recorded| recorded.delivered.item_count)
            .collect();
        assert_eq!(counts, vec![4, 5, 3], "newest first");
        assert_eq!(history[0].at, at(3));
        assert_eq!(
            disclosures.last_for(&destination.id).map(|d| d.item_count),
            Some(4),
            "the most recent is still what the top of the card describes"
        );
    }

    #[test]
    fn a_consumer_receiving_the_same_thing_twice_does_not_grow_a_history() {
        // A reader watching the event stream receives the same projection every few seconds. An
        // entry per response would fill sixteen slots in under a minute and answer no question
        // anybody would ask.
        let disclosures = Disclosures::new();
        let destination = Destination {
            id: "living-canvas:owner".to_owned(),
            trust: ConsumerTrust::Owner,
            retains: false,
            external_boundary: true,
        };
        for minute in 1..=8 {
            disclosures.record_for(&destination, &a_delivery(3), at(minute));
        }
        assert_eq!(disclosures.history_for(&destination.id).len(), 1);
    }

    #[test]
    fn a_long_lived_gateway_does_not_remember_every_delivery_it_ever_made() {
        // Held in memory in a process meant to run for months. The durable record is the
        // ContextDisclosed contribution in the Journal; this is a window onto its recent end.
        let disclosures = Disclosures::new();
        let destination = Destination {
            id: "living-canvas:owner".to_owned(),
            trust: ConsumerTrust::Owner,
            retains: false,
            external_boundary: true,
        };
        let overshoot = u32::try_from(DELIVERIES_REMEMBERED).expect("small") + 20;
        for items in 1..=overshoot {
            disclosures.record_for(&destination, &a_delivery(items), at(i64::from(items)));
        }

        let history = disclosures.history_for(&destination.id);
        assert_eq!(history.len(), DELIVERIES_REMEMBERED);
        // And what it dropped is the oldest, not the newest.
        assert_eq!(history[0].delivered.item_count, overshoot,);
    }

    #[test]
    fn one_consumer_history_is_not_another_consumer_history() {
        let disclosures = Disclosures::new();
        let owner = Destination {
            id: "living-canvas:owner".to_owned(),
            trust: ConsumerTrust::Owner,
            retains: false,
            external_boundary: true,
        };
        let public = Destination {
            id: "living-canvas:public".to_owned(),
            trust: ConsumerTrust::Public,
            retains: false,
            external_boundary: true,
        };
        disclosures.record_for(&owner, &a_delivery(9), at(1));
        disclosures.record_for(&public, &a_delivery(2), at(2));

        assert_eq!(disclosures.history_for(&owner.id).len(), 1);
        assert_eq!(
            disclosures.history_for(&public.id)[0].delivered.item_count,
            2
        );
        assert!(
            disclosures.history_for("living-canvas:nobody").is_empty(),
            "a consumer nothing was supplied to has no history rather than somebody else's"
        );
    }

    fn at(minute: i64) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + time::Duration::minutes(minute)
    }

    fn public() -> Destination {
        Destination {
            id: "living-canvas:public".into(),
            trust: ConsumerTrust::Public,
            retains: false,
            external_boundary: true,
        }
    }

    fn delivered(count: u32) -> Delivered {
        Delivered {
            items: vec![Uuid::from_u128(1)],
            provenance_count: 1,
            item_count: count,
            accounted_for: count,
            withheld: vec![Withheld {
                subject: Some("utterance".into()),
                because: WithheldBecause::AboveConsumerTrust,
            }],
        }
    }

    #[test]
    fn the_first_delivery_to_a_consumer_is_recorded() {
        let disclosures = Disclosures::new();
        let record = disclosures
            .record_for(&public(), &delivered(3), at(0))
            .expect("a first delivery is something new");
        assert_eq!(record.kind, Kind::ContextDisclosed as u16);
        assert_eq!(record.origin_organ, "web-gateway");
        assert_eq!(
            record.causation_id,
            Uuid::nil(),
            "a disclosure cites nothing"
        );
    }

    #[test]
    fn asking_again_for_the_same_thing_is_not_a_new_delivery() {
        // Otherwise a reader watching the event stream would write a contribution every few
        // seconds, and thousands of identical rows answer no question anyone would ask.
        let disclosures = Disclosures::new();
        disclosures
            .record_for(&public(), &delivered(3), at(0))
            .expect("the first is recorded");
        assert!(
            disclosures
                .record_for(&public(), &delivered(3), at(1))
                .is_none()
        );
    }

    #[test]
    fn being_supplied_something_different_is_a_new_delivery() {
        let disclosures = Disclosures::new();
        disclosures
            .record_for(&public(), &delivered(3), at(0))
            .expect("the first is recorded");
        assert!(
            disclosures
                .record_for(&public(), &delivered(4), at(1))
                .is_some(),
            "a consumer now seeing something else has been supplied something else"
        );
    }

    #[test]
    fn two_consumers_are_recorded_apart_from_each_other() {
        let disclosures = Disclosures::new();
        let owner = Destination {
            id: "living-canvas:alice".into(),
            trust: ConsumerTrust::Owner,
            retains: false,
            external_boundary: true,
        };
        disclosures
            .record_for(&public(), &delivered(3), at(0))
            .expect("the public reader");
        assert!(
            disclosures
                .record_for(&owner, &delivered(3), at(0))
                .is_some(),
            "another consumer receiving the same thing has still been supplied it"
        );
    }

    #[test]
    fn a_consumer_that_keeps_nothing_and_crosses_nothing_is_not_recorded() {
        let inspector = Destination {
            id: "inspector".into(),
            trust: ConsumerTrust::Owner,
            retains: false,
            external_boundary: false,
        };
        let disclosures = Disclosures::new();
        assert!(
            disclosures
                .record_for(&inspector, &delivered(3), at(0))
                .is_none(),
            "an inspector that renders and forgets needs no contribution"
        );
    }

    #[test]
    fn the_record_names_what_was_held_back_without_restating_it() {
        let disclosures = Disclosures::new();
        let record = disclosures
            .record_for(&public(), &delivered(3), at(0))
            .expect("a record");
        let decoded: ContextDisclosedV1 =
            ciborium::from_reader(record.payload.as_slice()).expect("a readable record");
        assert_eq!(decoded.withheld.len(), 1);
        assert_eq!(
            decoded.withheld[0].because,
            WithheldBecause::AboveConsumerTrust
        );
        assert_eq!(decoded.withheld[0].subject.as_deref(), Some("utterance"));
        assert_eq!(decoded.item_count, 3);
        // Provenance, not content: one contribution is named and nothing it said is here.
        assert_eq!(decoded.items, vec![Uuid::from_u128(1)]);
    }
}
