// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Reading a session's own launch files back off the host.
//!
//! The inverse of what a launch wrote, and deliberately written as such: [`read_launch`] parses
//! exactly the file [`crate::plan::SessionPlan::launch_environment`] produces, and a round-trip test
//! holds the two together. A parser written to accept "roughly that shape" would drift from the
//! writer the first time a field moved, and the failure would look like a session that stopped
//! existing rather than like a mismatch.
//!
//! ## Why the launch file is enough, together with the lease
//!
//! Between them they carry the whole of a session: the lease is the authority a person approved, and
//! the launch file is the task and the token ceilings, which are not authority. Nothing else was
//! ever needed — every path, unit and command is derived from those two through the same `plan()`
//! the launch used.
//!
//! ## What this module refuses to infer
//!
//! Whether the session is still running. That is asked of the service manager, because a file says
//! what a launch intended and only a live unit says what is true now. A recovery that read a lease
//! file and concluded *therefore an agent is working* would report every session that ever ran.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use cybou_capsule::Lease;
use cybou_protocol::model::ModelUsageSnapshot;

use crate::plan::Ceilings;
use crate::registry::Found;

/// Where a session's launch decision is written. Matches `crate::plan`.
const LEASE_ROOT: &str = "/run/cybou-agent-leases";

/// Why one session on the host could not be read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CannotRead {
    /// The launch file is missing a value this build needs.
    Missing(&'static str),
    /// A value is present but is not what its name says it is.
    Malformed(&'static str),
    /// The lease bytes are not a lease.
    UnreadableLease,
    /// The published ledger is not a snapshot.
    UnreadableUsage,
}

impl core::fmt::Display for CannotRead {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Missing(name) => write!(formatter, "the launch file does not define {name}"),
            Self::Malformed(name) => write!(formatter, "{name} is not the kind of value it names"),
            Self::UnreadableLease => formatter.write_str("the lease file is not a lease"),
            Self::UnreadableUsage => {
                formatter.write_str("the published ledger is not a usage snapshot")
            }
        }
    }
}

impl core::error::Error for CannotRead {}

/// Parse the launch file a session wrote for its gateway.
///
/// Returns the task the model bearer was for and the ceilings that bound it — and nothing that is
/// authority, because a launch file carries none. Anything it did carry that the lease also says
/// would be a second answer to the same question, which is the defect this whole split exists to
/// close, so an unexpected name is passed over rather than believed.
///
/// # Errors
///
/// Returns [`CannotRead`] when a value this build needs is absent or is not what its name says.
pub fn read_launch(contents: &str) -> Result<(Uuid, Ceilings), CannotRead> {
    let values: BTreeMap<&str, &str> = contents
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(name, value)| (name.trim(), value.trim()))
        .collect();

    let get = |name: &'static str| values.get(name).copied().ok_or(CannotRead::Missing(name));
    let number = |name: &'static str| -> Result<u64, CannotRead> {
        get(name)?.parse().map_err(|_| CannotRead::Malformed(name))
    };

    let task_id = Uuid::parse_str(get("CYBOU_AGENT_TASK_ID")?)
        .map_err(|_| CannotRead::Malformed("CYBOU_AGENT_TASK_ID"))?;
    Ok((
        task_id,
        Ceilings {
            token_limit: number("CYBOU_MODEL_TOKEN_LIMIT")?,
            max_output_tokens: u32::try_from(number("CYBOU_MODEL_MAX_OUTPUT_TOKENS")?)
                .map_err(|_| CannotRead::Malformed("CYBOU_MODEL_MAX_OUTPUT_TOKENS"))?,
            sensitivity: u8::try_from(number("CYBOU_MODEL_SENSITIVITY")?)
                .map_err(|_| CannotRead::Malformed("CYBOU_MODEL_SENSITIVITY"))?,
        },
    ))
}

/// Decode a lease exactly as the gateway does.
///
/// # Errors
///
/// Returns [`CannotRead::UnreadableLease`] when the bytes are not a lease this build understands.
pub fn read_lease(bytes: &[u8]) -> Result<Lease, CannotRead> {
    ciborium::from_reader(bytes).map_err(|_| CannotRead::UnreadableLease)
}

