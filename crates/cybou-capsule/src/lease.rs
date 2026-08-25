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
//! ## Two kinds of ending, and they are not the same kind
//!
//! ```text
//! lifetime over, or withdrawn  ->  the capsule is finished
//! model money spent            ->  completions are refused
//! ```
//!
//! Folding them together was wrong twice over. It made a capsule that runs out of model budget
//! indistinguishable from one that must be frozen, so an agent halfway through a compile would have
//! been stopped for running out of money it was not spending. And because a zero ceiling read as an
//! exhausted one, a capsule that wanted no model at all was dead before it started — including the
//! grant this crate hands out as the starting point for building a profile, and every capsule on an
//! unplugged host, which is the configuration this system exists to survive.
//!
//! ## A ceiling is not a budget line
//!
//! The spending ceiling is enforced where the accounting is — at the model gateway, which sees every
//! completion — and never asked of the agent. An agent reporting its own consumption is the executor
//! grading its own homework, refused elsewhere in this repository in the case where it matters most.
//!
//! Spend is whole units of the operator's smallest currency denomination. Comparing a ceiling as a
//! float is a ceiling that is occasionally exceeded by a fraction in whichever direction the
//! rounding went, and *occasionally* is not a property a limit may have.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::grant::CapsuleGrant;
use crate::profile::ProfileId;
use crate::reach::Reach;
use crate::verdict::{Verdict, decide};

/// Why a capsule is finished.
///
/// Two, not three. Running out of model budget is not here, because it does not finish a capsule —
/// it refuses completions, and a capsule with no model grant never has it happen at all.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Ended {
    /// Its lifetime ran out.
    Expired,
    /// A person or a policy withdrew it.
    ///
    /// Distinct from expiry, because it is the one somebody did on purpose and the only one that can
    /// happen while everything else is fine. A surface reporting both the same way would tell an
    /// operator their agent ran out of time when in fact they stopped it.
    Revoked,
}

impl Ended {
    /// How this reads to a person.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Expired => "the lease reached the end of its lifetime",
            Self::Revoked => "the lease was withdrawn",
        }
    }
}

/// A grant, bounded in time, with a ledger of what it has spent.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lease {
    /// Which explicit launch profile produced this lease.
    pub(crate) profile_id: ProfileId,
    /// What was granted.
    pub(crate) grant: CapsuleGrant,
    /// When it was issued.
    #[serde(with = "time::serde::rfc3339")]
    pub(crate) issued_at: OffsetDateTime,
    /// What has been spent against the model grant, if there is one.
    ///
    /// The ledger belongs to the lease and the ceiling belongs to the grant: one is what happened,
    /// the other is what was permitted. Keeping them together would mean a record of spending that
    /// could be edited by re-granting.
    pub(crate) model_spent: u64,
    /// Whether somebody withdrew it, and when.
    ///
    /// Recorded rather than represented by deleting the lease: an agent stopped at 14:02 is a fact
    /// about what happened, and a record that could only say a lease does not exist could not say
    /// that it used to.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub(crate) revoked_at: Option<OffsetDateTime>,
}

impl Lease {
    /// Mint a lease after the selected profile and resulting grant have been validated.
    pub(crate) fn issued_from_profile(
        profile_id: ProfileId,
        grant: CapsuleGrant,
        at: OffsetDateTime,
    ) -> Self {
        Self {
            profile_id,
            grant,
            issued_at: at,
            model_spent: 0,
            revoked_at: None,
        }
    }

    #[cfg(test)]
    fn issued(grant: CapsuleGrant, at: OffsetDateTime) -> Self {
        Self::issued_from_profile(
            ProfileId::parse("test-profile").expect("static profile id"),
            grant,
            at,
        )
    }

    /// The explicit launch profile that produced this lease.
    #[must_use]
    pub const fn profile_id(&self) -> &ProfileId {
        &self.profile_id
    }

    /// The exact capability grant enforced for this lease.
    #[must_use]
    pub const fn grant(&self) -> &CapsuleGrant {
        &self.grant
    }

    /// When this lease was minted.
    #[must_use]
    pub const fn issued_at(&self) -> OffsetDateTime {
        self.issued_at
    }

    /// What has been charged against its model ceiling.
    #[must_use]
    pub const fn model_spent(&self) -> u64 {
        self.model_spent
    }

    /// When this lease was first withdrawn, if it was.
    #[must_use]
    pub const fn revoked_at(&self) -> Option<OffsetDateTime> {
        self.revoked_at
    }

    /// When this lease runs out.
    #[must_use]
    pub fn expires_at(&self) -> OffsetDateTime {
        self.issued_at + self.grant.budget.lifetime
    }

