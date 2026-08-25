// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The agent session owner.
//!
//! Today it can work out and show what one selection implies, and nothing more: `plan` mints the one
//! lease and prints every file, unit and teardown step that launch would produce, without touching a
//! filesystem or a service manager. Carrying the plan out is the next step and is deliberately not
//! half-present here — a coordinator that starts some of a session and cannot end it is worse than
//! one that has not started yet.
//!
//! It is already useful as it stands: the derivation is the part that was missing. Every runtime
//! piece of a session now comes from one object, so `plan` is also the answer to *what exactly did
//! this person approve*, printable before anything runs.

use std::path::PathBuf;

use cybou_agentd::{Ceilings, Launch, plan};
use cybou_capsule::{
    CapabilityProfile, LeaseRequest, ModelGrant, NetworkGrant, ResourceBudget, Workspace,
    issue_lease,
};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

fn usage() -> &'static str {
    "usage: cybou-agentd plan --profile ID --agent NAME --workspace PATH \\\n\
     \x20   --memory-mib N --cpus N --tasks-max N --lifetime-seconds N \\\n\
     \x20   --token-limit N --max-output-tokens N --sensitivity N \\\n\
     \x20   [--model CLASS --spend-limit N] [--host HOST]… [--may-execute] \\\n\
     \x20   [--capsule-id UUID] [--task-id UUID]"
}

/// One launch selection, exactly as a person made it.
#[derive(Default)]
struct Selection {
    profile: Option<String>,
    agent: Option<String>,
    workspace: Option<PathBuf>,
    memory_mib: Option<u32>,
    cpus: Option<u32>,
    tasks_max: Option<u32>,
    lifetime_seconds: Option<i64>,
    token_limit: Option<u64>,
    max_output_tokens: Option<u32>,
    sensitivity: Option<u8>,
    model: Option<String>,
    spend_limit: Option<u64>,
    hosts: Vec<String>,
    may_execute: bool,
    capsule_id: Option<Uuid>,
    task_id: Option<Uuid>,
}

fn main() -> Result<(), String> {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().as_deref() {
        Some("plan") => show_plan(&parse(arguments.collect())?),
        _ => Err(usage().to_owned()),
    }
}

fn parse(arguments: Vec<String>) -> Result<Selection, String> {
    let mut selection = Selection::default();
    let mut arguments = arguments.into_iter();
    while let Some(flag) = arguments.next() {
        let mut value = || {
            arguments
                .next()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match flag.as_str() {
            "--may-execute" => selection.may_execute = true,
            "--profile" => selection.profile = Some(value()?),
            "--agent" => selection.agent = Some(value()?),
            "--workspace" => selection.workspace = Some(PathBuf::from(value()?)),
            "--model" => selection.model = Some(value()?),
            "--host" => selection.hosts.push(value()?),
            "--memory-mib" => selection.memory_mib = Some(number(&flag, &value()?)?),
            "--cpus" => selection.cpus = Some(number(&flag, &value()?)?),
            "--tasks-max" => selection.tasks_max = Some(number(&flag, &value()?)?),
            "--lifetime-seconds" => selection.lifetime_seconds = Some(number(&flag, &value()?)?),
            "--token-limit" => selection.token_limit = Some(number(&flag, &value()?)?),
            "--max-output-tokens" => selection.max_output_tokens = Some(number(&flag, &value()?)?),
            "--sensitivity" => selection.sensitivity = Some(number(&flag, &value()?)?),
            "--spend-limit" => selection.spend_limit = Some(number(&flag, &value()?)?),
            "--capsule-id" => selection.capsule_id = Some(uuid(&flag, &value()?)?),
            "--task-id" => selection.task_id = Some(uuid(&flag, &value()?)?),
            other => return Err(format!("unknown argument {other}\n{}", usage())),
        }
    }
    Ok(selection)
}

fn number<T: std::str::FromStr>(flag: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("{flag} is not a number in range"))
}

fn uuid(flag: &str, value: &str) -> Result<Uuid, String> {
    Uuid::parse_str(value).map_err(|_| format!("{flag} is not a UUID"))
}

fn required<T>(value: Option<T>, flag: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("{flag} is required\n{}", usage()))
}

fn show_plan(selection: &Selection) -> Result<(), String> {
    let now = OffsetDateTime::now_utc();

    // The one mint, called once. Every name printed below is derived from what it returns.
    let mut profile = CapabilityProfile::bounded(
        required(selection.profile.clone(), "--profile")?,
        ResourceBudget {
            memory_mib: required(selection.memory_mib, "--memory-mib")?,
            cpus: required(selection.cpus, "--cpus")?,
            tasks_max: required(selection.tasks_max, "--tasks-max")?,
            lifetime: Duration::seconds(required(
                selection.lifetime_seconds,
                "--lifetime-seconds",
            )?),
        },
    )
    .map_err(|error| error.to_string())?;
    profile.network = NetworkGrant {
        hosts: selection.hosts.clone(),
    };
    // A class with no ceiling beside it is half a selection: it would leave the spending bound to be
    // invented by whichever component needed one first, which is the shape this crate exists to end.
    profile.model = match &selection.model {
        Some(class) => Some(ModelGrant {
            class: class.clone(),
            spend_limit: required(selection.spend_limit, "--spend-limit")?,
        }),
        None => None,
    };
    profile.may_execute = selection.may_execute;

    let lease = issue_lease(
        LeaseRequest {
            selected_profile: profile,
            capsule_id: selection.capsule_id.unwrap_or_else(Uuid::new_v4),
            agent: required(selection.agent.clone(), "--agent")?,
            workspace: Workspace::at(required(selection.workspace.clone(), "--workspace")?),
        },
        now,
    )
    .map_err(|error| error.to_string())?;

    let launch = Launch {
        lease,
        task_id: selection.task_id.unwrap_or_else(Uuid::new_v4),
        ceilings: Ceilings {
            token_limit: required(selection.token_limit, "--token-limit")?,
            max_output_tokens: required(selection.max_output_tokens, "--max-output-tokens")?,
            sensitivity: required(selection.sensitivity, "--sensitivity")?,
        },
    };
    let plan = plan::plan(&launch, now).map_err(|error| error.to_string())?;

    println!("session {}", plan.instance);
    println!("profile {}", launch.lease.profile_id().as_str());
    println!("agent {}", launch.lease.grant().agent);
    println!(
        "workspace {}",
        launch.lease.grant().workspace.root.display()
    );
    println!("expires {}", plan.expires_at);
    println!("lease-file {}", plan.lease_file.display());
    println!("launch-file {}", plan.launch_file.display());
    println!("gateway-unit {}", plan.gateway_unit);
    println!("capsule-unit {}", plan.capsule_unit);
    println!("model-socket {}", plan.model_socket.display());
    println!("model-token {}", plan.model_token.display());
    for line in plan.launch_environment.lines() {
        println!("launch-env {line}");
    }
    for step in plan.teardown() {
        match step {
            cybou_agentd::TeardownStep::StopCapsule(unit) => {
                println!("teardown stop-capsule {unit}");
            }
            cybou_agentd::TeardownStep::StopGateway(unit) => {
                println!("teardown stop-gateway {unit}");
            }
            cybou_agentd::TeardownStep::Remove(path) => {
                println!("teardown remove {}", path.display());
            }
        }
    }
    Ok(())
}
