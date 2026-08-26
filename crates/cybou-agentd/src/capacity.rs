// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What this whole host will hand out, as against what one capsule may have.
//!
//! A profile bounds one session and bounds it well. Nothing bounded the sum, so four honest
//! four-gigabyte grants fitted on an eight-gigabyte host and every one of them was within policy.
//! Each session was correct and the host was oversubscribed, which is the shape of failure that
//! cannot be found by looking at any single grant.
//!
//! ## Reserved is a decision; used is an observation
//!
//! ```text
//! what the capsules are using now   telemetry, and it moves
//! what has been promised to them    admission, and it is a number somebody chose
//! ```
//!
//! Admission is done against the second. Deciding by the first would mean a host that admits a
//! session because the last one happens to be idle — and then cannot keep either promise when both
//! start working. A reservation is what was *granted*, and it is owed for as long as the lease lives
//! whether or not anybody is using it yet.
//!
//! The consequence worth stating: this will refuse launches on a host that looks half empty. That is
//! the point. The alternative is a ceiling that holds only while nothing is happening.
//!
//! ## Checking and taking must be one step
//!
//! Two callers that each ask *is there room* and then each take it will both be told yes. So there is
//! no public "is there room": [`crate::registry::SessionRegistry::admit`] decides and inserts under
//! one lock, and this module only supplies the arithmetic it uses.

use serde::{Deserialize, Serialize};

use cybou_capsule::{CapsuleGrant, SpendPolicy};

/// What an operator will let this host promise in total.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostCapacity {
    /// The most sessions that may be live at once.
    ///
    /// Its own limit rather than something implied by memory. Sessions cost more than their
    /// ceilings — units, sockets, brokers, gateways — and a host can be made unusable by many small
    /// capsules well before any of them reaches a byte of what it was promised.
    pub max_sessions: u32,
    /// The most memory that may be promised across every live session, in mebibytes.
    pub memory_mib: u32,
    /// The most CPU that may be promised across every live session.
    pub cpus: u32,
    /// The most processes that may be promised across every live session.
    pub tasks_max: u32,
    /// The most money that may be promised across every live session.
    ///
    /// A grant that may spend nothing reserves nothing here, which is the whole reason a zero-cost
    /// selection is a selection rather than an empty budget.
    pub spend_units: u64,
}

impl HostCapacity {
    /// A host nobody has bounded.
    ///
    /// What an absent configuration means, said out loud rather than defaulted into. It is the
    /// behaviour of every version before this module existed, and naming it is what lets a surface
    /// that must not run unbounded refuse to start without one.
    #[must_use]
    pub const fn unbounded() -> Self {
        Self {
            max_sessions: u32::MAX,
            memory_mib: u32::MAX,
            cpus: u32::MAX,
            tasks_max: u32::MAX,
            spend_units: u64::MAX,
        }
    }

    /// Whether this is a real bound or the absence of one.
    #[must_use]
    pub fn is_bounded(&self) -> bool {
        *self != Self::unbounded()
    }

    /// Read what an operator wrote.
    ///
    /// # Errors
    ///
    /// Returns [`NotAdmitted::UnreadableCapacity`] when the bytes are not a capacity this build
    /// understands. Refused whole rather than partly read: a file half of which parsed would bound
    /// the host by numbers nobody wrote.
    pub fn read(bytes: &[u8]) -> Result<Self, NotAdmitted> {
        serde_json::from_slice(bytes).map_err(|_| NotAdmitted::UnreadableCapacity)
    }
}

/// What has already been promised.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Reserved {
    /// How many sessions hold a promise.
    pub sessions: u32,
    /// Memory promised, in mebibytes.
    pub memory_mib: u64,
    /// CPU promised.
    pub cpus: u64,
    /// Processes promised.
    pub tasks_max: u64,
    /// Money promised.
    pub spend_units: u64,
}

