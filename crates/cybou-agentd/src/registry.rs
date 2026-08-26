// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What is running, and how an owner finds that out again after it has itself restarted.
//!
//! A capsule and a gateway are built on purpose to outlive the thing that started them, up to their
//! own hard deadlines — that is what makes the boundary hold without a coordinator being alive. The
//! consequence is that an owner which came back and reported *no running agents* would be wrong
//! about the host in the direction that matters most: an agent would still be working, inside a
//! capsule nobody was watching, with no surface offering a person a way to stop it.
//!
//! So the registry is not a memory of what this process started. It is a reading of what is on the
//! host.
//!
//! ## The host is the record, and the lease is why that is safe
//!
//! Everything needed to describe a session is already written down where the session put it: the
//! lease carries the whole grant, and the launch file carries the task and its token ceilings. A
//! recovering owner reads them back and re-derives the plan through exactly the same function a
//! launch used, so a recovered session and a fresh one are the same object rather than two
//! descriptions that agree today.
//!
//! ## What is not running is over, and its reason is not recoverable
//!
//! This is the one judgement here, and it is deliberately a small one. If the capsule unit is gone,
//! the session is over — but *why* it ended was known only to the owner that died with it. Recovery
//! does not guess. It does not resurrect the session, it does not invent a verdict, and it does not
//! report an agent that finished as one that was stopped. It hands back the plan so the leftovers can
//! be torn down, which is the one thing still worth doing about a session nobody can describe.
//!
//! A lease that has run out is a different case and needs no judgement: the clock says so, and the
//! clock is the same one that ended the capsule.

use std::collections::BTreeMap;

use time::OffsetDateTime;
use uuid::Uuid;

use cybou_capsule::Lease;

use crate::capacity::{HostCapacity, NotAdmitted, Reserved, admits};
use crate::plan::{CannotPlan, Ceilings, Launch, SessionPlan, plan};
use crate::session::{Session, SessionEnd};
use crate::view::Ledger;
use cybou_protocol::agent::SessionView;

/// One session found on the host during recovery.
///
/// The two files are read as they were written, and whether the capsule is still up is asked of the
/// service manager rather than inferred from either of them. A file is what a launch intended; a
/// running unit is what is actually true now.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    /// The authoritative lease, read back from this session's lease file.
    pub lease: Lease,
    /// The task the model bearer was for, from the launch file.
    pub task_id: Uuid,
    /// The per-token ceilings, from the launch file.
    pub ceilings: Ceilings,
    /// Whether this session's capsule unit is still active, as the service manager reports it.
    pub capsule_active: bool,
}

/// What one session was launched as, and where it has got to.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveSession {
    /// Everything the launch implied, re-derived rather than remembered.
    pub plan: SessionPlan,
    /// What has happened to it.
    pub session: Session,
    /// What its gateway had spent when last read, if it has ever been read.
    ///
    /// Held beside the session rather than fetched when a listing is built, because reading a file
    /// per session on every call would put filesystem work on the path a surface refreshes. The
    /// snapshot carries the instant it was taken, so a reading that is a little old says so.
    pub ledger: Ledger,
}

/// Sessions this owner is holding.
///
/// Ordered by capsule identity rather than by arrival, so a listing is stable across restarts and a
/// surface does not reorder itself for reasons a person cannot see.
#[derive(Debug, Default)]
pub struct SessionRegistry {
    sessions: BTreeMap<Uuid, LiveSession>,
}

/// What a recovery pass concluded.
#[derive(Debug, Default)]
pub struct Recovered {
    /// Sessions still running, which this owner now holds.
    pub registry: SessionRegistry,
    /// Plans whose capsule is gone. Nothing is claimed about why; they are here to be torn down.
    pub orphaned: Vec<SessionPlan>,
    /// Sessions found on the host that could not be read back at all.
    ///
    /// Reported rather than skipped. A launch file this build cannot re-derive is either a defect or
    /// a session from a different version, and both are things an operator should be told about
    /// rather than have quietly vanish from a list of what is running.
    pub unreadable: Vec<(Uuid, CannotPlan)>,
}

impl SessionRegistry {
    /// An owner holding nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Hold one session.
    pub fn insert(&mut self, live: LiveSession) {
        self.sessions
            .insert(live.plan.lease.grant().capsule_id, live);
    }

