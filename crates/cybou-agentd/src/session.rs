// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What has happened to one session, and nothing about what is permitted.
//!
//! This is a report. It is worth saying twice, because a type called `SessionState` sitting in a
//! coordinator is the most natural place in this whole system for a boundary to accidentally move:
//! the moment anything asks it whether an agent may proceed, the capsule's limits have quietly
//! become conditional on a coordinator process being alive and correct.
//!
//! Nothing asks it. The capsule ends at its unit's deadline whether or not this process exists, and
//! the model gateway refuses on the lease's own clock whether or not this process exists. What this
//! module adds is the ability to say *which* of those happened, to a person, afterwards — and the
//! difference between "stopped" and "ran out" is exactly the difference a surface has to show.
//!
//! ## The first ending is the ending
//!
//! A session that was stopped and then expired was stopped, for the same reason
//! [`cybou_capsule::Lease::revoke`] keeps its first instant: the later event is not a second thing
//! that happened, and letting it overwrite the first would replace a person's decision with a timer.

use time::OffsetDateTime;
use uuid::Uuid;

use cybou_capsule::{Ended, Lease};

/// Why a session is over.
///
/// Four, where the lease has two. A lease can only expire or be withdrawn; a session can also end
/// because the agent finished its work, or because the launch never came up — and reporting either
/// of those as an expiry would tell a person their time ran out when it did not.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionEnd {
    /// The lease reached the end of its lifetime.
    Expired,
    /// A person or a policy withdrew the lease.
    Stopped,
    /// The agent's own process finished.
    AgentFinished,
    /// The session never came up, or fell over while running.
    Failed(String),
}

impl SessionEnd {
    /// How this reads to a person.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Expired => "the lease reached the end of its lifetime".to_owned(),
            Self::Stopped => "the session was stopped".to_owned(),
            Self::AgentFinished => "the agent finished".to_owned(),
            Self::Failed(why) => format!("the session failed: {why}"),
        }
    }

    /// The lease ending this session ending corresponds to, if any.
    #[must_use]
    pub const fn from_lease(ended: Ended) -> Self {
        match ended {
            Ended::Expired => Self::Expired,
            Ended::Revoked => Self::Stopped,
        }
    }
}

/// Where a session is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionState {
    /// The launch is being carried out; the agent is not up yet.
    Launching,
    /// The agent is up and working inside its capsule.
    Running,
    /// The ending has begun: the reason is fixed and teardown is under way.
    Ending(SessionEnd),
    /// Teardown finished.
    Ended(SessionEnd),
}

/// Why a reported transition was refused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CannotTransition {
    /// A session that has begun ending cannot be reported as working again.
    AlreadyEnding(SessionEnd),
}

impl core::fmt::Display for CannotTransition {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::AlreadyEnding(end) => {
                write!(
                    formatter,
                    "the session is already ending: {}",
                    end.describe()
                )
            }
        }
    }
}

impl core::error::Error for CannotTransition {}

/// One agent session, as reported.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Session {
    capsule_id: Uuid,
    state: SessionState,
    started_at: OffsetDateTime,
    ended_at: Option<OffsetDateTime>,
    task: Option<cybou_protocol::agent::AgentTaskView>,
}

impl Session {
    /// Begin reporting a session that is being launched.
    #[must_use]
    pub const fn launching(capsule_id: Uuid, at: OffsetDateTime) -> Self {
        Self {
            capsule_id,
            state: SessionState::Launching,
            started_at: at,
            ended_at: None,
            task: None,
        }
    }

    /// Set the current agent task state.
    pub fn set_task(&mut self, task: cybou_protocol::agent::AgentTaskView) {
        self.task = Some(task);
    }

    /// The current agent task state, if one was provided.
    #[must_use]
    pub const fn task(&self) -> Option<&cybou_protocol::agent::AgentTaskView> {
        self.task.as_ref()
    }

    /// Which capsule this session is.
    #[must_use]
    pub const fn capsule_id(&self) -> Uuid {
        self.capsule_id
    }