impl Reserved {
    /// What one grant reserves.
    ///
    /// Widened to `u64` on the way in. Four grants of three billion mebibytes is not a number a host
    /// can honour, but it is also not a number that may quietly wrap into a small one and be
    /// admitted.
    #[must_use]
    pub fn of(grant: &CapsuleGrant) -> Self {
        Self {
            sessions: 1,
            memory_mib: u64::from(grant.budget.memory_mib),
            cpus: u64::from(grant.budget.cpus),
            tasks_max: u64::from(grant.budget.tasks_max),
            spend_units: grant.model.as_ref().map_or(0, |model| match model.spend {
                SpendPolicy::Capped(limit) => limit,
                // Nothing may be spent, so nothing is held against the host's envelope. A zero-cost
                // selection that reserved money would make free models the scarcest thing on offer.
                SpendPolicy::ZeroCostOnly => 0,
            }),
        }
    }

    /// Everything these grants reserve together.
    #[must_use]
    pub fn across<'a>(grants: impl IntoIterator<Item = &'a CapsuleGrant>) -> Self {
        grants
            .into_iter()
            .map(Self::of)
            .fold(Self::default(), Self::and)
    }

    /// This and one more, saturating rather than wrapping.
    #[must_use]
    pub const fn and(self, other: Self) -> Self {
        Self {
            sessions: self.sessions.saturating_add(other.sessions),
            memory_mib: self.memory_mib.saturating_add(other.memory_mib),
            cpus: self.cpus.saturating_add(other.cpus),
            tasks_max: self.tasks_max.saturating_add(other.tasks_max),
            spend_units: self.spend_units.saturating_add(other.spend_units),
        }
    }
}

/// Why a host will not promise this as well as everything it has already promised.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotAdmitted {
    /// The configured capacity could not be read.
    UnreadableCapacity,
    /// This host will hold no more sessions at once.
    Sessions {
        /// How many are already live.
        held: u32,
        /// How many it will hold.
        limit: u32,
    },
    /// Memory would be promised twice.
    Memory {
        /// What the host would then owe, in mebibytes.
        wanted: u64,
        /// What it will owe.
        limit: u32,
    },
    /// CPU would be promised twice.
    Cpus {
        /// What the host would then owe.
        wanted: u64,
        /// What it will owe.
        limit: u32,
    },
    /// Processes would be promised twice.
    Tasks {
        /// What the host would then owe.
        wanted: u64,
        /// What it will owe.
        limit: u32,
    },
    /// The host's spending envelope would be exceeded.
    Spend {
        /// What would then be promised.
        wanted: u64,
        /// What may be.
        limit: u64,
    },
}

impl core::fmt::Display for NotAdmitted {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnreadableCapacity => {
                formatter.write_str("this host's capacity could not be read")
            }
            Self::Sessions { held, limit } => write!(
                formatter,
                "this host holds {held} of {limit} session(s) already"
            ),
            Self::Memory { wanted, limit } => write!(
                formatter,
                "{wanted} MiB would be promised and this host promises at most {limit} MiB"
            ),
            Self::Cpus { wanted, limit } => write!(
                formatter,
                "{wanted} CPUs would be promised and this host promises at most {limit}"
            ),
            Self::Tasks { wanted, limit } => write!(
                formatter,
                "{wanted} processes would be promised and this host promises at most {limit}"
            ),
            Self::Spend { wanted, limit } => write!(
                formatter,
                "{wanted} unit(s) would be promised and this host promises at most {limit}"
            ),
        }
    }
}

impl core::error::Error for NotAdmitted {}

