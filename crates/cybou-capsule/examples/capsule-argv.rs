// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Print the command a capsule would be built with, one argument per line.
//!
//! Exists so the escape gate tests what this crate actually produces rather than a command somebody
//! wrote out by hand in a shell script. A gate against a hand-written command tests the shell
//! script, and passes forever after the code stops agreeing with it.
//!
//! One argument per line because a capsule's arguments contain paths, and a path may contain a
//! space. Anything that joined them would be a quoting bug waiting for a directory name.

use std::path::PathBuf;

use cybou_capsule::backend::{Bubblewrap, CapsuleBackend, CapsuleRuntimeBindings};
use cybou_capsule::compile::compile;
use cybou_capsule::grant::{
    CapsuleGrant, ModelGrant, NetworkGrant, ResourceBudget, SpendPolicy, Workspace,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let workspace = arguments
        .next()
        .ok_or("usage: capsule-argv <workspace> [program …]")?;
    let program: Vec<String> = arguments.collect();
    if program.is_empty() {
        return Err("no program given; a capsule with nothing to run is not a test".into());
    }

    let hosts: Vec<String> = std::env::var("CYBOU_EGRESS_HOSTS")
        .ok()
        .into_iter()
        .flat_map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|host| !host.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .collect();
    let grant = CapsuleGrant {
        capsule_id: uuid::Uuid::from_u128(0x9a75),
        agent: "gate".to_owned(),
        workspace: Workspace::at(PathBuf::from(workspace)),
        network: NetworkGrant { hosts },
        budget: ResourceBudget {
            memory_mib: 512,
            cpus: 1,
            tasks_max: 64,
            lifetime: time::Duration::minutes(2),
        },
        model: std::env::var("CYBOU_MODEL_CLASS")
            .ok()
            .map(|class| ModelGrant {
                class,
                spend: SpendPolicy::Capped(100),
            }),
        tools: Vec::new(),
        may_execute: true,
    };

    let spec = compile(&grant)?;
    // Named rather than guessed at. An example that fell back to some plausible location would
    // build a capsule around an entry program that might not be the one just compiled, and the
    // difference would not show up until a barrier was missing.
    let entry = std::env::var("CYBOU_CAPSULE_ENTRY")
        .map_err(|_| "CYBOU_CAPSULE_ENTRY must name the entry program on this host")?;

    let mut backend = Bubblewrap::entering_through(entry);
    let mut bindings = CapsuleRuntimeBindings::default();
    if !grant.network.hosts.is_empty() {
        let bridge = std::env::var("CYBOU_EGRESS_BRIDGE")
            .map_err(|_| "CYBOU_EGRESS_BRIDGE must name the bridge program on this host")?;
        let socket = std::env::var("CYBOU_EGRESS_SOCKET")
            .map_err(|_| "CYBOU_EGRESS_SOCKET must name this capsule's broker socket")?;
        backend = backend.with_egress_bridge(bridge);
        bindings.egress_socket_host = Some(PathBuf::from(socket));
    }
    if grant.model.is_some() {
        let bridge = std::env::var("CYBOU_MODEL_BRIDGE")
            .map_err(|_| "CYBOU_MODEL_BRIDGE must name the model bridge on this host")?;
        let socket = std::env::var("CYBOU_MODEL_SOCKET")
            .map_err(|_| "CYBOU_MODEL_SOCKET must name this capsule's gateway socket")?;
        let token = std::env::var("CYBOU_MODEL_TOKEN_FILE")
            .map_err(|_| "CYBOU_MODEL_TOKEN_FILE must name this capsule's lease-token file")?;
        backend = backend.with_model_bridge(bridge);
        bindings.model_socket_host = Some(PathBuf::from(socket));
        bindings.model_token_host = Some(PathBuf::from(token));
    }
    for argument in backend.command(&spec, &bindings, &program)? {
        println!("{argument}");
    }
    Ok(())
}