    /// Why this capsule is finished, if it is.
    ///
    /// Revocation is checked first. A lease that was withdrawn *and* has since expired was withdrawn:
    /// that is what somebody did, and reporting the expiry instead would quietly replace an action a
    /// person took with a clock running out.
    #[must_use]
    pub fn ended(&self, now: OffsetDateTime) -> Option<Ended> {
        if self.revoked_at.is_some() {
            return Some(Ended::Revoked);
        }
        (now >= self.expires_at()).then_some(Ended::Expired)
    }

    /// Whether this capsule still exists as far as any permission is concerned.
    #[must_use]
    pub fn is_live(&self, now: OffsetDateTime) -> bool {
        self.ended(now).is_none()
    }

    /// Whether this lease may ask a model of this class for anything.
    ///
    /// False for a capsule with no model grant, which is a capsule that was never going to ask. That
    /// is not an ending and not a failure — on an unplugged host it is the ordinary case.
    #[must_use]
    pub fn may_use_model(&self, class: &str) -> bool {
        self.grant
            .model
            .as_ref()
            .is_some_and(|model| model.class == class && self.model_spent < model.spend_limit)
    }

    /// Whether a model grant exists and has been used up.
    ///
    /// Distinct from having no model grant, and the distinction is what a surface needs: one says
    /// *this capsule spent what you gave it*, the other says *you gave it none*.
    #[must_use]
    pub fn model_spent_out(&self) -> bool {
        self.grant
            .model
            .as_ref()
            // At the ceiling, not past it. `>` would let every ceiling be exceeded by exactly one
            // unit, which stays invisible until a month of them is added up.
            .is_some_and(|model| self.model_spent >= model.spend_limit)
    }

    /// What remains of the spending ceiling, or nothing if there is no model grant.
    #[must_use]
    pub fn remaining_spend(&self) -> Option<u64> {
        self.grant
            .model
            .as_ref()
            .map(|model| model.spend_limit.saturating_sub(self.model_spent))
    }