    /// Hold one session if this host can still promise what it was granted.
    ///
    /// Deciding and taking are one call, and that is the whole of why this lives here rather than
    /// beside the arithmetic. Two callers that each ask *is there room* and then each take it are
    /// both told yes, and a host that answered them separately would be oversubscribed by exactly
    /// the sessions it admitted correctly.
    ///
    /// Admission is against what has been *promised*, not what is being used. A session admitted
    /// because the others happen to be idle is a promise the host cannot keep the moment they are
    /// not.
    ///
    /// # Errors
    ///
    /// Returns the [`NotAdmitted`] naming the first limit the total would cross. Nothing is inserted
    /// in that case, so a refused launch leaves the registry exactly as it was.
    pub fn admit(&mut self, capacity: HostCapacity, live: LiveSession) -> Result<(), NotAdmitted> {
        let already = Reserved::across(self.sessions.values().map(|held| held.plan.lease.grant()));
        admits(capacity, already, live.plan.lease.grant())?;
        self.insert(live);
        Ok(())
    }

    /// What this host has already promised across every session it holds.
    #[must_use]
    pub fn reserved(&self) -> Reserved {
        Reserved::across(self.sessions.values().map(|held| held.plan.lease.grant()))
    }

    /// One session, if it is held.
    #[must_use]
    pub fn get(&self, capsule_id: Uuid) -> Option<&LiveSession> {
        self.sessions.get(&capsule_id)
    }

    /// One session, mutably.
    pub fn get_mut(&mut self, capsule_id: Uuid) -> Option<&mut LiveSession> {
        self.sessions.get_mut(&capsule_id)
    }

    /// Stop holding one session and hand back its plan, so its teardown can be run.
    pub fn take(&mut self, capsule_id: Uuid) -> Option<LiveSession> {
        self.sessions.remove(&capsule_id)
    }

    /// How many sessions are held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Whether nothing is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// Every held session, as a surface should show it.
    ///
    /// Whatever [`Self::read_ledgers`] last read for each, which is [`Ledger::Elsewhere`] until
    /// something has. Never a nought this owner invented: it holds the grant a person approved and
    /// the gateway holds the ledger, so a registry reporting nought would be stating, of every
    /// session on it, that nothing has been spent.
    #[must_use]
    pub fn views(&self) -> Vec<SessionView> {
        self.sessions
            .values()
            .map(|live| crate::view::of(&live.session, &live.plan, live.ledger))
            .collect()
    }

    /// Re-read the ledger each held session's gateway published.
    ///
    /// The reader is supplied rather than called directly, so the judgement of what a published
    /// ledger means stays here and the filesystem stays out of it. A session whose gateway has
    /// published nothing keeps whatever was last read — an absent file is a gateway that has not
    /// written yet, not a session that has spent nothing, and overwriting a real figure with a nought
    /// because a read failed would be the same lie in a new place.
    ///
    /// A snapshot naming a different capsule is not read. It says which session it is about, and
    /// believing one that says a different one would attribute another session's spending to this
    /// one — from a path left behind by an earlier session, or simply from the wrong file.
    pub fn read_ledgers(
        &mut self,
        read: impl Fn(&std::path::Path) -> Option<cybou_protocol::model::ModelUsageSnapshot>,
    ) {
        for live in self.sessions.values_mut() {
            let Some(path) = live.plan.model_usage.as_ref() else {
                continue;
            };
            if let Some(snapshot) = read(path)
                && snapshot.capsule_id == live.plan.lease.grant().capsule_id
            {
                live.ledger = Ledger::published(&snapshot);
            }
        }
    }

    /// Ask every held session's lease whether it is over, and begin ending the ones that are.
    ///
    /// The lease's clock, not one kept here. Returns the sessions whose ending began on this call,
    /// so a caller runs each teardown exactly once.
    pub fn expire(&mut self, now: OffsetDateTime) -> Vec<Uuid> {
        let mut ended = Vec::new();
        for (capsule_id, live) in &mut self.sessions {
            let lease = live.plan.lease.clone();
            if live.session.observe(&lease, now) {
                ended.push(*capsule_id);
            }
        }
        ended
    }
}