/// Whether one more grant fits beside everything already promised.
///
/// Reported by which limit was reached rather than as a single refusal, because *this host is full*
/// and *this host will not promise that much memory* send a person to different places.
///
/// # Errors
///
/// Returns the [`NotAdmitted`] naming the first limit the total would cross.
pub fn admits(
    capacity: HostCapacity,
    already: Reserved,
    grant: &CapsuleGrant,
) -> Result<(), NotAdmitted> {
    let total = already.and(Reserved::of(grant));

    if total.sessions > capacity.max_sessions {
        return Err(NotAdmitted::Sessions {
            held: already.sessions,
            limit: capacity.max_sessions,
        });
    }
    if total.memory_mib > u64::from(capacity.memory_mib) {
        return Err(NotAdmitted::Memory {
            wanted: total.memory_mib,
            limit: capacity.memory_mib,
        });
    }
    if total.cpus > u64::from(capacity.cpus) {
        return Err(NotAdmitted::Cpus {
            wanted: total.cpus,
            limit: capacity.cpus,
        });
    }
    if total.tasks_max > u64::from(capacity.tasks_max) {
        return Err(NotAdmitted::Tasks {
            wanted: total.tasks_max,
            limit: capacity.tasks_max,
        });
    }
    if total.spend_units > capacity.spend_units {
        return Err(NotAdmitted::Spend {
            wanted: total.spend_units,
            limit: capacity.spend_units,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use cybou_capsule::{
        CapabilityProfile, LeaseRequest, ModelGrant, NetworkGrant, ResourceBudget, Workspace,
        issue_lease,
    };
    use time::{Duration, OffsetDateTime};
    use uuid::Uuid;

    use super::*;

    fn at() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000).expect("a fixed instant")
    }

    fn grant(memory_mib: u32, spend: Option<SpendPolicy>) -> CapsuleGrant {
        let mut profile = CapabilityProfile::bounded(
            "sandboxed-autonomous",
            ResourceBudget {
                memory_mib,
                cpus: 2,
                tasks_max: 512,
                lifetime: Duration::hours(4),
            },
        )
        .expect("a valid profile");
        profile.network = NetworkGrant::default();
        profile.model = spend.map(|spend| ModelGrant {
            class: "Strong".to_owned(),
            spend,
        });
        profile.may_execute = true;
        issue_lease(
            LeaseRequest {
                selected_profile: profile,
                capsule_id: Uuid::new_v4(),
                agent: "opencode".to_owned(),
                workspace: Workspace::at("/srv/project"),
            },
            at(),
        )
        .expect("a lease is issued")
        .grant()
        .clone()
    }

    fn host() -> HostCapacity {
        HostCapacity {
            max_sessions: 2,
            memory_mib: 8192,
            cpus: 4,
            tasks_max: 2048,
            spend_units: 200,
        }
    }

    #[test]
    fn four_honest_grants_do_not_fit_on_a_host_that_can_hold_two() {
        // The failure no single grant could show. Each of these is within its profile and the host
        // is oversubscribed by the third.
        let capacity = HostCapacity {
            memory_mib: 8192,
            max_sessions: 8,
            ..host()
        };
        let held = grant(4096, None);
        let already = Reserved::across([&held, &held]);

        assert_eq!(
            admits(capacity, already, &grant(4096, None)),
            Err(NotAdmitted::Memory {
                wanted: 12288,
                limit: 8192
            })
        );
    }

    #[test]
    fn one_more_fits_until_it_does_not() {
        let capacity = host();
        assert!(admits(capacity, Reserved::default(), &grant(4096, None)).is_ok());

        let one = Reserved::of(&grant(4096, None));
        assert!(admits(capacity, one, &grant(4096, None)).is_ok());

        let two = one.and(Reserved::of(&grant(4096, None)));
        assert_eq!(
            admits(capacity, two, &grant(1, None)),
            Err(NotAdmitted::Sessions { held: 2, limit: 2 })
        );
    }

    #[test]
    fn a_session_count_is_its_own_limit_and_not_implied_by_memory() {
        // Sessions cost more than their ceilings: units, sockets, brokers, gateways. A host can be
        // made unusable by many small capsules long before any reaches a byte of what it was
        // promised.
        let capacity = HostCapacity {
            max_sessions: 1,
            ..host()
        };
        let tiny = grant(1, None);

        assert_eq!(
            admits(capacity, Reserved::of(&tiny), &tiny),
            Err(NotAdmitted::Sessions { held: 1, limit: 1 })
        );
    }

    #[test]
    fn admission_is_against_what_was_promised_and_not_what_is_being_used() {
        // Deciding by current usage would admit a session because the last one happens to be idle,
        // and then break both promises when they both start working. Nothing in this module can see
        // usage at all, which is the point: it could not be tempted.
        let capacity = host();
        let idle_but_promised = Reserved::of(&grant(8192, None));

        assert_eq!(
            admits(capacity, idle_but_promised, &grant(1, None)),
            Err(NotAdmitted::Memory {
                wanted: 8193,
                limit: 8192
            })
        );
    }

    #[test]
    fn a_zero_cost_session_reserves_no_money() {
        // It may spend nothing, so it holds nothing against the envelope. Reserving for it would
        // make free models the scarcest thing on offer.
        let free = Reserved::of(&grant(512, Some(SpendPolicy::ZeroCostOnly)));
        assert_eq!(free.spend_units, 0);

        let capped = Reserved::of(&grant(512, Some(SpendPolicy::Capped(100))));
        assert_eq!(capped.spend_units, 100);
    }

    #[test]
    fn the_spending_envelope_is_across_sessions_and_not_within_one() {
        let capacity = host();
        let expensive = grant(512, Some(SpendPolicy::Capped(150)));

        assert!(admits(capacity, Reserved::default(), &expensive).is_ok());
        assert_eq!(
            admits(capacity, Reserved::of(&expensive), &expensive),
            Err(NotAdmitted::Spend {
                wanted: 300,
                limit: 200
            })
        );
    }

    #[test]
    fn a_total_that_would_wrap_is_refused_rather_than_admitted() {
        // Four grants of three billion mebibytes is not a number a host can honour, and it is also
        // not one that may quietly become small and be let in.
        let capacity = host();
        let enormous = Reserved {
            sessions: 1,
            memory_mib: u64::MAX - 1,
            cpus: 0,
            tasks_max: 0,
            spend_units: 0,
        };

        assert!(matches!(
            admits(capacity, enormous, &grant(4096, None)),
            Err(NotAdmitted::Memory { .. })
        ));
    }

    #[test]
    fn an_unbounded_host_is_the_absence_of_a_bound_and_says_so() {
        // What every version before this module did, named rather than defaulted into, so a surface
        // that must not run unbounded can refuse to start without a real one.
        let capacity = HostCapacity::unbounded();
        assert!(!capacity.is_bounded());
        assert!(host().is_bounded());
        assert!(admits(capacity, Reserved::default(), &grant(4096, None)).is_ok());
    }

    #[test]
    fn a_capacity_survives_the_file_it_is_written_in() {
        let written = serde_json::to_vec(&host()).expect("encodes");
        assert_eq!(HostCapacity::read(&written).expect("decodes"), host());
        assert_eq!(
            HostCapacity::read(b"{\"maxSessions\": 1}"),
            Err(NotAdmitted::UnreadableCapacity)
        );
    }

    #[test]
    fn which_limit_was_reached_is_named_rather_than_summarised() {
        // "This host is full" and "this host will not promise that much memory" send a person to
        // different places.
        let capacity = host();
        let wide = grant(512, None);

        let by_cpu = admits(
            HostCapacity {
                cpus: 1,
                ..capacity
            },
            Reserved::default(),
            &wide,
        );
        assert!(matches!(by_cpu, Err(NotAdmitted::Cpus { .. })));

        let by_tasks = admits(
            HostCapacity {
                tasks_max: 8,
                ..capacity
            },
            Reserved::default(),
            &wide,
        );
        assert!(matches!(by_tasks, Err(NotAdmitted::Tasks { .. })));
    }
}
