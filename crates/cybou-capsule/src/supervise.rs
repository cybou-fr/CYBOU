// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Putting a capsule under a resource ceiling that the kernel keeps.
//!
//! The sandbox and the budget are different mechanisms. Bubblewrap shapes what a capsule can *see*;
//! a cgroup decides what it can *consume*, and neither substitutes for the other — a capsule with a
//! perfect filesystem view can still take the machine down with a fork bomb.
//!
//! Still pure: this builds an argument vector, so what a capsule was held to is answerable by
//! reading rather than by running.
//!
//! ## A transient service, not a scope
//!
//! This is a measured decision and not a stylistic one. Asked for the same limits:
//!
//! ```text
//! systemd-run --user --scope -p MemoryMax=64M   ->  MemoryMax=infinity
//! systemd-run --user --unit=… -p MemoryMax=64M  ->  memory.max=67108864
//! ```
//!
//! A user scope accepted the properties, reported success, and enforced nothing. That is the worst
//! available outcome for a limit: a `--scope` implementation would have looked correct in the code,
//! in the command, and in every record, and held a capsule to nothing at all. The service form was
//! checked against the kernel's own `memory.max`, `pids.max` and `cpu.max` rather than against what
//! systemd said about itself, for exactly that reason.
//!
//! ## The lifetime belongs to the unit
//!
//! `RuntimeMaxSec`, not a timer in Mind. A lifetime enforced by something that has to still be
//! running is a lifetime that ends when that thing does — and *ending is not asking* means the end
//! must not depend on anybody being there to ask.

use crate::spec::KernelCapsuleSpec;

/// Wrap a capsule's command in a transient unit that the kernel holds to the budget.
///
/// The returned vector runs `command` under a cgroup carrying the spec's ceilings.
#[must_use]
pub fn under_budget(spec: &KernelCapsuleSpec, command: &[String]) -> Vec<String> {
    let mut argv = vec![
        "systemd-run".to_owned(),
        "--user".to_owned(),
        // A transient service. Not `--scope`: see the note above, where a scope accepted these
        // properties and enforced none of them.
        format!("--unit={}", unit_name(spec)),
        // Do not leave a failed unit behind. Without this a capsule that dies badly stays in the
        // manager's list until somebody resets it, and the next capsule with the same name refuses
        // to start — which reads as the sandbox being broken rather than as tidying not having
        // happened.
        "--collect".to_owned(),
        // Wait for it, and give back what it exited with. A supervisor that returned as soon as the
        // unit was accepted would report success for a capsule that failed a second later.
        "--wait".to_owned(),
    ];

    for property in properties(spec) {
        argv.push("--property".to_owned());
        argv.push(property);
    }

    // Everything after this is the command. Same reason as everywhere else in this crate: a program
    // whose name begins with a dash is a program, not an option.
    argv.push("--".to_owned());
    argv.extend(command.iter().cloned());
    argv
}

/// The unit name for one capsule.
///
/// Derived from the capsule's identity so a unit in the manager's list can be traced back to what it
/// was running, and so two capsules never collide. A UUID's hyphens and hex digits are all systemd
/// accepts without escaping, which is why the identity is used directly and the agent's name — which
/// a person chose — is not.
#[must_use]
pub fn unit_name(spec: &KernelCapsuleSpec) -> String {
    format!("cybou-capsule-{}", spec.capsule_id)
}

/// The ceilings, as systemd spells them.
fn properties(spec: &KernelCapsuleSpec) -> Vec<String> {
    vec![
        format!("MemoryMax={}M", spec.cgroup.memory_mib),
        // Memory that cannot be swapped out from under the ceiling. Without it a capsule at its
        // memory limit pushes the host into swap instead of being stopped, which is the limit
        // failing in the direction that hurts the machine rather than the capsule.
        format!("MemorySwapMax=0"),
        format!("TasksMax={}", spec.cgroup.tasks_max),
        // A percentage of one CPU per CPU granted. 200% is two.
        format!("CPUQuota={}%", spec.cgroup.cpus.saturating_mul(100)),
        format!("RuntimeMaxSec={}", spec.cgroup.runtime_max_seconds),
    ]
}

#[cfg(test)]
mod tests {
    use time::Duration;
    use uuid::Uuid;

    use super::*;
    use crate::compile::compile;
    use crate::grant::{CapsuleGrant, NetworkGrant, ResourceBudget, Workspace};

