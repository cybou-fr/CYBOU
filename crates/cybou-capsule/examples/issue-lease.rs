// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Mint one standing lease from one explicit launch selection and write it where a runtime can read
//! it.
//!
//! Exists because the lease has to *travel*. The gateway, the capsule backend and the session owner
//! are separate processes by design, and until now each one that needed a lease built its own from
//! whatever values reached it. Two such reconstructions can both be valid and still describe
//! different permissions, and nothing downstream can say which one a person actually approved.
//!
//! So there is one mint — [`issue_lease`] — and this is how its output leaves the process that
//! called it: CBOR, the encoding the lease already round-trips through in its own tests.
//!
//! This is a gate and bring-up tool, not the launch surface. The launch surface is the session owner
//! that holds Launch, Stop and expiry; when it exists it calls the same mint and writes the same
//! bytes.

use std::path::PathBuf;

use cybou_capsule::grant::{ModelGrant, NetworkGrant, ResourceBudget, SpendPolicy, Workspace};
use cybou_capsule::profile::{CapabilityProfile, LeaseRequest, issue_lease};

fn required(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|_| format!("{name} must name part of the launch selection"))
}

fn number(name: &str) -> Result<u32, String> {
    required(name)?
        .parse()
        .map_err(|_| format!("{name} is not an unsigned integer"))
}

fn spend_policy(value: &str) -> Result<SpendPolicy, String> {
    if value == "zero-cost" {
        return Ok(SpendPolicy::ZeroCostOnly);
    }
    value
        .parse()
        .map(SpendPolicy::Capped)
        .map_err(|_| "CYBOU_MODEL_SPEND_LIMIT is an integer or the word zero-cost".to_owned())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args()
        .nth(1)
        .ok_or("usage: issue-lease <output-path>")?;

    let hosts: Vec<String> = std::env::var("CYBOU_EGRESS_HOSTS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|host| !host.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    let mut profile = CapabilityProfile::bounded(
        required("CYBOU_PROFILE_ID")?,
        ResourceBudget {
            memory_mib: number("CYBOU_CAPSULE_MEMORY_MIB")?,
            cpus: number("CYBOU_CAPSULE_CPUS")?,
            tasks_max: number("CYBOU_CAPSULE_TASKS_MAX")?,
            lifetime: time::Duration::seconds(i64::from(number("CYBOU_AGENT_LEASE_SECONDS")?)),
        },
    )?;
    profile.network = NetworkGrant { hosts };
    // A model class with no policy beside it is half a selection. Both or neither, so a launch
    // cannot grant a class and leave the spending bound to be invented further down.
    //
    // `zero-cost` is a word rather than the number nought, because "spend up to nothing" and "spend
    // nothing, on something that costs nothing" are different selections that an integer cannot
    // tell apart.
    profile.model = match std::env::var("CYBOU_MODEL_CLASS") {
        Ok(class) => Some(ModelGrant {
            class,
            spend: spend_policy(&required("CYBOU_MODEL_SPEND_LIMIT")?)?,
        }),
        Err(_) => None,
    };
    profile.may_execute = required("CYBOU_CAPSULE_MAY_EXECUTE")? == "yes";

    let lease = issue_lease(
        LeaseRequest {
            selected_profile: profile,
            capsule_id: required("CYBOU_CAPSULE_ID")?.parse()?,
            agent: required("CYBOU_AGENT")?,
            workspace: Workspace::at(PathBuf::from(required("CYBOU_AGENT_WORKSPACE")?)),
        },
        time::OffsetDateTime::now_utc(),
    )?;

    let mut encoded = Vec::new();
    ciborium::into_writer(&lease, &mut encoded)?;
    std::fs::write(&out, &encoded)?;
    Ok(())
}
