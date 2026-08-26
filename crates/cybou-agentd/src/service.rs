// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The `org.cybou.Runtime.Agent1` surface: what is running, and how to stop it.
//!
//! ## Read and stop, and deliberately not launch
//!
//! A CLI launch is already bounded by who can run it: whoever invokes `cybou-agentd launch` is
//! `cybou` on this host. Putting `Launch` on the bus removes that, because any process under the same
//! UID could then ask for a capsule — and the only thing left standing between such a request and a
//! real grant would be the profile it names.
//!
//! That is not an argument against ever offering it. It is an argument that `Launch` arrives with a
//! registry of operator-approved profiles the owner reads *itself*, and takes a profile id rather
//! than a set of ceilings. An endpoint that accepted memory, CPUs, hosts and a lifetime from its
//! caller would be asking the caller to invent its own `CapsuleGrant`, which is the one thing a
//! capability profile exists to prevent.
//!
//! So this surface answers what is running and ends what a person asks it to end. Both are safe in a
//! way `Launch` is not: neither can widen anything, and stopping is the direction that removes
//! authority rather than granting it.
//!
//! ## Stopping is stopping units, not asking an agent
//!
//! `Stop` runs the session's teardown. It does not send anything to the agent and does not wait for
//! it to agree — the capsule is a cgroup with a kill switch, and a boundary made of requests is not
//! one.
//!
//! ## A session leaves the registry when it is gone, not when it was asked to go
//!
//! The order matters and the earlier version had it wrong. It took the session out of the registry
//! first and tore it down afterwards, so a capsule that refused to die was a capsule this owner had
//! forgotten: absent from every listing, with no surface offering to stop it, and an agent still
//! working inside it. The one failure a person could not recover from was the one reported as
//! success.
//!
//! So: the reason is fixed, the units are terminated, the host is asked whether they are actually
//! gone, and only then is the session forgotten. An ending that cannot be confirmed leaves the
//! session listed and answers `false`, which is the truthful thing to tell a caller whose request
//! did not take effect.
//!
//! The reason is fixed **before** the teardown, because a session torn down first and labelled
//! afterwards could be marked expired if the clock ran out in between, which would replace a
//! person's decision with a timer. Where that record then goes — a listing of finished sessions a
//! person can still read — is not built, and until it is, a stopped session simply stops being
//! listed.

#![allow(missing_docs)]

use std::sync::{Arc, Mutex};

use cybou_fabric::encode;
use time::OffsetDateTime;
use uuid::Uuid;
use zbus::{fdo, interface};

use crate::registry::SessionRegistry;
use crate::session::SessionEnd;

/// What the surface needs in order to end a session on the host.
///
/// A trait rather than a direct call, so the decision to run teardown stays testable without a
/// service manager, and so this file holds no policy about how a unit is stopped.
pub trait Teardown: Send + Sync {
    /// End everything one session put on the host, and say whether it ended.
    ///
    /// The answer is not decoration. A session removed from the registry on the strength of having
    /// *asked* would be a capsule still running that this owner has forgotten — no listing showing
    /// it, no surface offering to stop it, and an agent working on inside. So the caller is told,
    /// and an unproven ending keeps the session where a person can still see it.
    fn tear_down(&self, plan: &crate::plan::SessionPlan) -> Ended;
}

/// Whether a session's units are actually gone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ended {
    /// Every unit this session started is confirmed inactive.
    Confirmed,
    /// Something is still running, or could not be asked about.
    ///
    /// Not a failure to report and forget. It is the case where the truthful thing is to keep
    /// showing the session, because it is still there.
    Unproven,
}

/// Process-owned Agent1 dispatch surface.
pub struct Agent1Service {
    registry: Arc<Mutex<SessionRegistry>>,
    teardown: Arc<dyn Teardown>,
}

impl Agent1Service {
    /// Serve this registry.
    #[must_use]
    pub fn new(registry: Arc<Mutex<SessionRegistry>>, teardown: Arc<dyn Teardown>) -> Self {
        Self { registry, teardown }
    }
}

#[allow(clippy::unused_async, reason = "zbus handlers are futures")]
#[interface(name = "org.cybou.Runtime.Agent1")]
impl Agent1Service {
    async fn ready(&self) -> bool {
        true
    }

