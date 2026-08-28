// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! How one agent session reads to a person.
//!
//! Here rather than beside the owner that builds it, for one reason: the owner and the browser must
//! show the same session. A type defined next to the owner and mirrored in the frontend is two
//! descriptions of one thing that agree on the day they are written, and a surface that assembled
//! these facts for itself would be a second answer to *what is running* — the answer that is not the
//! owner's, and therefore the one that is wrong whenever they differ.
//!
//! ## What it says, and what it deliberately does not
//!
//! Every field is a fact somebody can stand behind: what a person granted, what the compiled spec
//! carries, or what the session's own history recorded. The resource figures are **ceilings**, not
//! readings — `memory_mib` is what was promised and what the kernel enforces, never what the capsule
//! currently occupies. Cybou can observe the latter; until that observation is pointed at a capsule's
//! cgroup, a number that merely *looked* like usage would be inventing the one thing a person is
//! watching for.
//!
//! Spending is the exception, and it arrives with the instant it was seen. *Has spent* and *had spent
//! when last observed* are different claims, and only the second is true of anything read out of a
//! published snapshot.
//!
//! ## Instants, not durations
//!
//! A duration is stale the moment it is serialised. Given both ends, a card keeps its own clock
//! honest and the owner sends nothing until something actually changes.

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

/// What a caller may choose when asking the session owner to launch an agent.
///
/// Deliberately carries no resource, network, lifetime, spending or token ceilings. Those are
/// authority and come only from the operator-approved profile the owner reads itself.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LaunchRequest {
    /// Which operator-approved profile to use.
    pub profile: String,
    /// Which agent pack from that profile to run.
    pub agent: String,
    /// The one directory the agent may change.
    pub workspace: String,
    /// Which model class offered by the profile, if the session needs a model.
    pub model_class: Option<String>,
    /// The first bounded task to give the agent.
    pub prompt: String,
}

/// How a session reads on a surface.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Standing {
    /// The launch is being carried out.
    Launching,
    /// The agent is working inside its capsule.
    Running,
    /// The capsule is frozen (cgroup freeze) / paused.
    Paused,
    /// The capsule is quarantined (frozen + network/model egress revoked).
    Quarantined,
    /// The ending has begun.
    Ending,
    /// It is over.
    Ended,
}

/// What was granted for money, and what had been charged when somebody last looked.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum SpendView {
    /// A ceiling, and what has been charged against it when that is known here.
    Capped {
        /// The whole ceiling, in the operator's smallest unit.
        limit: u64,
        /// What has been charged, or `None` when the reporter holds no ledger.
        spent: Option<u64>,
    },
    /// No money at all, and only routes that cost none.
    ///
    /// Carries what was charged anyway, because that is the one number worth showing here: under
    /// this policy it should be nought, and anything else means a route that was declared free
    /// billed — which a person selecting nothing is entitled to see rather than have summarised away.
    ZeroCost {
        /// What was charged despite the policy, or `None` when the reporter holds no ledger.
        spent: Option<u64>,
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
    /// When the launch began.
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    /// When the lease runs out.
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
    /// When the session finished ending, if it has.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub ended_at: Option<OffsetDateTime>,
    /// The model class the lease granted, if any.
    pub model_class: Option<String>,
    /// What may be spent, and what had been when the ledger was last read.
    pub spend: Option<SpendView>,
    /// When that spending figure was observed, if it was.
    ///
    /// Beside the figure rather than folded into it, so a card can say *seen four minutes ago* —
    /// which is the truth — instead of stating a figure about now that nothing here can establish.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub spend_observed_at: Option<OffsetDateTime>,
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
    /// Task state, progress and result, if any prompt was supplied.
    #[serde(default)]
    pub task: Option<AgentTaskView>,
}

/// One model an operator approved for an agent profile.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferedModelView {
    /// Model class (e.g. "Free", "Fast", "Strong").
    pub class: String,
    /// Whether this model requires zero spending.
    pub zero_cost: bool,
    /// Spending cap in currency units / tokens if capped.
    pub spend_limit: Option<u64>,
}

