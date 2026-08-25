// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What a person sees about a running agent.
//!
//! Everything here already existed and none of it was gathered anywhere. The lease knows what was
//! granted, the plan knows which units carry it, the session knows what has happened — and a surface
//! wanting to show one card had to reach into three places and decide for itself which facts belong
//! together. Written once, here, so the browser card and the bus surface show the same session
//! rather than two assemblies of it that drift.
//!
//! ## It shows what was granted, not what is being used
//!
//! The distinction is the whole honesty of the card. `4 GB` on this view is the ceiling a person
//! selected and the kernel enforces; it is not a reading of what the capsule currently occupies.
//! Cybou can observe the latter — that is what the telemetry layer is for — and until that
//! observation is actually wired to a capsule's cgroup, showing a number that *looked* like usage
//! would be inventing the one thing a person is watching the card for.
//!
//! So every field below is a fact this crate can stand behind: it came off the approved lease, the
//! compiled spec, or the session's own recorded history. Nothing is estimated and nothing is
//! averaged.
//!
//! ## Spending is the exception, and it is on the lease
//!
//! What a session has spent is real and observed — the gateway charges the lease on every completion
//! — so it is here. It is read from the lease rather than from the agent, for the reason stated
//! wherever it comes up in this repository: an agent reporting its own consumption is the executor
//! grading its own homework.

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use cybou_capsule::{Lease, SpendPolicy};

use crate::plan::SessionPlan;
use crate::session::{Session, SessionEnd, SessionState};

/// How a session reads on a surface.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Standing {
    /// The launch is being carried out.
    Launching,
    /// The agent is working inside its capsule.
    Running,
    /// The ending has begun.
    Ending,
    /// It is over.
    Ended,
}

/// What was granted for money, said in words a surface can print without deciding anything.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum SpendView {
    /// A ceiling, and what remains of it.
    Capped {
        /// The whole ceiling, in the operator's smallest unit.
        limit: u64,
        /// What has been charged so far.
        spent: u64,
    },
    /// No money at all, and only routes that cost none.
    ///
    /// Carries what was charged anyway, because that is the one number worth showing here: under
    /// this policy it should be nought, and anything else means a route that was declared free
    /// billed — which a person selecting `€0` is entitled to see rather than have summarised away.
    ZeroCost {
        /// What was charged despite the policy. Nought when the promise held.
        spent: u64,
    },
}

/// One agent session, as a surface should show it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    /// The capsule's identity, which is the session's.
    pub capsule_id: Uuid,
    /// Which agent is running.
    pub agent: String,
    /// The profile a person selected.
    pub profile: String,
    /// The one directory the agent may change.
    pub workspace: String,
    /// Where the session is.
    pub standing: Standing,
    /// Why it is over, in a person's words, when it is.
    pub ended_because: Option<String>,
    /// How long it has been up, in whole seconds.
    pub uptime_seconds: i64,
    /// How long the lease has left, in whole seconds; zero once it is over.
    pub remaining_seconds: i64,
    /// The model class the lease granted, if any.
    pub model_class: Option<String>,
    /// What may be spent, and what has been.
    pub spend: Option<SpendView>,
    /// Memory ceiling in mebibytes, as granted.
    pub memory_mib: u32,
    /// CPU ceiling, as granted.
    pub cpus: u32,
    /// Process ceiling, as granted.
    pub tasks_max: u32,
    /// Exactly the hosts this capsule may reach.
    pub hosts: Vec<String>,
    /// The units a person can look up in a service manager.
    pub units: Vec<String>,
}

