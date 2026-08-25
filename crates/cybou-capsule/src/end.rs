// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Making the end of a lease physical.
//!
//! [`crate::lease`] produces the first half of an ending: after it, no request is `Allowed`. That is
//! a decision, and a decision is not a stop. An agent that has stopped being permitted anything is
//! still running, still holding memory, still writing to its workspace, still talking to whatever it
//! had already opened. This module produces the second half.
//!
//! ## Ending is not asking
//!
//! So: `SIGKILL`, and never `SIGTERM`. A termination signal is a request for a process to wind
//! itself up, honoured at the process's discretion — which is precisely the discretion an untrusted
//! agent does not have here. A capsule that could decline to end by trapping a signal would have a
//! lifetime that lasts as long as it likes, and a lifetime like that is not a lifetime.
//!
//! For the same reason the ending goes to the cgroup and not to a process. The capsule's first
//! process is not the capsule; killing it and calling the capsule ended would leave everything it
//! forked running, reparented and now unnamed.
//!
//! ## Freeze first, then kill
//!
//! Not for tidiness. Killing a live process tree is a race the tree can win: a capsule under a task
//! ceiling of a few hundred can fork faster than the signals arrive, and each new process is one the
//! kill pass has already gone past. Freezing the cgroup ends that race before it starts — nothing
//! runs, so nothing forks, and the kill that follows is against a set that cannot change.
//!
//! This is only sound because `SIGKILL` reaches a frozen cgroup, which is a kernel behaviour and not
//! an obvious one — if it did not, this sequence would leave a capsule alive forever while every
//! record said it had been ended, which is the worst available way for this to be wrong. It was
//! checked rather than assumed, on a frozen unit, by reading `cgroup.procs` afterwards:
//!
//! ```text
//! cgroup.freeze = 1  ->  kill --signal=SIGKILL  ->  ActiveState=inactive, cgroup.procs empty
//! ```
//!
//! `scripts/test-capsule-gate.sh` repeats it, so a kernel where that stops being true is a gate
//! failure rather than a surprise.
//!
//! ## Why the reason for ending does not appear here
//!
//! [`crate::lease::Ended`] has two variants and they matter — an operator told their agent ran out of
//! time when in fact somebody stopped it has been told the wrong thing. But they differ in what is
//! *recorded*, not in what is *done*. There is one way for a capsule to end, and a design offering a
//! gentler ending for the gentler reason would be offering a lease that expires more politely than it
//! is revoked, which is a lease an agent could prefer to reach.
//!
//! ## There is no thaw
//!
//! Deliberately. Nothing in this design resumes a capsule, because nothing in it un-ends a lease: a
//! withdrawn grant is not withheld pending reconsideration. A thaw here would be a way to restart
//! something that had been stopped, reachable by anything that could reach the stopping.

use crate::spec::KernelCapsuleSpec;
use crate::supervise::unit_name;

/// Everything that has to happen for a capsule to be over, in order.
///
/// Run each in turn. The first stops the capsule running; the second stops it existing. Stopping
/// after the first would leave a frozen capsule holding its memory ceiling forever.
#[must_use]
pub fn end(spec: &KernelCapsuleSpec) -> [Vec<String>; 2] {
    [freeze(spec), kill(spec)]
}

/// Stop every process in the capsule, without ending any of them.
///
/// On its own this is not an ending — it is what makes the ending that follows unraceable.
#[must_use]
pub fn freeze(spec: &KernelCapsuleSpec) -> Vec<String> {
    vec![
        "systemctl".to_owned(),
        "--user".to_owned(),
        "freeze".to_owned(),
        service_name(spec),
    ]
}

/// End the capsule.
#[must_use]
pub fn kill(spec: &KernelCapsuleSpec) -> Vec<String> {
    vec![
        "systemctl".to_owned(),
        "--user".to_owned(),
        "kill".to_owned(),
        // The whole cgroup. The default reaches the main process, and a capsule's main process is
        // not the capsule — everything it forked would carry on, reparented and no longer named by
        // anything.
        "--kill-whom=all".to_owned(),
        // Not SIGTERM. A termination signal is a request honoured at the recipient's discretion, and
        // the recipient here is the party whose discretion this whole crate exists to bound.
        "--signal=SIGKILL".to_owned(),
        service_name(spec),
    ]
}

/// The unit as systemd is addressed about it.
fn service_name(spec: &KernelCapsuleSpec) -> String {
    format!("{}.service", unit_name(spec))
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

    #[test]
    fn a_capsule_is_not_asked_to_end() {
        // A capsule that could decline by trapping a signal would have a lifetime lasting as long as
        // it liked, which is not a lifetime.
        let kill = kill(&spec());
        assert!(kill.iter().any(|item| item == "--signal=SIGKILL"));
        assert!(!kill.iter().any(|item| item.contains("SIGTERM")));
        assert!(!kill.iter().any(|item| item.contains("SIGINT")));
    }

    #[test]
    fn the_ending_reaches_the_cgroup_and_not_a_process() {
        // Killing only the first process would leave everything it forked running, reparented and no
        // longer named by anything that could find it again.
        assert!(kill(&spec()).iter().any(|item| item == "--kill-whom=all"));
    }

    #[test]
    fn nothing_is_running_by_the_time_the_kill_arrives() {
        // Freezing first is not tidiness. A capsule under a task ceiling of a few hundred can fork
        // faster than signals arrive, and every new process is one the kill pass has gone past.
        let steps = end(&spec());
        assert!(steps[0].iter().any(|item| item == "freeze"));
        assert!(steps[1].iter().any(|item| item == "kill"));
    }

    #[test]
    fn a_frozen_capsule_is_not_a_finished_one() {
        // Stopping after the freeze would leave it holding its memory ceiling for as long as the
        // host is up. Both steps, or neither is an ending.
        assert_eq!(end(&spec()).len(), 2);
    }

    #[test]
    fn every_step_names_the_capsule_that_is_ending() {
        // An ending addressed to the wrong unit is an ending of somebody else's agent.
        let spec = spec();
        let expected = format!("{}.service", unit_name(&spec));
        for step in end(&spec) {
            assert!(step.contains(&expected), "{step:?} does not name the unit");
        }
    }

    #[test]
    fn ending_is_deterministic() {
        let spec = spec();
        let first = end(&spec);
        for _ in 0..8 {
            assert_eq!(end(&spec), first);
        }
    }
}