/// Rebuild what is running from what was found on the host.
///
/// Each session is re-derived through the same [`plan`] a launch used, so a recovered session and a
/// fresh one are the same object rather than two descriptions that happen to agree.
#[must_use]
pub fn recover(found: Vec<Found>, now: OffsetDateTime) -> Recovered {
    let mut out = Recovered::default();

    for entry in found {
        let capsule_id = entry.lease.grant().capsule_id;
        let launch = Launch {
            lease: entry.lease,
            task_id: entry.task_id,
            ceilings: entry.ceilings,
        };
        // Planned against the launch's own instant rather than now, because a plan derived at a
        // later time would refuse every session whose lease has since run out — and those are
        // exactly the ones whose leftovers still need a plan to tear them down with.
        let issued = launch.lease.issued_at();
        let plan = match plan(&launch, issued) {
            Ok(plan) => plan,
            Err(why) => {
                out.unreadable.push((capsule_id, why));
                continue;
            }
        };

        // The service manager's word, not a file's. A lease file says what a launch intended; a
        // running unit is what is true now.
        if !entry.capsule_active {
            out.orphaned.push(plan);
            continue;
        }

        let mut session = Session::launching(capsule_id, issued);
        // A capsule that is up is running. Nothing here can tell whether its agent is mid-thought or
        // idle, and claiming either would be this module inventing an observation.
        let _ = session.running();
        if session.observe(&plan.lease, now) {
            // Its lease ran out while nobody was holding it. The capsule is on borrowed time its own
            // unit deadline will end; the session is over as far as any permission is concerned, and
            // saying so is what lets a teardown happen at all.
            session.finish_ending(now);
            out.orphaned.push(plan);
            continue;
        }
        // Nothing is claimed about spending until a gateway's own ledger has been read.
        out.registry.insert(LiveSession {
            plan,
            session,
            ledger: Ledger::Elsewhere,
        });
    }
    out
}

/// The one reason recovery may attribute to a session, and it is not a guess.
///
/// Used when a lease is found to have run out. Every other ending — an agent that finished, a person
/// who stopped it, a launch that fell over — was known only to the owner that died, and recovery
/// reports those as nothing at all rather than as the nearest plausible reason.
#[must_use]
pub const fn recovered_ending() -> SessionEnd {
    SessionEnd::Expired
}

#[cfg(test)]
mod tests {
    use cybou_capsule::{
        CapabilityProfile, LeaseRequest, ModelGrant, NetworkGrant, ResourceBudget, SpendPolicy,
        Workspace, issue_lease,
    };
    use time::Duration;

    use super::*;
    use crate::view::Standing;

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    fn lease(capsule: u128, lifetime: Duration) -> Lease {
        let mut profile = CapabilityProfile::bounded(
            "sandboxed-autonomous",
            ResourceBudget {
                memory_mib: 4096,
                cpus: 2,
                tasks_max: 512,
                lifetime,
            },
        )
        .expect("a valid profile");
        profile.network = NetworkGrant::to(&["github.com"]);
        profile.model = Some(ModelGrant {
            class: "Strong".to_owned(),
            spend: SpendPolicy::Capped(100),
        });
        profile.may_execute = true;
        issue_lease(
            LeaseRequest {
                selected_profile: profile,
                capsule_id: Uuid::from_u128(capsule),
                agent: "opencode".to_owned(),
                workspace: Workspace::at("/srv/project"),
            },
            at(0),
        )
        .expect("a lease is issued")
    }

    fn found(capsule: u128, lifetime: Duration, capsule_active: bool) -> Found {
        Found {
            lease: lease(capsule, lifetime),
            task_id: Uuid::from_u128(0xd002),
            ceilings: Ceilings {
                token_limit: 1000,
                max_output_tokens: 32,
                sensitivity: 1,
            },
            capsule_active,
        }
    }

    #[test]
    fn an_owner_that_restarted_finds_the_agent_that_kept_working() {
        // The failure this exists to prevent. A capsule outlives its coordinator on purpose, so an
        // owner reporting nothing running would leave a working agent with no surface to stop it.
        let recovered = recover(vec![found(0xd001, Duration::hours(4), true)], at(600));

        assert_eq!(recovered.registry.len(), 1);
        assert!(recovered.orphaned.is_empty());
        let view = &recovered.registry.views()[0];
        assert_eq!(view.standing, Standing::Running);
        assert_eq!(view.agent, "opencode");
        assert!(view.is_live());
    }

    #[test]
    fn a_recovered_session_is_the_same_object_a_launch_would_have_produced() {
        // Re-derived through the same plan(), so recovery cannot drift from launch by agreeing with
        // it today and disagreeing after somebody changes one of them.
        let entry = found(0xd001, Duration::hours(4), true);
        let expected = plan(
            &Launch {
                lease: entry.lease.clone(),
                task_id: entry.task_id,
                ceilings: entry.ceilings,
            },
            entry.lease.issued_at(),
        )
        .expect("a plan");

        let recovered = recover(vec![entry], at(600));
        let live = recovered
            .registry
            .get(Uuid::from_u128(0xd001))
            .expect("held");
        assert_eq!(live.plan, expected);
    }