/// Read the ledger a session's gateway published.
///
/// The one place a spend figure may come from. An owner reading its own copy of the lease would
/// report nought forever: it holds the grant a person approved, and the gateway holds the ledger.
///
/// # Errors
///
/// Returns [`CannotRead::UnreadableUsage`] when the bytes are not a snapshot this build understands.
/// A snapshot that cannot be read is reported rather than treated as nought, because *nobody looked*
/// and *nothing was spent* are different facts and only one of them is ever safe to show.
pub fn read_usage(bytes: &[u8]) -> Result<ModelUsageSnapshot, CannotRead> {
    serde_json::from_slice(bytes).map_err(|_| CannotRead::UnreadableUsage)
}

/// The pair of files one session wrote, by capsule identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaunchFiles {
    /// The session this pair belongs to.
    pub capsule_id: Uuid,
    /// The authoritative lease.
    pub lease: PathBuf,
    /// The task and the token ceilings.
    pub launch: PathBuf,
}

/// Where this build expects a session's files to be.
#[must_use]
pub fn files_for(capsule_id: Uuid) -> LaunchFiles {
    let root = Path::new(LEASE_ROOT);
    LaunchFiles {
        capsule_id,
        lease: root.join(format!("{capsule_id}.lease")),
        launch: root.join(format!("{capsule_id}.env")),
    }
}

/// Every session that left files behind, by the identity in the filename.
///
/// A `.lease` with no `.env` beside it is not returned. Half a launch is not a session to recover:
/// its ceilings were never written, so nothing could re-derive what it was, and treating the pair as
/// optional would mean inventing bounds for a bearer somebody approved with different ones.
///
/// # Errors
///
/// Returns the underlying failure when the directory itself cannot be read. A directory that is
/// absent is not an error — it is a host with no sessions.
pub fn sessions_on(root: &Path) -> Result<Vec<LaunchFiles>, std::io::Error> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };

    let mut out = Vec::new();
    for entry in entries {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(identity) = name.strip_suffix(".lease") else {
            continue;
        };
        let Ok(capsule_id) = Uuid::parse_str(identity) else {
            // A filename that is not a capsule identity was not written by a launch. Reading it
            // would be this module accepting a session named by whoever could create a file.
            continue;
        };
        let launch = root.join(format!("{capsule_id}.env"));
        if launch.is_file() {
            out.push(LaunchFiles {
                capsule_id,
                lease: path,
                launch,
            });
        }
    }
    out.sort_by_key(|files| files.capsule_id);
    Ok(out)
}

/// Read one session's pair of files into something [`crate::registry::recover`] can use.
///
/// `capsule_active` is supplied rather than looked up, so the judgement that a unit is running stays
/// with whatever can actually ask a service manager, and this stays testable without one.
///
/// # Errors
///
/// Returns [`CannotRead`] when either file cannot be read back.
pub fn read_session(
    lease_bytes: &[u8],
    launch: &str,
    capsule_active: bool,
) -> Result<Found, CannotRead> {
    let lease = read_lease(lease_bytes)?;
    let (task_id, ceilings) = read_launch(launch)?;
    Ok(Found {
        lease,
        task_id,
        ceilings,
        capsule_active,
    })
}

#[cfg(test)]
mod tests {
    use cybou_capsule::{
        CapabilityProfile, LeaseRequest, ModelGrant, NetworkGrant, ResourceBudget, SpendPolicy,
        Workspace, issue_lease,
    };
    use time::{Duration, OffsetDateTime};

