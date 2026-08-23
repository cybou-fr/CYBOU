// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Deciding what a given reader may be told, and keeping the account of it.
//!
//! This decision used to live inline inside the D-Bus adapter, which meant it could only run with a
//! session bus and a live Mind behind it. That is the same shape as every defect this codebase has
//! actually shipped: arithmetic welded to a component, a stylesheet nothing compiles, a daemon
//! behind a `cfg`. **The rule that decides what a stranger sees was the one rule no test could
//! run**, and the one live bug the disclosure surface has had — three thousand contributions
//! reported as accounted for against ten items supplied — came out of exactly this bookkeeping.
//!
//! Nothing here reaches anything. It is handed a sensitivity and hands back a verdict, so the whole
//! of it runs in an ordinary unit test on any machine.
//!
//! Two counts, and the difference between them is the point. `supplied` is how many items crossed
//! the boundary; `accounted_for` is how many of those could say where they came from. A concept
//! carries no evidence, so it is supplied and unaccountable — and a surface that reported one
//! number would be unable to show that, which is the whole thing the disclosure route exists for.

use std::collections::HashSet;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

use cybou_protocol::disclosure::{MAX_RECORDED_PROVENANCE, Withheld, WithheldBecause};
use uuid::Uuid;

use crate::Delivered;

/// What becomes of one item on its way to a reader.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Verdict {
    /// It may be passed on.
    Supply,
    /// It may not, for this reason.
    Withhold(WithheldBecause),
}

/// Whether an item this exposing may be passed to a reader permitted this much.
///
/// The whole policy, in one comparison, stated where it can be read. It is deliberately not a
/// method on anything: a rule that is a free function of two numbers cannot come to depend on the
/// state of whoever is applying it.
#[must_use]
pub const fn verdict(sensitivity: u8, permitted_sensitivity: u8) -> Verdict {
    if sensitivity <= permitted_sensitivity {
        Verdict::Supply
    } else {
        Verdict::Withhold(WithheldBecause::AboveConsumerTrust)
    }
}

/// The account of one delivery: what crossed, what did not, and what it came from.
///
/// Interior mutability because the readers that fill it share one source across a request and each
/// answers on its own. Nothing is held across an await.
#[derive(Debug, Default)]
pub struct Ledger {
    supplied: AtomicU64,
    accounted_for: AtomicU64,
    /// The sources recorded, and every source seen.
    ///
    /// Two structures because they answer different questions and only one of them may grow into a
    /// permanent record. The set decides whether a contribution is new — linear search over a
    /// thousands-long list made a wide delivery quadratic — and the list is what gets written,
    /// bounded. The set is discarded when the delivery ends; only its size outlives it.
    provenance: Mutex<(Vec<Uuid>, HashSet<Uuid>)>,
    withheld: Mutex<Vec<Withheld>>,
}

impl Ledger {
    /// An account with nothing in it.
    #[must_use]
    pub fn new() -> Self {
        Self {
            supplied: AtomicU64::new(0),
            accounted_for: AtomicU64::new(0),
            provenance: Mutex::new((Vec::new(), HashSet::new())),
            withheld: Mutex::new(Vec::new()),
        }
    }

    /// Start a fresh delivery, discarding what the last one recorded.
    ///
    /// One projection is one delivery. Accumulating across requests would report an item as held
    /// back long after the request it was held back from.
    pub fn begin(&self) {
        self.supplied.store(0, Ordering::Relaxed);
        self.accounted_for.store(0, Ordering::Relaxed);
        if let Ok(mut provenance) = self.provenance.lock() {
            provenance.0.clear();
            provenance.1.clear();
        }
        if let Ok(mut withheld) = self.withheld.lock() {
            withheld.clear();
        }
    }