    #[test]
    fn a_session_whose_capsule_is_gone_is_over_and_gets_no_invented_reason() {
        // Why it ended was known only to the owner that died with it. Reporting an agent that
        // finished as one that was stopped would put a person's decision where there was none.
        let recovered = recover(vec![found(0xd001, Duration::hours(4), false)], at(600));

        assert!(recovered.registry.is_empty());
        assert_eq!(
            recovered.orphaned.len(),
            1,
            "its leftovers still need clearing"
        );
        assert_eq!(
            recovered.orphaned[0].lease.grant().capsule_id,
            Uuid::from_u128(0xd001)
        );
    }

    #[test]
    fn a_lease_that_ran_out_while_nobody_watched_is_recovered_only_to_be_torn_down() {
        // The clock needs no judgement: it is the same one that ends the capsule's own unit.
        let recovered = recover(vec![found(0xd001, Duration::minutes(5), true)], at(60 * 60));

        assert!(recovered.registry.is_empty());
        assert_eq!(recovered.orphaned.len(), 1);
        assert_eq!(recovered_ending(), SessionEnd::Expired);
    }

    #[test]
    fn what_cannot_be_read_back_is_reported_rather_than_vanishing_from_the_list() {
        // A launch this build cannot re-derive is a defect or a session from another version. Both
        // are things an operator should be told, not things that quietly stop existing.
        let mut broken = found(0xd001, Duration::hours(4), true);
        broken.ceilings.token_limit = 0;

        let recovered = recover(vec![broken], at(600));
        assert!(recovered.registry.is_empty());
        assert_eq!(recovered.unreadable.len(), 1);
        assert_eq!(recovered.unreadable[0].0, Uuid::from_u128(0xd001));
    }

    #[test]
    fn a_listing_is_ordered_by_identity_so_it_does_not_reshuffle_between_restarts() {
        // A surface that reordered itself for reasons a person cannot see is a surface they stop
        // trusting to tell them what changed.
        let recovered = recover(
            vec![
                found(0xd003, Duration::hours(4), true),
                found(0xd001, Duration::hours(4), true),
                found(0xd002, Duration::hours(4), true),
            ],
            at(60),
        );

        let ids: Vec<Uuid> = recovered
            .registry
            .views()
            .iter()
            .map(|view| view.capsule_id)
            .collect();
        assert_eq!(
            ids,
            vec![
                Uuid::from_u128(0xd001),
                Uuid::from_u128(0xd002),
                Uuid::from_u128(0xd003)
            ]
        );
    }

    #[test]
    fn a_registry_holds_grants_and_never_ledgers() {
        // The gateway charges its own copy of every lease. A registry reporting nought would state,
        // of every session it holds, that nothing has been spent.
        let recovered = recover(vec![found(0xd001, Duration::hours(4), true)], at(60));
        let view = &recovered.registry.views()[0];

        match view.spend {
            Some(crate::view::SpendView::Capped { spent, .. }) => {
                assert_eq!(spent, None, "this owner holds no ledger");
            }
            other => panic!("a capped grant showed as {other:?}"),
        }
    }

    fn snapshot(spent: u64, observed: OffsetDateTime) -> cybou_protocol::model::ModelUsageSnapshot {
        cybou_protocol::model::ModelUsageSnapshot {
            capsule_id: Uuid::from_u128(0xd001),
            spend_units: spent,
            tokens: 1234,
            completions: 3,
            observed_at: observed,
        }
    }

    #[test]
    fn a_spend_appears_only_once_a_gateway_has_published_one() {
        // Before that the owner holds the grant and nothing else. Reporting nought would state, of a
        // session that may have been billed, that nothing was spent.
        let mut recovered = recover(vec![found(0xd001, Duration::hours(4), true)], at(60));
        let view = &recovered.registry.views()[0];
        assert!(matches!(
            view.spend,
            Some(crate::view::SpendView::Capped { spent: None, .. })
        ));
        assert_eq!(view.spend_observed_at, None);

        recovered
            .registry
            .read_ledgers(|_| Some(snapshot(42, at(120))));

        let view = &recovered.registry.views()[0];
        assert!(matches!(
            view.spend,
            Some(crate::view::SpendView::Capped {
                spent: Some(42),
                ..
            })
        ));
        assert_eq!(
            view.spend_observed_at,
            Some(at(120)),
            "a figure arrives with the instant somebody looked"
        );
    }