    fn spec() -> KernelCapsuleSpec {
        compile(&CapsuleGrant {
            capsule_id: Uuid::from_u128(8472),
            agent: "opencode".to_owned(),
            workspace: Workspace::at("/srv/project"),
            network: NetworkGrant::default(),
            budget: ResourceBudget {
                memory_mib: 4096,
                cpus: 2,
                tasks_max: 512,
                lifetime: Duration::hours(4),
            },
            model: None,
            tools: Vec::new(),
            may_execute: true,
        })
        .expect("compiles")
    }

    fn argv() -> Vec<String> {
        under_budget(
            &spec(),
            &["bwrap".to_owned(), "--".to_owned(), "sh".to_owned()],
        )
    }

    fn property(argv: &[String], name: &str) -> Option<String> {
        argv.iter()
            .zip(argv.iter().skip(1))
            .find(|(flag, value)| *flag == "--property" && value.starts_with(name))
            .map(|(_, value)| value.clone())
    }

    #[test]
    fn a_capsule_runs_as_a_service_and_never_as_a_scope() {
        // Measured, not stylistic. A user scope accepted MemoryMax, reported success, and enforced
        // nothing — the worst available outcome for a limit, because it looks correct everywhere.
        let argv = argv();
        assert!(!argv.iter().any(|item| item == "--scope"));
        assert!(argv.iter().any(|item| item.starts_with("--unit=")));
    }

    #[test]
    fn every_ceiling_the_budget_names_reaches_systemd() {
        let argv = argv();
        assert_eq!(
            property(&argv, "MemoryMax"),
            Some("MemoryMax=4096M".to_owned())
        );
        assert_eq!(property(&argv, "TasksMax"), Some("TasksMax=512".to_owned()));
        assert_eq!(
            property(&argv, "CPUQuota"),
            Some("CPUQuota=200%".to_owned())
        );
        assert_eq!(
            property(&argv, "RuntimeMaxSec"),
            Some("RuntimeMaxSec=14400".to_owned())
        );
    }

    #[test]
    fn a_capsule_at_its_memory_limit_is_stopped_rather_than_pushed_into_swap() {
        // Otherwise the limit fails in the direction that hurts the machine instead of the capsule:
        // the host starts swapping and everything on it slows down, while the capsule carries on.
        assert_eq!(
            property(&argv(), "MemorySwapMax"),
            Some("MemorySwapMax=0".to_owned())
        );
    }

    #[test]
    fn the_lifetime_is_the_units_and_not_a_timer_somewhere_else() {
        // A lifetime enforced by something that has to still be running ends when that thing does,
        // and "ending is not asking" means the end must not depend on anybody being there.
        assert!(property(&argv(), "RuntimeMaxSec").is_some());
    }

    #[test]
    fn the_unit_is_named_after_the_capsule_and_not_after_anything_a_person_typed() {
        // A UUID's hex and hyphens are all systemd accepts unescaped. An agent's name is chosen by a
        // person and would need quoting nobody would remember to do.
        let spec = spec();
        let name = unit_name(&spec);
        assert!(name.contains(&spec.capsule_id.to_string()));
        assert!(!name.contains("opencode"));
        assert!(
            name.chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-'),
            "{name} would need escaping"
        );
    }

    #[test]
    fn the_supervisor_waits_for_what_it_started() {
        // Returning as soon as the unit was accepted would report success for a capsule that failed
        // a second later.
        assert!(argv().iter().any(|item| item == "--wait"));
    }

    #[test]
    fn a_failed_capsule_does_not_block_the_next_one() {
        // Without --collect a unit that died badly stays in the manager's list, and the next capsule
        // with that name refuses to start — which reads as the sandbox being broken rather than as
        // tidying not having happened.
        assert!(argv().iter().any(|item| item == "--collect"));
    }

    #[test]
    fn the_command_is_after_a_separator() {
        let hostile = vec!["--property".to_owned(), "MemoryMax=infinity".to_owned()];
        let argv = under_budget(&spec(), &hostile);
        let separator = argv.iter().position(|item| item == "--").expect("present");
        assert_eq!(&argv[separator + 1..], hostile.as_slice());
        // And the smuggled property is not one: everything before the separator came from the spec.
        assert_eq!(
            property(&argv, "MemoryMax"),
            Some("MemoryMax=4096M".to_owned())
        );
    }

    #[test]
    fn building_the_supervision_is_deterministic() {
        let spec = spec();
        let command = ["bwrap".to_owned()];
        let first = under_budget(&spec, &command);
        for _ in 0..8 {
            assert_eq!(under_budget(&spec, &command), first);
        }
    }
}
