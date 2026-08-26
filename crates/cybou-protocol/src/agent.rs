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
}

impl SessionView {
    /// Whether a person would consider this session live.
    #[must_use]
    pub const fn is_live(&self) -> bool {
        matches!(self.standing, Standing::Launching | Standing::Running)
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

#[cfg(test)]
mod tests {
    use super::*;

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
