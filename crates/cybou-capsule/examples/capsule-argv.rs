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

use cybou_capsule::backend::{Bubblewrap, CapsuleBackend};
use cybou_capsule::compile::compile;
use cybou_capsule::grant::{CapsuleGrant, NetworkGrant, ResourceBudget, Workspace};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let workspace = arguments
        .next()
        .ok_or("usage: capsule-argv <workspace> [program …]")?;
    let program: Vec<String> = arguments.collect();
    if program.is_empty() {
        return Err("no program given; a capsule with nothing to run is not a test".into());
    }

    let grant = CapsuleGrant {
        capsule_id: uuid::Uuid::from_u128(0x9a75),
        agent: "gate".to_owned(),
        workspace: Workspace::at(PathBuf::from(workspace)),
        // Named, and deliberately not honoured: the spec denies the network whatever a grant lists,
        // until an egress broker exists. The gate checks that the denial actually happens.
        network: NetworkGrant::to(&["github.com"]),
        budget: ResourceBudget {
            memory_mib: 512,
            cpus: 1,
            tasks_max: 64,
            lifetime: time::Duration::minutes(2),
        },
        model: None,
        tools: Vec::new(),
        may_execute: true,
    };

    let spec = compile(&grant)?;
    for argument in Bubblewrap.command(&spec, &program) {
        println!("{argument}");
    }
    Ok(())
}