    #[test]
    fn a_ledger_about_a_different_session_is_not_believed() {
        // A snapshot says which capsule it is about. Reading one that names another would attribute
        // somebody else's spending here — from a stale file, or from the wrong path entirely.
        let mut recovered = recover(vec![found(0xd001, Duration::hours(4), true)], at(60));
        let mut elsewhere = snapshot(999, at(120));
        elsewhere.capsule_id = Uuid::from_u128(0xdead);

        recovered.registry.read_ledgers(|_| Some(elsewhere));

        let view = &recovered.registry.views()[0];
        assert!(matches!(
            view.spend,
            Some(crate::view::SpendView::Capped { spent: None, .. })
        ));
        assert_eq!(view.spend_observed_at, None);
    }

    #[test]
    fn a_ledger_that_could_not_be_read_leaves_the_last_one_standing() {
        // An absent or unreadable file is a gateway that has not written yet, not a session that has
        // spent nothing. Overwriting a real figure with a nought because a read failed would be the
        // same lie in a new place.
        let mut recovered = recover(vec![found(0xd001, Duration::hours(4), true)], at(60));
        recovered
            .registry
            .read_ledgers(|_| Some(snapshot(42, at(120))));
        recovered.registry.read_ledgers(|_| None);

        let view = &recovered.registry.views()[0];
        assert!(matches!(
            view.spend,
            Some(crate::view::SpendView::Capped {
                spent: Some(42),
                ..
            })
        ));
        assert_eq!(view.spend_observed_at, Some(at(120)));
    }

    #[test]
    fn expiry_begins_once_per_session_so_a_teardown_runs_once() {
        let mut recovered = recover(vec![found(0xd001, Duration::minutes(5), true)], at(60));
        assert_eq!(recovered.registry.len(), 1);

        let ended = recovered.registry.expire(at(60 * 60));
        assert_eq!(ended, vec![Uuid::from_u128(0xd001)]);
        assert!(
            recovered.registry.expire(at(60 * 60 + 1)).is_empty(),
            "a second pass has nothing left to end"
        );
    }

    #[test]
    fn a_session_is_admitted_only_if_the_host_can_still_promise_it() {
        // Deciding and taking are one call. Two callers each asking whether there is room, and each
        // then taking it, are both told yes — and the host is oversubscribed by exactly the sessions
        // it admitted correctly.
        let capacity = HostCapacity {
            max_sessions: 1,
            memory_mib: 8192,
            cpus: 8,
            tasks_max: 4096,
            spend_units: 1000,
        };
        let mut registry = SessionRegistry::new();

        let first = recover(vec![found(0xd001, Duration::hours(4), true)], at(60))
            .registry
            .take(Uuid::from_u128(0xd001))
            .expect("held");
        assert!(registry.admit(capacity, first).is_ok());

        let second = recover(vec![found(0xd002, Duration::hours(4), true)], at(60))
            .registry
            .take(Uuid::from_u128(0xd002))
            .expect("held");
        assert_eq!(
            registry.admit(capacity, second),
            Err(crate::capacity::NotAdmitted::Sessions { held: 1, limit: 1 })
        );
        assert_eq!(
            registry.len(),
            1,
            "a refused launch leaves the registry exactly as it was"
        );
    }

    #[test]
    fn what_a_host_has_promised_is_the_sum_of_what_it_holds() {
        let mut registry = SessionRegistry::new();
        assert_eq!(registry.reserved(), Reserved::default());

        for capsule in [0xd001, 0xd002] {
            let live = recover(vec![found(capsule, Duration::hours(4), true)], at(60))
                .registry
                .take(Uuid::from_u128(capsule))
                .expect("held");
            registry
                .admit(HostCapacity::unbounded(), live)
                .expect("an unbounded host admits it");
        }

        let reserved = registry.reserved();
        assert_eq!(reserved.sessions, 2);
        assert_eq!(reserved.memory_mib, 8192, "4096 promised twice");
        assert_eq!(reserved.spend_units, 200, "100 promised twice");
    }

    #[test]
    fn a_session_can_be_taken_out_to_be_torn_down() {
        let mut recovered = recover(vec![found(0xd001, Duration::hours(4), true)], at(60));
        let taken = recovered
            .registry
            .take(Uuid::from_u128(0xd001))
            .expect("held");

        assert_eq!(taken.plan.lease.grant().capsule_id, Uuid::from_u128(0xd001));
        assert!(recovered.registry.is_empty());
        assert!(recovered.registry.get(Uuid::from_u128(0xd001)).is_none());
    }
}