    /// Record what a completion cost.
    ///
    /// Saturating, so a cost larger than the whole ceiling exhausts the grant rather than wrapping
    /// to a small number and permitting the next request. There is no arithmetic here that can make
    /// a lease look healthier than it is.
    pub const fn charge(&mut self, cost: u64) {
        self.model_spent = self.model_spent.saturating_add(cost);
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
/// The same decision [`decide`] makes, and then the clock and the ledger.
///
/// A request that would be refused outright is still reported as refused rather than as expired. The
/// two are different facts about what an agent tried, and an audit recording *lease ended* for an
/// attempt to read the key store would have lost the interesting half.
#[must_use]
pub fn decide_under_lease(lease: &Lease, reach: &Reach, now: OffsetDateTime) -> Verdict {
    let verdict = decide(&lease.grant, reach);
    if !verdict.is_allowed() {
        return verdict;
    }

    // A finished capsule permits nothing. This is the whole reason a grant is given an end.
    if let Some(ended) = lease.ended(now) {
        return Verdict::NotGranted {
            wanted: format!("{}, but {}", reach.name(), ended.describe()),
        };
    }

    // A spent model grant refuses completions and nothing else. An agent halfway through a compile
    // is not stopped for running out of money it was not spending.
    if let Reach::UseModel { class } = reach
        && !lease.may_use_model(class)
    {
        return Verdict::NotGranted {
            wanted: if lease.model_spent_out() {
                format!("a {class} model, and this capsule has spent its model budget")
            } else {
                format!("a {class} model")
            },
        };
    }

    verdict
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use time::Duration;
    use uuid::Uuid;

    use super::*;
    use crate::grant::{ModelGrant, NetworkGrant, ResourceBudget, Workspace};

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
                tasks_max: 512,
                lifetime: Duration::hours(4),
            },
            model: Some(ModelGrant {
                class: "Strong".to_owned(),
                spend_limit: 100,
            }),
            tools: vec!["git".to_owned()],
            may_execute: true,
        }
    }

    fn reading() -> Reach {
        Reach::ReadPath {
            path: PathBuf::from("/srv/project/src/main.rs"),
        }
    }

    fn asking_a_model() -> Reach {
        Reach::UseModel {
            class: "Strong".to_owned(),
        }
    }

    const FOUR_HOURS: i64 = 4 * 60 * 60;

    #[test]
    fn a_live_lease_permits_what_the_grant_permits() {
        let lease = Lease::issued(grant(), at(0));
        assert!(lease.is_live(at(60)));
        assert!(decide_under_lease(&lease, &reading(), at(60)).is_allowed());
    }

    #[test]
    fn a_capsule_that_needs_no_model_is_not_dead_before_it_starts() {
        // The defect this split closes. One ceiling folded into the resource budget meant a zero
        // limit read as an exhausted one, so a capsule with no model business — and every capsule on
        // an unplugged host, which is the configuration this system exists to survive — was finished
        // at the instant it was issued.
        let mut without = grant();
        without.model = None;
        let lease = Lease::issued(without, at(0));

        assert!(
            lease.is_live(at(60)),
            "a capsule with no model grant was finished"
        );
        assert!(decide_under_lease(&lease, &reading(), at(60)).is_allowed());
        assert_eq!(lease.remaining_spend(), None);
        assert!(
            !lease.model_spent_out(),
            "having no model grant is not having spent one"
        );
        assert!(!decide_under_lease(&lease, &asking_a_model(), at(60)).is_allowed());
    }

    #[test]
    fn a_free_model_with_a_zero_ceiling_is_still_a_grant() {
        // Zero means: use something that costs nothing, and run up no bill. A real configuration,
        // and not the same as having no grant at all.
        let mut free = grant();
        free.model = Some(ModelGrant {
            class: "Local".to_owned(),
            spend_limit: 0,
        });
        let lease = Lease::issued(free, at(0));

        assert!(lease.is_live(at(60)), "the capsule is fine");
        assert!(lease.model_spent_out(), "there is nothing to spend");
        assert_eq!(lease.remaining_spend(), Some(0));
    }

    #[test]
    fn spending_out_refuses_completions_and_stops_nothing_else() {
        // The other half of the split. An agent halfway through a compile is not frozen for running
        // out of money it was not spending.
        let mut lease = Lease::issued(grant(), at(0));
        lease.charge(100);

        assert!(lease.is_live(at(60)), "the capsule is still running");
        assert!(decide_under_lease(&lease, &reading(), at(60)).is_allowed());
        assert!(
            decide_under_lease(
                &lease,
                &Reach::RunProgram {
                    program: "cargo".to_owned()
                },
                at(60)
            )
            .is_allowed()
        );

        match decide_under_lease(&lease, &asking_a_model(), at(60)) {
            Verdict::NotGranted { wanted } => {
                assert!(wanted.contains("spent its model budget"), "{wanted}");
            }
            other => panic!("a spent-out model grant produced {other:?}"),
        }
    }

    #[test]
    fn a_lease_that_ran_out_permits_nothing_it_used_to() {
        // The point of giving a grant an end. Four hours to the second is over.
        let lease = Lease::issued(grant(), at(0));
        assert!(lease.is_live(at(FOUR_HOURS - 1)));
        assert_eq!(lease.ended(at(FOUR_HOURS)), Some(Ended::Expired));
        assert!(!decide_under_lease(&lease, &reading(), at(FOUR_HOURS)).is_allowed());
    }

    #[test]
    fn a_ceiling_is_reached_at_the_ceiling_and_not_one_unit_past_it() {
        // `>` would let every ceiling be exceeded by exactly one unit, which is the kind of error
        // that is invisible until somebody adds up a month of them.
        let mut lease = Lease::issued(grant(), at(0));
        lease.charge(99);
        assert!(lease.may_use_model("Strong"), "99 of 100 is not spent out");
        assert_eq!(lease.remaining_spend(), Some(1));

        lease.charge(1);
        assert!(!lease.may_use_model("Strong"));
        assert!(lease.model_spent_out());
        assert_eq!(lease.remaining_spend(), Some(0));
    }

    #[test]
    fn a_cost_larger_than_the_whole_ceiling_exhausts_it_rather_than_wrapping() {
        // Saturating, so no arithmetic here can make a lease look healthier than it is.
        let mut lease = Lease::issued(grant(), at(0));
        lease.charge(u64::MAX);
        lease.charge(u64::MAX);
        assert_eq!(lease.model_spent, u64::MAX);
        assert!(lease.model_spent_out());
        assert_eq!(lease.remaining_spend(), Some(0));
    }

    #[test]
    fn a_model_class_the_grant_does_not_name_is_refused_however_much_budget_is_left() {
        let lease = Lease::issued(grant(), at(0));
        assert!(!lease.may_use_model("Fast"));
        assert!(
            !decide_under_lease(
                &lease,
                &Reach::UseModel {
                    class: "Fast".to_owned()
                },
                at(60)
            )
            .is_allowed()
        );
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
        assert_eq!(lease.ended(at(FOUR_HOURS + 1)), Some(Ended::Revoked));
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
    fn the_two_endings_are_told_apart() {
        // Only one of them is something a person did. A surface reporting them identically would
        // tell an operator their agent ran out of time when they stopped it.
        let expired = Lease::issued(grant(), at(0));
        let mut revoked = Lease::issued(grant(), at(0));
        revoked.revoke(at(10));

        assert_eq!(expired.ended(at(FOUR_HOURS)), Some(Ended::Expired));
        assert_eq!(revoked.ended(at(60)), Some(Ended::Revoked));
        assert_ne!(Ended::Expired.describe(), Ended::Revoked.describe());
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