    /// Every session this owner holds, as a surface should show it.
    async fn sessions(&self) -> fdo::Result<Vec<u8>> {
        let views = self
            .registry
            .lock()
            .map_err(|_| fdo::Error::Failed("the session registry is unavailable".to_owned()))?
            .views();
        encode(&views).map_err(|error| fdo::Error::Failed(error.to_string()))
    }

    /// One session, by the capsule identity that is also its own.
    async fn session(&self, capsule_id: String) -> fdo::Result<Vec<u8>> {
        let capsule_id = identity(&capsule_id)?;
        let registry = self
            .registry
            .lock()
            .map_err(|_| fdo::Error::Failed("the session registry is unavailable".to_owned()))?;
        let view = registry
            .views()
            .into_iter()
            .find(|view| view.capsule_id == capsule_id)
            .ok_or_else(|| fdo::Error::FileNotFound("no such session".to_owned()))?;
        encode(&view).map_err(|error| fdo::Error::Failed(error.to_string()))
    }

    /// End one session now.
    ///
    /// Idempotent in the way that matters: a second Stop on a session this owner no longer holds is
    /// not an error worth reporting differently from the first, because in both cases the session is
    /// over by the time the caller is answered.
    async fn stop(&self, capsule_id: String) -> fdo::Result<bool> {
        let capsule_id = identity(&capsule_id)?;
        let now = OffsetDateTime::now_utc();

        // Marked as ending while it is still held, so a listing taken during the teardown shows a
        // session on its way out rather than one that is fine.
        let plan = {
            let mut registry = self.registry.lock().map_err(|_| {
                fdo::Error::Failed("the session registry is unavailable".to_owned())
            })?;
            let Some(live) = registry.get_mut(capsule_id) else {
                return Ok(false);
            };
            live.session.begin_ending(SessionEnd::Stopped);
            live.plan.clone()
        };

        if self.teardown.tear_down(&plan) == Ended::Unproven {
            // Still there. Keeping it listed is the whole point: a forgotten capsule is one nobody
            // can be shown and nobody can be offered a way to stop.
            return Ok(false);
        }

        let mut registry = self
            .registry
            .lock()
            .map_err(|_| fdo::Error::Failed("the session registry is unavailable".to_owned()))?;
        if let Some(mut live) = registry.take(capsule_id) {
            live.session.finish_ending(now);
        }
        Ok(true)
    }
}