/// One profile an operator approved on this host, as a caller may choose between them.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferedProfileView {
    /// Unique profile identifier.
    pub id: String,
    /// Which agents may run under these bounds (e.g. [`opencode`]).
    pub agents: Vec<String>,
    /// Permitted workspace directory roots (e.g. [`/projects`]).
    pub workspace_roots: Vec<String>,
    /// Memory ceiling in mebibytes.
    pub memory_mib: u32,
    /// CPU ceiling.
    pub cpus: u32,
    /// Process ceiling.
    pub tasks_max: u32,
    /// Maximum session lifetime in seconds.
    pub lifetime_seconds: i64,
    /// Permitted network egress hosts.
    pub hosts: Vec<String>,
    /// Available model classes and spend policies.
    pub models: Vec<OfferedModelView>,
    /// Whether execution inside capsule is allowed.
    pub may_execute: bool,
}

/// The catalogue of offered agent profiles and readiness of the host agent runtime.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentOffersResponse {
    /// Operator-approved profiles.
    pub profiles: Vec<OfferedProfileView>,
    /// State of profile catalogue ("ready", "not-configured", "invalid", "unreadable").
    #[serde(default)]
    pub profiles_state: String,
    /// State of host capacity ("ready", "zero-capacity", "unbounded", "unreadable").
    #[serde(default)]
    pub capacity_state: String,
    /// State of model provider connection ("ready", "not-configured", "unreachable").
    #[serde(default)]
    pub provider_state: String,
    /// Legacy compatibility boolean.
    pub capacity_bounded: bool,
    /// Legacy compatibility boolean.
    pub provider_connected: bool,
}

/// Current task state and execution result of an agent session.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTaskView {
    /// The prompt or task given to the agent.
    pub prompt: String,
    /// Current phase or progress description.
    pub phase: String,
    /// The final answer or summary produced by the agent.
    #[serde(default)]
    pub result: Option<String>,
    /// Any boundary requests refused by the capsule runtime during execution.
    #[serde(default)]
    pub refused_permissions: Vec<String>,
}

impl SessionView {
    /// Whether a person would consider this session live.
    #[must_use]
    pub const fn is_live(&self) -> bool {
        matches!(
            self.standing,
            Standing::Launching | Standing::Running | Standing::Paused | Standing::Quarantined
        )
    }

    /// How long is left at `now`, never negative.
    ///
    /// A lease that ran out has no time left rather than minus four minutes, and a surface should
    /// not have to know that a countdown can pass through zero.
    #[must_use]
    pub fn remaining(&self, now: OffsetDateTime) -> Duration {
        (self.expires_at - now).max(Duration::ZERO)
    }

    /// How long this session ran, or has been running.
    #[must_use]
    pub fn uptime(&self, now: OffsetDateTime) -> Duration {
        (self.ended_at.unwrap_or(now) - self.started_at).max(Duration::ZERO)
    }
}

/// Action command requested on an active or live capsule session.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapsuleAction {
    /// Freeze the cgroup processes.
    Freeze,
    /// Thaw/resume frozen processes.
    Resume,
    /// Freeze cgroup and revoke model/network egress leases.
    Quarantine,
    /// Terminate and release capsule resources.
    Stop,
}