    /// Note that something was supplied, and what it came from.
    pub fn supply(&self, evidence: &[Uuid]) {
        self.supplied.fetch_add(1, Ordering::Relaxed);
        if evidence.is_empty() {
            // Supplied and unaccountable: counted here and not in `accounted_for`, which is the
            // whole difference the disclosure surface exists to show.
            return;
        }
        self.accounted_for.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut provenance) = self.provenance.lock() {
            for id in evidence {
                if !provenance.1.insert(*id) {
                    continue;
                }
                // Counted by the set whether or not it is recorded, so the record can say how many
                // there were even when it could not carry them.
                if provenance.0.len() < MAX_RECORDED_PROVENANCE {
                    provenance.0.push(*id);
                }
            }
        }
    }

    /// Note that something was held back, and why.
    pub fn withhold(&self, subject: Option<String>, because: WithheldBecause) {
        if let Ok(mut withheld) = self.withheld.lock() {
            withheld.push(Withheld { subject, because });
        }
    }

    /// Decide one item, record what was decided, and answer whether to pass it on.
    ///
    /// `subject` is a closure so that a supplied item never pays for a string nobody will read, and
    /// so the only place a withheld subject is materialised is the branch that withholds it.
    ///
    /// The subject is recorded even when the reader may not see it. Which reader is told is the
    /// route's decision, not this one's: a person asking "why did it not tell me that?" needs the
    /// subject, and the record has to hold it for them to be answerable at all.
    pub fn decide<F>(
        &self,
        sensitivity: u8,
        permitted_sensitivity: u8,
        subject: F,
        evidence: &[Uuid],
    ) -> bool
    where
        F: FnOnce() -> Option<String>,
    {
        match verdict(sensitivity, permitted_sensitivity) {
            Verdict::Supply => {
                self.supply(evidence);
                true
            }
            Verdict::Withhold(because) => {
                self.withhold(subject(), because);
                false
            }
        }
    }

    /// The account as it now stands.
    #[must_use]
    pub fn delivered(&self) -> Delivered {
        let (items, provenance_count) = self.provenance.lock().map_or_else(
            |_| (Vec::new(), 0),
            |held| {
                (
                    held.0.clone(),
                    u32::try_from(held.1.len()).unwrap_or(u32::MAX),
                )
            },
        );
        Delivered {
            items,
            provenance_count,
            item_count: u32::try_from(self.supplied.load(Ordering::Relaxed)).unwrap_or(u32::MAX),
            accounted_for: u32::try_from(self.accounted_for.load(Ordering::Relaxed))
                .unwrap_or(u32::MAX),
            withheld: self
                .withheld
                .lock()
                .map(|held| held.clone())
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sensitivity an ordinary reader is permitted, and one above it.
    const ORDINARY: u8 = 0;
    const THE_PERSONS: u8 = 1;

    #[test]
    fn the_policy_is_one_comparison_and_says_which_it_made() {
        assert_eq!(verdict(ORDINARY, ORDINARY), Verdict::Supply);
        assert_eq!(
            verdict(THE_PERSONS, ORDINARY),
            Verdict::Withhold(WithheldBecause::AboveConsumerTrust)
        );
        assert_eq!(verdict(THE_PERSONS, THE_PERSONS), Verdict::Supply);
        assert_eq!(verdict(u8::MAX, u8::MAX), Verdict::Supply);
    }

    #[test]
    fn what_was_supplied_can_never_exceed_what_was_accounted_for_the_other_way() {
        // The one bug this surface has shipped, as an assertion. `items` is a set of sources on a
        // different scale entirely: two items citing five contributions between them is two
        // supplied, two accounted for, and five sources — not five of anything.
        let ledger = Ledger::new();
        ledger.supply(&[Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3)]);
        ledger.supply(&[Uuid::from_u128(3), Uuid::from_u128(4), Uuid::from_u128(5)]);

        let delivered = ledger.delivered();
        assert_eq!(delivered.item_count, 2);
        assert_eq!(delivered.accounted_for, 2);
        assert_eq!(delivered.items.len(), 5, "one source, counted once");
        assert_eq!(delivered.provenance_count, 5);
        assert!(delivered.accounted_for <= delivered.item_count);
    }

    #[test]
    fn something_supplied_that_cannot_say_where_it_came_from_is_counted_and_not_accounted_for() {
        // A concept carries no evidence. Reporting one number would hide that entirely.
        let ledger = Ledger::new();
        ledger.supply(&[Uuid::from_u128(1)]);
        ledger.supply(&[]);

        let delivered = ledger.delivered();
        assert_eq!(delivered.item_count, 2);
        assert_eq!(delivered.accounted_for, 1);
    }

    #[test]
    fn a_withheld_item_is_recorded_with_its_subject_and_never_its_value() {
        let ledger = Ledger::new();
        let passed = ledger.decide(
            THE_PERSONS,
            ORDINARY,
            || Some("disk-pressure".to_owned()),
            &[Uuid::from_u128(9)],
        );

        assert!(!passed);
        let delivered = ledger.delivered();
        assert_eq!(delivered.item_count, 0, "nothing crossed");
        assert!(
            delivered.items.is_empty(),
            "a withheld item put its sources into a permanent record"
        );
        assert_eq!(
            delivered.withheld,
            vec![Withheld {
                subject: Some("disk-pressure".to_owned()),
                because: WithheldBecause::AboveConsumerTrust,
            }]
        );
    }

    #[test]
    fn one_delivery_does_not_report_what_the_last_one_held_back() {
        // Accumulating across requests would tell a reader that something was kept from them long
        // after the request it was kept from.
        let ledger = Ledger::new();
        ledger.decide(THE_PERSONS, ORDINARY, || Some("mood".to_owned()), &[]);
        assert_eq!(ledger.delivered().withheld.len(), 1);

        ledger.begin();
        ledger.decide(ORDINARY, ORDINARY, || None, &[Uuid::from_u128(1)]);

        let delivered = ledger.delivered();
        assert!(delivered.withheld.is_empty());
        assert_eq!(delivered.item_count, 1);
    }

    #[test]
    fn deciding_records_exactly_one_outcome_per_item() {
        // Every item is either supplied or withheld, and the two counts plus the withheld list
        // account for all of them. An item that fell through both would be invisible.
        let ledger = Ledger::new();
        for index in 0..10u128 {
            let sensitivity = if index % 3 == 0 {
                THE_PERSONS
            } else {
                ORDINARY
            };
            ledger.decide(
                sensitivity,
                ORDINARY,
                || Some(format!("subject-{index}")),
                &[Uuid::from_u128(index)],
            );
        }

        let delivered = ledger.delivered();
        assert_eq!(
            usize::try_from(delivered.item_count).expect("a small count")
                + delivered.withheld.len(),
            10
        );
    }

    #[test]
    fn a_delivery_citing_more_sources_than_it_can_record_says_how_many_there_were() {
        // Unbounded, this grew with the biography and went into a permanent contribution on every
        // delivery. Bounded and silent it would be worse: a permanent record that omits without
        // saying so. The record says so by arithmetic — the count exceeds what it carries.
        let ledger = Ledger::new();
        let cited: Vec<Uuid> = (0..(MAX_RECORDED_PROVENANCE as u128 * 3))
            .map(Uuid::from_u128)
            .collect();
        ledger.supply(&cited);

        let delivered = ledger.delivered();
        assert_eq!(delivered.items.len(), MAX_RECORDED_PROVENANCE);
        assert_eq!(
            delivered.provenance_count,
            u32::try_from(cited.len()).expect("a small count")
        );
        assert!(
            usize::try_from(delivered.provenance_count).expect("a small count")
                > delivered.items.len(),
            "a truncated record could not be told from a complete one"
        );
    }

    #[test]
    fn a_delivery_within_the_bound_records_every_source_it_cited() {
        // The control. A record that always claimed to be truncated would pass the test above.
        let ledger = Ledger::new();
        ledger.supply(&[Uuid::from_u128(1), Uuid::from_u128(2)]);

        let delivered = ledger.delivered();
        assert_eq!(delivered.items.len(), 2);
        assert_eq!(delivered.provenance_count, 2);
    }

    #[test]
    fn a_reader_permitted_everything_is_told_everything() {
        // The control. A policy that withheld regardless would pass every test above.
        let ledger = Ledger::new();
        ledger.decide(
            THE_PERSONS,
            u8::MAX,
            || Some("disk-pressure".to_owned()),
            &[Uuid::from_u128(1)],
        );

        let delivered = ledger.delivered();
        assert_eq!(delivered.item_count, 1);
        assert!(delivered.withheld.is_empty());
    }
}