    use super::*;
    use crate::plan::{Launch, plan};

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    const CAPSULE: Uuid = Uuid::from_u128(0xe001);
    const TASK: Uuid = Uuid::from_u128(0xe002);

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
        profile.network = NetworkGrant::to(&["github.com"]);
        profile.model = Some(ModelGrant {
            class: "Strong".to_owned(),
            spend: SpendPolicy::Capped(100),
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

    fn ceilings() -> Ceilings {
        Ceilings {
            token_limit: 200_000,
            max_output_tokens: 4096,
            sensitivity: 1,
        }
    }

    fn written() -> String {
        plan(
            &Launch {
                lease: lease(),
                task_id: TASK,
                ceilings: ceilings(),
            },
            at(1),
        )
        .expect("a plan")
        .launch_environment
    }

    #[test]
    fn what_a_launch_wrote_is_what_a_recovery_reads() {
        // The guarantee that matters. A parser written to accept roughly the right shape would drift
        // from the writer the first time a field moved, and the failure would look like a session
        // that stopped existing rather than like a mismatch.
        let (task_id, read) = read_launch(&written()).expect("reads back");

        assert_eq!(task_id, TASK);
        assert_eq!(read, ceilings());
    }

    #[test]
    fn a_lease_survives_the_file_it_was_written_to() {
        let original = lease();
        let mut encoded = Vec::new();
        ciborium::into_writer(&original, &mut encoded).expect("encodes");

        assert_eq!(read_lease(&encoded).expect("decodes"), original);
        assert_eq!(read_lease(b"not a lease"), Err(CannotRead::UnreadableLease));
    }

    #[test]
    fn a_launch_file_missing_a_ceiling_is_named_rather_than_defaulted() {
        // Defaulting one would put a bound on a bearer that nobody approved, and would do it
        // silently, at exactly the moment a person is trying to find out what is running.
        let without = written()
            .lines()
            .filter(|line| !line.starts_with("CYBOU_MODEL_TOKEN_LIMIT"))
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(
            read_launch(&without),
            Err(CannotRead::Missing("CYBOU_MODEL_TOKEN_LIMIT"))
        );
    }

    #[test]
    fn a_value_that_is_not_what_it_says_is_refused() {
        let wrong = written().replace("CYBOU_MODEL_SENSITIVITY=1", "CYBOU_MODEL_SENSITIVITY=999");
        assert_eq!(
            read_launch(&wrong),
            Err(CannotRead::Malformed("CYBOU_MODEL_SENSITIVITY"))
        );

        let not_a_uuid = written().replace(&TASK.to_string(), "the-task");
        assert_eq!(
            read_launch(&not_a_uuid),
            Err(CannotRead::Malformed("CYBOU_AGENT_TASK_ID"))
        );
    }

    #[test]
    fn a_launch_file_that_carries_authority_is_not_believed() {
        // A name the lease already answers is a second answer to the same question. Reading one here
        // would reopen the defect the whole split closed: a file could then widen a grant.
        let smuggled = format!(
            "{}CYBOU_MODEL_SPEND_LIMIT=999999\nCYBOU_AGENT_LEASE_SECONDS=99999\n",
            written()
        );
        let (task_id, read) = read_launch(&smuggled).expect("reads back");

        assert_eq!(task_id, TASK);
        assert_eq!(read, ceilings(), "nothing smuggled in changed a bound");
    }

    #[test]
    fn a_published_ledger_reads_back_and_a_broken_one_is_not_read_as_nought() {
        // "Nobody looked" and "nothing was spent" are different facts, and only one of them is ever
        // safe to put in front of a person who set a spending bound.
        let snapshot = ModelUsageSnapshot {
            capsule_id: CAPSULE,
            spend_units: 42,
            tokens: 1234,
            completions: 3,
            observed_at: at(600),
        };
        let rendered = serde_json::to_vec(&snapshot).expect("encodes");

        assert_eq!(read_usage(&rendered).expect("decodes"), snapshot);
        assert_eq!(read_usage(b"{"), Err(CannotRead::UnreadableUsage));
    }

    #[test]
    fn the_two_files_of_one_session_are_named_from_its_identity() {
        let files = files_for(CAPSULE);
        assert_eq!(files.capsule_id, CAPSULE);
        assert!(files.lease.ends_with(format!("{CAPSULE}.lease")));
        assert!(files.launch.ends_with(format!("{CAPSULE}.env")));
    }

    #[test]
    fn a_host_with_no_launch_directory_has_no_sessions_rather_than_a_fault() {
        let missing = Path::new("/nonexistent/cybou-agent-leases");
        assert_eq!(sessions_on(missing).expect("not a fault"), Vec::new());
    }

    #[test]
    fn only_a_complete_pair_is_a_session() {
        // Half a launch cannot be re-derived: its ceilings were never written. Treating the pair as
        // optional would mean inventing bounds for a bearer somebody approved with different ones.
        let root = std::env::temp_dir().join(format!("cybou-discovery-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("a directory");

        let paired = Uuid::from_u128(0xe010);
        std::fs::write(root.join(format!("{paired}.lease")), b"x").expect("write");
        std::fs::write(root.join(format!("{paired}.env")), b"x").expect("write");

        let lonely = Uuid::from_u128(0xe011);
        std::fs::write(root.join(format!("{lonely}.lease")), b"x").expect("write");

        // Not a capsule identity, so not written by a launch.
        std::fs::write(root.join("notes.lease"), b"x").expect("write");
        std::fs::write(root.join("notes.env"), b"x").expect("write");

        let found = sessions_on(&root).expect("reads");
        std::fs::remove_dir_all(&root).ok();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].capsule_id, paired);
    }
}
