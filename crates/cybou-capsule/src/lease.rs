// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! How long a grant lasts, what it has spent, and how it ends.
//!
//! [`crate::grant::CapsuleGrant`] says what a capsule may do. It says nothing about *until when*,
//! and a permission with no end is the one shape of grant nobody can withdraw by waiting. So the
//! lease carries the clock and the ledger, and every verdict is asked against it.
//!
//! ## Ending is not asking
//!
//! When a lease ends, the agent is not told to stop. Telling an untrusted party to stop is a request,
//! and a boundary made of requests is not a boundary — the whole of ADR-0042 turns on that sentence,
//! and it applies to the clock as much as to the network.
//!
//! What actually happens is that nothing further is permitted, and the capsule is frozen by whatever
//! holds it. This module produces the first half: after the end, no request is `Allowed`. It cannot
//! produce the second and does not pretend to.
//!
//! ## A ceiling is not a budget line
//!
//! The spending ceiling is enforced where the accounting is — at the model gateway, which sees every
//! completion — and never asked of the agent. An agent reporting its own consumption is the executor
//! grading its own homework, refused elsewhere in this repository in the case where it matters most.
//!
//! Spend is recorded in whole units of the operator's smallest currency denomination. Comparing a
//! ceiling as a float is a ceiling that is occasionally exceeded by a fraction in whichever direction
//! the rounding went, and "occasionally" is not a property a limit may have.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::grant::CapsuleGrant;

/// Why a lease is no longer good.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ended {
    /// Its lifetime ran out.
    Expired,
    /// It reached what it was allowed to spend.
    SpentOut,
    /// A person or a policy withdrew it.
    ///
    /// Distinct from the other two, because it is the one somebody did on purpose and the only one
    /// that can happen while everything else is fine. A surface that reported all three the same way
    /// would tell an operator their agent ran out of time when in fact they stopped it.
    Revoked,
}

impl Ended {
    /// How this reads to a person.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Expired => "the lease reached the end of its lifetime",
            Self::SpentOut => "the lease reached what it was allowed to spend",
            Self::Revoked => "the lease was withdrawn",
        }
    }
}

/// A grant, bounded in time and spending.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lease {
    /// What was granted.
    pub grant: CapsuleGrant,
    /// When it was issued.
    #[serde(with = "time::serde::rfc3339")]
    pub issued_at: OffsetDateTime,
    /// What has been spent against it.
    pub spent: u64,
    /// Whether somebody withdrew it, and when.
    ///
    /// Recorded rather than represented by deleting the lease: an agent stopped at 14:02 is a fact
    /// about what happened, and a record that could only say a lease does not exist could not say
    /// that it used to.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub revoked_at: Option<OffsetDateTime>,
}

impl Lease {
    /// Issue a lease for this grant.
    #[must_use]
    pub fn issued(grant: CapsuleGrant, at: OffsetDateTime) -> Self {
        Self {
            grant,
            issued_at: at,
            spent: 0,
            revoked_at: None,
        }
    }

    /// When this lease runs out.
    #[must_use]
    pub fn expires_at(&self) -> OffsetDateTime {
        self.issued_at + self.grant.budget.lifetime
    }

    /// Why this lease is no longer good, if it is not.
    ///
    /// Revocation is checked first. A lease that was withdrawn *and* has since expired was withdrawn:
    /// that is what somebody did, and reporting the expiry instead would quietly replace an action a
    /// person took with a clock running out.
    #[must_use]
    pub fn ended(&self, now: OffsetDateTime) -> Option<Ended> {
        if self.revoked_at.is_some() {
            return Some(Ended::Revoked);
        }
        if now >= self.expires_at() {
            return Some(Ended::Expired);
        }
        // At the ceiling, not past it. A lease that may spend a hundred has spent its hundred when
        // it reaches a hundred, and `>` would let every ceiling be exceeded by exactly one unit.
        if self.spent >= self.grant.budget.model_spend_limit {
            return Some(Ended::SpentOut);
        }
        None
    }