/// Fine-grained live telemetry snapshot from within an agent capsule's boundary.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapsuleTelemetryRecord {
    /// Target capsule UUID.
    pub capsule_id: Uuid,
    /// Current standing of the capsule.
    pub standing: Standing,
    /// Total process count currently active in cgroup.
    pub pids_count: u32,
    /// Memory used in megabytes.
    pub memory_used_mib: u64,
    /// Memory ceiling in megabytes.
    pub memory_max_mib: u64,
    /// CPU usage percentage in [0.0, 100.0].
    pub cpu_usage_pct: f32,
    /// Number of network egress requests mediated.
    pub egress_requests_count: u64,
    /// Number of denied network egress attempts.
    pub egress_denied_count: u64,
    /// Number of files modified in the workspace.
    pub files_modified_count: u64,
    /// Input prompt tokens consumed.
    pub tokens_in: u64,
    /// Output completion tokens generated.
    pub tokens_out: u64,
    /// Currently executing tool name or thought turn, if any.
    pub active_tool: Option<String>,
    /// Recent security and activity events.
    pub recent_activity: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_launch_request_carries_selection_but_no_authority() {
        let request = LaunchRequest {
            profile: "bounded".to_owned(),
            agent: "opencode".to_owned(),
            workspace: "/srv/project".to_owned(),
            model_class: Some("Strong".to_owned()),
            prompt: "Inspect the repository".to_owned(),
        };

        let value = serde_json::to_value(request).expect("serialize launch request");
        assert_eq!(value["profile"], "bounded");
        assert_eq!(value["modelClass"], "Strong");
        for authority in [
            "memoryMiB",
            "cpus",
            "tasksMax",
            "hosts",
            "spendLimit",
            "lifetimeSeconds",
        ] {
            assert!(
                value.get(authority).is_none(),
                "request carried {authority}"
            );
        }

        let attempted = serde_json::json!({
            "profile": "bounded",
            "agent": "opencode",
            "workspace": "/srv/project",
            "modelClass": "Strong",
            "prompt": "Inspect the repository",
            "memoryMiB": 65536
        });
        assert!(
            serde_json::from_value::<LaunchRequest>(attempted).is_err(),
            "authority-shaped input is refused rather than silently ignored"
        );
    }

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    fn view() -> SessionView {
        SessionView {
            capsule_id: Uuid::from_u128(0x0c01),
            agent: "opencode".to_owned(),
            profile: "sandboxed-autonomous".to_owned(),
            workspace: "/srv/project".to_owned(),
            standing: Standing::Running,
            ended_because: None,
            started_at: at(0),
            expires_at: at(4 * 60 * 60),
            ended_at: None,
            model_class: Some("Strong".to_owned()),
            spend: Some(SpendView::Capped {
                limit: 100,
                spent: Some(42),
            }),
            spend_observed_at: Some(at(120)),
            memory_mib: 4096,
            cpus: 2,
            tasks_max: 512,
            hosts: vec!["github.com".to_owned()],
            units: vec!["cybou-capsule-x.service".to_owned()],
            task: None,
        }
    }

    #[test]
    fn a_clock_is_the_readers_arithmetic() {
        // A duration is stale the moment it is serialised. Given both ends a card keeps its own
        // clock honest without the owner resending anything every second.
        let view = view();
        assert_eq!(view.uptime(at(600)), Duration::seconds(600));
        assert_eq!(
            view.remaining(at(600)),
            Duration::seconds(4 * 60 * 60 - 600)
        );
        assert!(view.is_live());
    }

    #[test]
    fn a_countdown_stops_at_nothing_left_rather_than_going_negative() {
        assert_eq!(view().remaining(at(5 * 60 * 60)), Duration::ZERO);
    }

    #[test]
    fn a_finished_session_counts_what_it_ran_for() {
        let mut ended = view();
        ended.standing = Standing::Ended;
        ended.ended_at = Some(at(300));
        assert_eq!(ended.uptime(at(9000)), Duration::seconds(300));
        assert!(!ended.is_live());
    }

    #[test]
    fn one_definition_travels_between_the_owner_and_whatever_draws_it() {
        // The reason this type is here rather than beside the owner. Two definitions agree on the
        // day they are written and a surface that assembled these facts itself would be a second
        // answer to what is running.
        let encoded = serde_json::to_string(&view()).expect("encodes");
        let decoded: SessionView = serde_json::from_str(&encoded).expect("decodes");
        assert_eq!(decoded, view());
    }

    #[test]
    fn a_reporter_with_no_ledger_says_unknown_rather_than_nought() {
        let unknown = SpendView::Capped {
            limit: 100,
            spent: None,
        };
        let encoded = serde_json::to_string(&unknown).expect("encodes");
        assert!(encoded.contains("null"), "{encoded}");
        assert_ne!(
            unknown,
            SpendView::Capped {
                limit: 100,
                spent: Some(0)
            },
            "nobody looked and nothing was spent are different facts"
        );
    }
}