fn identity(value: &str) -> fdo::Result<Uuid> {
    Uuid::parse_str(value)
        .map_err(|_| fdo::Error::InvalidArgs("a session is named by its capsule UUID".to_owned()))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use cybou_capsule::{
        CapabilityProfile, LeaseRequest, ModelGrant, NetworkGrant, ResourceBudget, SpendPolicy,
        Workspace, issue_lease,
    };
    use time::Duration;

    use super::*;
    use crate::plan::{Ceilings, Launch, SessionPlan, plan};
    use crate::registry::LiveSession;
    use crate::session::Session;

    #[derive(Default)]
    struct CountingTeardown(AtomicUsize);

    impl Teardown for CountingTeardown {
        fn tear_down(&self, _plan: &SessionPlan) -> Ended {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ended::Confirmed
        }
    }

    /// A host where the capsule will not die.
    #[derive(Default)]
    struct StubbornTeardown(AtomicUsize);

    impl Teardown for StubbornTeardown {
        fn tear_down(&self, _plan: &SessionPlan) -> Ended {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ended::Unproven
        }
    }

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    const CAPSULE: Uuid = Uuid::from_u128(0xf001);

    fn held() -> LiveSession {
        let mut profile = CapabilityProfile::bounded(
            "sandboxed-autonomous",
            ResourceBudget {
                memory_mib: 4096,
                cpus: 2,
                tasks_max: 512,
                lifetime: Duration::hours(4),
            },
        )
        .expect("a valid profile");
        profile.network = NetworkGrant::to(&["github.com"]);
        profile.model = Some(ModelGrant {
            class: "Strong".to_owned(),
            spend: SpendPolicy::Capped(100),
        });
        profile.may_execute = true;
        let lease = issue_lease(
            LeaseRequest {
                selected_profile: profile,
                capsule_id: CAPSULE,
                agent: "opencode".to_owned(),
                workspace: Workspace::at("/srv/project"),
            },
            at(0),
        )
        .expect("a lease is issued");

        let plan = plan(
            &Launch {
                lease,
                task_id: Uuid::from_u128(0xf002),
                ceilings: Ceilings {
                    token_limit: 1000,
                    max_output_tokens: 32,
                    sensitivity: 1,
                },
            },
            at(1),
        )
        .expect("a plan");

        let mut session = Session::launching(CAPSULE, at(0));
        session.running().expect("it came up");
        LiveSession {
            plan,
            session,
            ledger: crate::view::Ledger::Elsewhere,
        }
    }

    fn serving() -> (
        Agent1Service,
        Arc<CountingTeardown>,
        Arc<Mutex<SessionRegistry>>,
    ) {
        let mut registry = SessionRegistry::new();
        registry.insert(held());
        let registry = Arc::new(Mutex::new(registry));
        let teardown = Arc::new(CountingTeardown::default());
        (
            Agent1Service::new(
                Arc::clone(&registry),
                Arc::clone(&teardown) as Arc<dyn Teardown>,
            ),
            teardown,
            registry,
        )
    }

    #[tokio::test]
    async fn stopping_a_session_tears_it_down_once_and_stops_listing_it() {
        let (service, teardown, registry) = serving();
        assert_eq!(registry.lock().expect("held").len(), 1);

        assert!(service.stop(CAPSULE.to_string()).await.expect("stops"));
        assert_eq!(teardown.0.load(Ordering::SeqCst), 1);
        assert!(
            registry.lock().expect("held").is_empty(),
            "the registry answers what is running"
        );
    }

    #[tokio::test]
    async fn stopping_something_already_gone_tears_nothing_down_again() {
        // A second Stop is not an error worth reporting differently: in both cases the session is
        // over by the time the caller is answered, and tearing down twice would run unit stops
        // against names that now belong to nothing.
        let (service, teardown, _) = serving();
        assert!(service.stop(CAPSULE.to_string()).await.expect("stops"));
        assert!(!service.stop(CAPSULE.to_string()).await.expect("answers"));

        assert_eq!(teardown.0.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_session_that_would_not_die_stays_listed_and_the_caller_is_told() {
        // The failure a person could not recover from, previously reported as success. A capsule
        // removed from the registry on the strength of having been asked to stop is one nobody can
        // be shown and nobody can be offered a way to stop.
        let mut registry = SessionRegistry::new();
        registry.insert(held());
        let registry = Arc::new(Mutex::new(registry));
        let teardown = Arc::new(StubbornTeardown::default());
        let service = Agent1Service::new(
            Arc::clone(&registry),
            Arc::clone(&teardown) as Arc<dyn Teardown>,
        );

        assert!(
            !service.stop(CAPSULE.to_string()).await.expect("answers"),
            "an ending that was not confirmed is not a stop"
        );
        assert_eq!(teardown.0.load(Ordering::SeqCst), 1);
        assert_eq!(
            registry.lock().expect("held").len(),
            1,
            "the session is still there, so it is still listed"
        );

        // And it shows as on its way out rather than as fine, because that is what it is.
        let views = registry.lock().expect("held").views();
        assert_eq!(views[0].standing, crate::view::Standing::Ending);
        assert_eq!(
            views[0].ended_because.as_deref(),
            Some("the session was stopped")
        );
    }

    #[tokio::test]
    async fn a_session_is_named_by_its_capsule_and_nothing_else_is_accepted() {
        let (service, teardown, _) = serving();
        assert!(service.stop("the-session".to_owned()).await.is_err());
        assert!(service.session("the-session".to_owned()).await.is_err());
        assert_eq!(teardown.0.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn a_listing_holds_grants_and_never_ledgers() {
        // This owner holds no ledger: the model gateway charges its own copy of every lease. A
        // listing reporting nought would state, of every session on it, that nothing was spent.
        let (service, _, _) = serving();
        let encoded = service.sessions().await.expect("a listing");
        let views: Vec<crate::view::SessionView> = cybou_fabric::decode(&encoded).expect("decodes");

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].capsule_id, CAPSULE);
        match views[0].spend {
            Some(crate::view::SpendView::Capped { spent, .. }) => assert_eq!(spent, None),
            other => panic!("a capped grant showed as {other:?}"),
        }
    }
}