impl SessionView {
    /// Gather one session from the three things that know about it.
    ///
    /// The lease is passed separately from the plan even though the plan carries one, because the
    /// lease is the thing that has been *charged* — the plan's copy is what was minted, and showing
    /// a spend of nought for a session that has spent something would be reading the wrong one.
    #[must_use]
    pub fn of(session: &Session, plan: &SessionPlan, lease: &Lease, now: OffsetDateTime) -> Self {
        let grant = lease.grant();
        let (standing, ended_because) = match session.state() {
            SessionState::Launching => (Standing::Launching, None),
            SessionState::Running => (Standing::Running, None),
            SessionState::Ending(end) => (Standing::Ending, Some(end.describe())),
            SessionState::Ended(end) => (Standing::Ended, Some(end.describe())),
        };

        Self {
            capsule_id: grant.capsule_id,
            agent: grant.agent.clone(),
            profile: lease.profile_id().as_str().to_owned(),
            workspace: grant.workspace.root.display().to_string(),
            standing,
            ended_because,
            uptime_seconds: session.uptime(now).whole_seconds().max(0),
            // Never negative. A lease that ran out has no time left rather than minus four minutes,
            // and a surface should not have to know that a countdown can go through zero.
            remaining_seconds: (plan.expires_at - now).whole_seconds().max(0),
            model_class: grant.model.as_ref().map(|model| model.class.clone()),
            spend: grant.model.as_ref().map(|model| match model.spend {
                SpendPolicy::Capped(limit) => SpendView::Capped {
                    limit,
                    spent: lease.model_spent(),
                },
                SpendPolicy::ZeroCostOnly => SpendView::ZeroCost {
                    spent: lease.model_spent(),
                },
            }),
            memory_mib: grant.budget.memory_mib,
            cpus: grant.budget.cpus,
            tasks_max: grant.budget.tasks_max,
            hosts: grant.network.hosts.clone(),
            units: units(plan),
        }
    }

    /// Whether a person would consider this session live.
    #[must_use]
    pub const fn is_live(&self) -> bool {
        matches!(self.standing, Standing::Launching | Standing::Running)
    }

    /// How long is left, as a duration.
    #[must_use]
    pub const fn remaining(&self) -> Duration {
        Duration::seconds(self.remaining_seconds)
    }
}

/// The units this session put on the host, so a person can look up any of them by name.
///
/// The egress broker is listed only when there is one. A capsule granted no network has no broker,
/// and naming a unit that was never started would send somebody looking for a fault that is a
/// correctly enforced grant.
fn units(plan: &SessionPlan) -> Vec<String> {
    let mut out = vec![
        format!("{}.service", plan.capsule_unit),
        plan.gateway_unit.clone(),
    ];
    if !plan.hosts.is_empty() {
        out.push(format!("{}.service", plan.egress_unit));
    }
    out
}

/// How an ending reads to a person, for a surface that has only the view.
#[must_use]
pub fn describe_end(end: &SessionEnd) -> String {
    end.describe()
}

#[cfg(test)]
mod tests {
    use cybou_capsule::{
        CapabilityProfile, LeaseRequest, ModelGrant, NetworkGrant, ResourceBudget, Workspace,
        issue_lease,
    };

    use super::*;
    use crate::plan::{Ceilings, Launch, plan};

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    const CAPSULE: Uuid = Uuid::from_u128(0x0c01);

    fn lease(hosts: &[&str], spend: Option<SpendPolicy>) -> Lease {
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
        profile.network = NetworkGrant::to(hosts);
        profile.model = spend.map(|spend| ModelGrant {
            class: "Strong".to_owned(),
            spend,
        });
        profile.may_execute = true;
        issue_lease(
            LeaseRequest {
                selected_profile: profile,
                capsule_id: CAPSULE,
                agent: "opencode".to_owned(),
                workspace: Workspace::at("/srv/project"),
            },
            at(0),
        )
        .expect("a lease is issued")
    }

    fn planned(lease: &Lease) -> SessionPlan {
        plan(
            &Launch {
                lease: lease.clone(),
                task_id: Uuid::from_u128(0x0c02),
                ceilings: Ceilings {
                    token_limit: 1000,
                    max_output_tokens: 32,
                    sensitivity: 1,
                },
            },
            at(1),
        )
        .expect("a plan")
    }

    fn running() -> Session {
        let mut session = Session::launching(CAPSULE, at(0));
        session.running().expect("it came up");
        session
    }

    #[test]
    fn a_card_shows_what_was_granted_and_says_where_each_number_came_from() {
        let lease = lease(
            &["github.com", "registry.npmjs.org"],
            Some(SpendPolicy::Capped(100)),
        );
        let view = SessionView::of(&running(), &planned(&lease), &lease, at(600));

        assert_eq!(view.agent, "opencode");
        assert_eq!(view.profile, "sandboxed-autonomous");
        assert_eq!(view.workspace, "/srv/project");
        assert_eq!(view.standing, Standing::Running);
        assert_eq!(view.uptime_seconds, 600);
        assert_eq!(view.remaining_seconds, 4 * 60 * 60 - 600);
        assert_eq!(view.memory_mib, 4096);
        assert_eq!(view.cpus, 2);
        assert_eq!(view.tasks_max, 512);
        assert_eq!(view.hosts, ["github.com", "registry.npmjs.org"]);
        assert_eq!(view.model_class.as_deref(), Some("Strong"));
        assert!(view.is_live());
    }