    /// Whether this lease still permits anything.
    #[must_use]
    pub fn is_live(&self, now: OffsetDateTime) -> bool {
        self.ended(now).is_none()
    }

    /// What remains of the spending ceiling.
    #[must_use]
    pub const fn remaining_spend(&self) -> u64 {
        self.grant
            .budget
            .model_spend_limit
            .saturating_sub(self.spent)
    }

    /// Record what a completion cost.
    ///
    /// Saturating, so a cost larger than the whole ceiling ends the lease rather than wrapping to a
    /// small number and permitting the next request. There is no arithmetic here that can make a
    /// lease look healthier than it is.
    pub const fn charge(&mut self, cost: u64) {
        self.spent = self.spent.saturating_add(cost);
    }

    /// Withdraw this lease.
    ///
    /// Idempotent, and it keeps the first instant. A second revocation is not a second event, and
    /// letting it move the time would let a repeated call quietly rewrite when an agent was stopped.
    pub const fn revoke(&mut self, at: OffsetDateTime) {
        if self.revoked_at.is_none() {
            self.revoked_at = Some(at);
        }
    }
}

/// Decide a request against a lease.
///
/// The same decision [`crate::verdict::decide`] makes, and then the clock and the ledger. A grant
/// that has run out permits nothing, including the things it used to permit — which is the point of
/// giving one an end at all.
///
/// A request that would be refused outright is still reported as refused rather than as expired. The
/// two are different facts about what an agent tried, and an audit that recorded *lease ended* for an
/// attempt to read the key store would have lost the interesting half.
#[must_use]
pub fn decide_under_lease(
    lease: &Lease,
    reach: &crate::reach::Reach,
    now: OffsetDateTime,
) -> crate::verdict::Verdict {
    let verdict = crate::verdict::decide(&lease.grant, reach);
    if !verdict.is_allowed() {
        return verdict;
    }
    match lease.ended(now) {
        Some(ended) => crate::verdict::Verdict::NotGranted {
            wanted: format!("{}, but {}", reach.name(), ended.describe()),
        },
        None => verdict,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use time::Duration;
    use uuid::Uuid;

    use super::*;
    use crate::grant::{NetworkGrant, ResourceBudget, Workspace};
    use crate::reach::Reach;
    use crate::verdict::Verdict;

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    fn grant() -> CapsuleGrant {
        CapsuleGrant {
            capsule_id: Uuid::from_u128(8472),
            agent: "opencode".to_owned(),
            workspace: Workspace::at("/srv/project"),
            network: NetworkGrant::to(&["github.com"]),
            budget: ResourceBudget {
                memory_mib: 4096,
                cpus: 2,
                lifetime: Duration::hours(4),
                model_spend_limit: 100,
            },
            model_class: "Strong".to_owned(),
            tools: vec!["git".to_owned()],
            may_execute: true,
        }
    }

    fn reading() -> Reach {
        Reach::ReadPath {
            path: PathBuf::from("/srv/project/src/main.rs"),
        }
    }

    #[test]
    fn a_live_lease_permits_what_the_grant_permits() {
        let lease = Lease::issued(grant(), at(0));
        assert!(lease.is_live(at(60)));
        assert!(decide_under_lease(&lease, &reading(), at(60)).is_allowed());
    }

    #[test]
    fn a_lease_that_ran_out_permits_nothing_it_used_to() {
        // The point of giving a grant an end. Four hours to the second is over.
        let lease = Lease::issued(grant(), at(0));
        let four_hours = 4 * 60 * 60;
        assert!(lease.is_live(at(four_hours - 1)));
        assert_eq!(lease.ended(at(four_hours)), Some(Ended::Expired));
        assert!(!decide_under_lease(&lease, &reading(), at(four_hours)).is_allowed());
    }

    #[test]
    fn a_ceiling_is_reached_at_the_ceiling_and_not_one_unit_past_it() {
        // `>` would let every ceiling be exceeded by exactly one unit, which is the kind of error
        // that is invisible until somebody adds up a month of them.
        let mut lease = Lease::issued(grant(), at(0));
        lease.charge(99);
        assert!(lease.is_live(at(60)), "99 of 100 is not spent out");
        assert_eq!(lease.remaining_spend(), 1);

        lease.charge(1);
        assert_eq!(lease.ended(at(60)), Some(Ended::SpentOut));
        assert_eq!(lease.remaining_spend(), 0);
    }

    #[test]
    fn a_cost_larger_than_the_whole_ceiling_ends_the_lease_rather_than_wrapping() {
        // Saturating, so no arithmetic here can make a lease look healthier than it is.
        let mut lease = Lease::issued(grant(), at(0));
        lease.charge(u64::MAX);
        lease.charge(u64::MAX);
        assert_eq!(lease.spent, u64::MAX);
        assert_eq!(lease.ended(at(60)), Some(Ended::SpentOut));
        assert_eq!(lease.remaining_spend(), 0);
    }

    #[test]
    fn revocation_is_immediate_and_keeps_the_instant_it_first_happened() {
        // A second revocation is not a second event. Letting it move the time would let a repeated
        // call quietly rewrite when an agent was stopped.
        let mut lease = Lease::issued(grant(), at(0));
        lease.revoke(at(120));
        assert_eq!(lease.ended(at(121)), Some(Ended::Revoked));
        assert!(!decide_under_lease(&lease, &reading(), at(121)).is_allowed());

        lease.revoke(at(900));
        assert_eq!(lease.revoked_at, Some(at(120)));
    }

    #[test]
    fn a_lease_that_was_withdrawn_and_then_expired_was_withdrawn() {
        // What somebody did outranks what the clock did. Reporting the expiry would quietly replace
        // a person's action with a timer running out.
        let mut lease = Lease::issued(grant(), at(0));
        lease.revoke(at(60));
        assert_eq!(lease.ended(at(4 * 60 * 60 + 1)), Some(Ended::Revoked));
    }

    #[test]
    fn a_request_that_would_be_refused_outright_is_not_reported_as_a_dead_lease() {
        // Two different facts about what an agent tried. An audit recording "lease ended" for an
        // attempt to read the key store would have lost the interesting half.
        let mut lease = Lease::issued(grant(), at(0));
        lease.revoke(at(60));

        let verdict = decide_under_lease(&lease, &Reach::ReachTheKeyStore, at(120));
        assert!(
            matches!(verdict, Verdict::Refused { .. }),
            "an attempt on the key store was recorded as a lease problem: {verdict:?}"
        );
    }

    #[test]
    fn an_ended_lease_says_which_way_it_ended() {
        // Three endings, and only one of them is something a person did. A surface reporting them
        // identically would tell an operator their agent ran out of time when they stopped it.
        let mut expired = Lease::issued(grant(), at(0));
        expired.charge(1);
        let mut spent = Lease::issued(grant(), at(0));
        spent.charge(100);
        let mut revoked = Lease::issued(grant(), at(0));
        revoked.revoke(at(10));

        assert_eq!(expired.ended(at(4 * 60 * 60)), Some(Ended::Expired));
        assert_eq!(spent.ended(at(60)), Some(Ended::SpentOut));
        assert_eq!(revoked.ended(at(60)), Some(Ended::Revoked));

        let mut described: Vec<&str> = [Ended::Expired, Ended::SpentOut, Ended::Revoked]
            .iter()
            .map(|ending| ending.describe())
            .collect();
        described.sort_unstable();
        described.dedup();
        assert_eq!(described.len(), 3);
    }

    #[test]
    fn a_lease_survives_the_wire() {
        // It travels from whatever issues it to whatever checks it, and those are different
        // processes by design.
        let mut lease = Lease::issued(grant(), at(0));
        lease.charge(42);
        lease.revoke(at(30));

        let mut encoded = Vec::new();
        ciborium::into_writer(&lease, &mut encoded).expect("encodes");
        let decoded: Lease = ciborium::from_reader(encoded.as_slice()).expect("decodes");
        assert_eq!(decoded, lease);
    }
}
