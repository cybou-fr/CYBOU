// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Print how this crate ends a capsule: its unit name, and each step, one argument per line.
//!
//! Same reason as `capsule-argv`. The gate has to end a real capsule to find out whether ending
//! works, and a gate that ended it with a command written out in the shell script would be testing
//! the shell script — passing forever after the code stopped agreeing with it, which is the shape of
//! failure that matters most here: an ending that does not end anything looks identical in every
//! record to one that does.
//!
//! The capsule identity is the same fixed one `capsule-argv` uses, so the unit the gate starts and
//! the unit these steps address are the same unit.

use std::path::PathBuf;

use cybou_capsule::compile::compile;
use cybou_capsule::end::{freeze, kill};
use cybou_capsule::grant::{CapsuleGrant, NetworkGrant, ResourceBudget, Workspace};
use cybou_capsule::supervise::unit_name;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let step = std::env::args()
        .nth(1)
        .ok_or("usage: capsule-end <unit|freeze|kill>")?;

    let grant = CapsuleGrant {
        capsule_id: uuid::Uuid::from_u128(0x9a75),
        agent: "gate".to_owned(),
        workspace: Workspace::at(PathBuf::from("/tmp")),
        network: NetworkGrant::default(),
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

    let lines = match step.as_str() {
        "unit" => vec![format!("{}.service", unit_name(&spec))],
        "freeze" => freeze(&spec),
        "kill" => kill(&spec),
        other => return Err(format!("no such step: {other}").into()),
    };
    for line in lines {
        println!("{line}");
    }
    Ok(())
}