    #[test]
    fn a_countdown_stops_at_nothing_left_rather_than_going_negative() {
        // A surface should not have to know that a remaining time can pass through zero.
        let lease = lease(&[], Some(SpendPolicy::Capped(100)));
        let view = SessionView::of(&running(), &planned(&lease), &lease, at(5 * 60 * 60));

        assert_eq!(view.remaining_seconds, 0);
    }

    #[test]
    fn what_was_spent_is_read_from_the_lease_and_not_from_the_agent() {
        // The gateway charges the lease on every completion. An agent reporting its own consumption
        // is the executor grading its own homework.
        let minted = lease(&[], Some(SpendPolicy::Capped(100)));
        let plan = planned(&minted);

        let mut charged = minted.clone();
        charged.charge(42);
        let view = SessionView::of(&running(), &plan, &charged, at(60));

        assert_eq!(
            view.spend,
            Some(SpendView::Capped {
                limit: 100,
                spent: 42
            })
        );
        assert_eq!(
            SessionView::of(&running(), &plan, &plan.lease, at(60)).spend,
            Some(SpendView::Capped {
                limit: 100,
                spent: 0
            }),
            "the plan's copy is what was minted, and reading it would report a session that has \
             spent nothing"
        );
    }

    #[test]
    fn a_zero_cost_session_shows_what_it_was_charged_anyway() {
        // Under this policy the number should be nought. Anything else means a route declared free
        // billed, and a person who selected no spending is entitled to see that rather than have it
        // summarised away.
        let mut lease = lease(&[], Some(SpendPolicy::ZeroCostOnly));
        let view = SessionView::of(&running(), &planned(&lease), &lease, at(60));
        assert_eq!(view.spend, Some(SpendView::ZeroCost { spent: 0 }));

        lease.charge(3);
        let broken = SessionView::of(&running(), &planned(&lease), &lease, at(60));
        assert_eq!(broken.spend, Some(SpendView::ZeroCost { spent: 3 }));
    }

    #[test]
    fn a_capsule_with_no_network_names_no_broker() {
        // Naming a unit that was never started would send somebody looking for a fault that is in
        // fact a correctly enforced grant.
        let lease = lease(&[], Some(SpendPolicy::Capped(100)));
        let view = SessionView::of(&running(), &planned(&lease), &lease, at(60));

        assert_eq!(view.units.len(), 2);
        assert!(view.units.iter().all(|unit| !unit.contains("egress")));
        assert!(view.hosts.is_empty());
    }

    #[test]
    fn every_unit_a_session_started_can_be_looked_up_by_name() {
        let lease = lease(&["github.com"], Some(SpendPolicy::Capped(100)));
        let plan = planned(&lease);
        let view = SessionView::of(&running(), &plan, &lease, at(60));

        assert!(
            view.units
                .contains(&format!("{}.service", plan.capsule_unit))
        );
        assert!(view.units.contains(&plan.gateway_unit));
        assert!(
            view.units
                .contains(&format!("{}.service", plan.egress_unit))
        );
    }

    #[test]
    fn an_ended_session_says_why_and_stops_counting() {
        // "You stopped it" and "your time ran out" are different things to tell a person, and the
        // uptime of a finished session is what it ran for rather than how long ago it started.
        let lease = lease(&[], Some(SpendPolicy::Capped(100)));
        let mut session = running();
        session.begin_ending(SessionEnd::Stopped);
        session.finish_ending(at(300));

        let view = SessionView::of(&session, &planned(&lease), &lease, at(9000));
        assert_eq!(view.standing, Standing::Ended);
        assert_eq!(
            view.ended_because.as_deref(),
            Some("the session was stopped")
        );
        assert_eq!(view.uptime_seconds, 300);
        assert!(!view.is_live());
    }

    #[test]
    fn a_view_survives_the_wire() {
        // It travels from whatever holds the session to whatever draws it, and those are different
        // processes by design.
        let lease = lease(&["github.com"], Some(SpendPolicy::ZeroCostOnly));
        let view = SessionView::of(&running(), &planned(&lease), &lease, at(60));

        let encoded = serde_json::to_string(&view).expect("encodes");
        let decoded: SessionView = serde_json::from_str(&encoded).expect("decodes");
        assert_eq!(decoded, view);
    }
}