    /// Where the session is.
    #[must_use]
    pub const fn state(&self) -> &SessionState {
        &self.state
    }

    /// When the launch began.
    #[must_use]
    pub const fn started_at(&self) -> OffsetDateTime {
        self.started_at
    }

    /// When the session finished ending, if it has.
    #[must_use]
    pub const fn ended_at(&self) -> Option<OffsetDateTime> {
        self.ended_at
    }

    /// How long this session has been up.
    ///
    /// Measured to the ending rather than to now once it is over, so a card showing an old session
    /// does not keep counting time it did not run for.
    #[must_use]
    pub fn uptime(&self, now: OffsetDateTime) -> time::Duration {
        self.ended_at.unwrap_or(now) - self.started_at
    }

    /// Report that the agent came up.
    ///
    /// # Errors
    ///
    /// Returns [`CannotTransition::AlreadyEnding`] for a session whose ending has begun. An agent
    /// process that reports itself ready after teardown started is a race, not a running session,
    /// and recording it as one would show a person a live agent that no longer has a capsule.
    pub fn running(&mut self) -> Result<(), CannotTransition> {
        match &self.state {
            SessionState::Launching | SessionState::Running => {
                self.state = SessionState::Running;
                Ok(())
            }
            SessionState::Ending(end) | SessionState::Ended(end) => {
                Err(CannotTransition::AlreadyEnding(end.clone()))
            }
        }
    }

    /// Begin ending, for this reason.
    ///
    /// Idempotent in the reason: the first ending is kept. Returns whether this call is the one that
    /// began the ending, so a caller can run teardown exactly once without tracking that separately.
    pub fn begin_ending(&mut self, reason: SessionEnd) -> bool {
        match self.state {
            SessionState::Launching | SessionState::Running => {
                self.state = SessionState::Ending(reason);
                true
            }
            SessionState::Ending(_) | SessionState::Ended(_) => false,
        }
    }

    /// Report that teardown finished.
    ///
    /// A session that had not begun ending is recorded as having failed, because arriving here
    /// without a reason means something tore the session down without saying why — and inventing a
    /// tidier reason would hide exactly the case worth seeing.
    pub fn finish_ending(&mut self, at: OffsetDateTime) {
        let reason = match &self.state {
            SessionState::Ending(reason) | SessionState::Ended(reason) => reason.clone(),
            SessionState::Launching | SessionState::Running => {
                SessionEnd::Failed("teardown ran with no recorded reason".to_owned())
            }
        };
        self.state = SessionState::Ended(reason);
        if self.ended_at.is_none() {
            self.ended_at = Some(at);
        }
    }

    /// Take the lease's word for whether this session is over.
    ///
    /// The lease's clock, not one kept here. A coordinator with its own timer is a second answer to
    /// when a session ends, and the two disagree the moment one of them is paused, resumed, or
    /// restarted on a host whose clock moved.
    ///
    /// Returns whether this call began the ending.
    pub fn observe(&mut self, lease: &Lease, now: OffsetDateTime) -> bool {
        match lease.ended(now) {
            Some(ended) => self.begin_ending(SessionEnd::from_lease(ended)),
            None => false,
        }
    }

    /// Whether a person would see this session as live.
    #[must_use]
    pub const fn is_live(&self) -> bool {
        matches!(self.state, SessionState::Launching | SessionState::Running)
    }
}

#[cfg(test)]
mod tests {
    use cybou_capsule::{
        CapabilityProfile, LeaseRequest, ModelGrant, ResourceBudget, SpendPolicy, Workspace,
        issue_lease,
    };
    use time::Duration;

    use super::*;

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    fn lease() -> Lease {
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
        profile.model = Some(ModelGrant {
            class: "Strong".to_owned(),
            spend: SpendPolicy::Capped(100),
        });
        profile.may_execute = true;
        issue_lease(
            LeaseRequest {
                selected_profile: profile,
                capsule_id: Uuid::from_u128(0x0704),
                agent: "opencode".to_owned(),
                workspace: Workspace::at("/srv/project"),
            },
            at(0),
        )
        .expect("a lease is issued")
    }

    fn session() -> Session {
        Session::launching(Uuid::from_u128(0x0704), at(0))
    }

    #[test]
    fn a_session_comes_up_and_reports_how_long_it_has_been_up() {
        let mut session = session();
        session.running().expect("it came up");

        assert_eq!(session.state(), &SessionState::Running);
        assert!(session.is_live());
        assert_eq!(session.uptime(at(600)), Duration::minutes(10));
    }

    #[test]
    fn an_agent_that_reports_ready_after_teardown_began_is_not_a_running_session() {
        // A race, and the one that matters: showing a person a live agent whose capsule is gone.
        let mut session = session();
        assert!(session.begin_ending(SessionEnd::Stopped));

        assert_eq!(
            session.running(),
            Err(CannotTransition::AlreadyEnding(SessionEnd::Stopped))
        );
        assert!(!session.is_live());
    }

    #[test]
    fn only_the_first_ending_runs_a_teardown() {
        // The return value is how a caller tears down exactly once. A second signal, an expiry
        // noticed a moment later and the agent's own exit all arrive for the same session.
        let mut session = session();
        session.running().expect("it came up");

        assert!(session.begin_ending(SessionEnd::Stopped));
        assert!(!session.begin_ending(SessionEnd::Expired));
        assert!(!session.begin_ending(SessionEnd::AgentFinished));
        assert_eq!(session.state(), &SessionState::Ending(SessionEnd::Stopped));
    }

    #[test]
    fn a_session_stopped_and_then_expired_was_stopped() {
        // What somebody did outranks what the clock did, exactly as the lease decides it.
        let mut lease = lease();
        let mut session = session();
        session.running().expect("it came up");

        lease.revoke(at(60));
        assert!(session.observe(&lease, at(61)));
        assert!(!session.observe(&lease, at(4 * 60 * 60 + 1)));

        session.finish_ending(at(62));
        assert_eq!(session.state(), &SessionState::Ended(SessionEnd::Stopped));
    }

    #[test]
    fn the_clock_that_ends_a_session_is_the_leases() {
        // Not a duration kept here. Two clocks disagree the first time one of them is restarted.
        let lease = lease();
        let mut session = session();
        session.running().expect("it came up");

        assert!(!session.observe(&lease, at(4 * 60 * 60 - 1)), "still live");
        assert!(session.observe(&lease, lease.expires_at()));
        assert_eq!(session.state(), &SessionState::Ending(SessionEnd::Expired));
    }

    #[test]
    fn an_agent_finishing_is_not_an_expiry() {
        // Four endings rather than two, because "your agent finished" and "your time ran out" are
        // different things to tell a person about the same stopped capsule.
        let mut session = session();
        session.running().expect("it came up");
        session.begin_ending(SessionEnd::AgentFinished);
        session.finish_ending(at(300));

        assert_eq!(
            session.state(),
            &SessionState::Ended(SessionEnd::AgentFinished)
        );
        assert_ne!(
            SessionEnd::AgentFinished.describe(),
            SessionEnd::Expired.describe()
        );
    }

    #[test]
    fn teardown_with_no_reason_is_recorded_as_a_failure() {
        // Arriving here without a reason means something tore a session down without saying why.
        // A tidier default would hide the one case worth looking at.
        let mut session = session();
        session.running().expect("it came up");
        session.finish_ending(at(300));

        match session.state() {
            SessionState::Ended(SessionEnd::Failed(why)) => assert!(why.contains("no recorded")),
            other => panic!("an unexplained teardown was recorded as {other:?}"),
        }
    }

    #[test]
    fn the_instant_a_session_ended_is_not_moved_by_a_later_call() {
        let mut session = session();
        session.begin_ending(SessionEnd::Stopped);
        session.finish_ending(at(120));
        session.finish_ending(at(900));

        assert_eq!(session.ended_at(), Some(at(120)));
        assert_eq!(session.uptime(at(900)), Duration::minutes(2));
    }
}
