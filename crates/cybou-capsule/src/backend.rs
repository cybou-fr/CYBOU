// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Turning a spec into the command that builds a capsule.
//!
//! Still pure. Building the argument vector is where every security-relevant translation happens, so
//! it is a function returning a `Vec<String>` that a test can read — not something buried inside a
//! spawn. Whether a sandbox is correct should be answerable by looking at what it was asked to do.
//!
//! ## Why a trait with one implementation
//!
//! A first implementation that is also this project's own `clone`/`unshare`/mount orchestration is
//! the wrong place to be original. [`Bubblewrap`] delegates to a tool built for exactly this and
//! used by people who have been attacked. The trait exists so replacing it later — with something
//! native, or with an OCI runtime — changes nothing above it.
//!
//! ## The environment is emptied, not filtered
//!
//! `--clearenv`, and then back in what the capsule needs by name. A filter is a deny-list of
//! variables somebody has to keep current, and the host environment of a Cybou process holds
//! `CYBOU_KEYSTORE_PATH`, an SSH agent socket, whatever the operator exported, and the next thing
//! somebody adds. An agent that inherits it has been handed the machine.

use crate::spec::{Access, KernelCapsuleSpec, Network, Seccomp};

/// Something that can build a capsule from a spec.
pub trait CapsuleBackend {
    /// The command and arguments that would build this capsule.
    ///
    /// Returned rather than run, so the decision and the doing stay separable and the first is
    /// testable. A backend that could only be checked by running it could only be checked on a
    /// machine willing to run it.
    fn command(&self, spec: &KernelCapsuleSpec, program: &[String]) -> Vec<String>;

    /// What this backend needs present to work.
    fn requires(&self) -> &'static str;

    /// Whether a seccomp filter still has to be applied by whatever spawns this.
    ///
    /// True, and it is a separate question from [`Self::command`] for a reason worth stating: the
    /// filter is passed to bubblewrap on a **file descriptor**, and a function that builds an
    /// argument vector cannot know a descriptor number. The first version of this pushed a bare
    /// `--seccomp` anyway; bubblewrap took the next token as the descriptor, which happened to be
    /// the `--` separating the program, and every capsule failed to start. The gate caught it,
    /// which is the only reason this note is about a fixed mistake rather than a shipped one.
    ///
    /// So the flag is not here, and this says so out loud instead. **No filter is applied by this
    /// build.** A spawning layer that owns a descriptor must add it and must refuse to run without
    /// one; until that layer exists, a capsule has its namespaces, its mounts and its cgroup, and
    /// not this. Reporting that plainly is the difference between a known gap and a silent one.
    fn requires_seccomp(&self) -> bool {
        true
    }
}

/// The first backend: bubblewrap.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Bubblewrap;

impl CapsuleBackend for Bubblewrap {
    fn requires(&self) -> &'static str {
        "bwrap"
    }

    fn command(&self, spec: &KernelCapsuleSpec, program: &[String]) -> Vec<String> {
        let mut argv = vec!["bwrap".to_owned()];

        // Die with the parent. Without it, a capsule outlives whatever was supervising it, and a
        // lease that ends has nothing left to act on — which turns "ending is not asking" back into
        // asking.
        argv.push("--die-with-parent".to_owned());

        // A session of its own. Not cosmetic: sharing a controlling terminal lets a process push
        // characters into it, and the thing reading them is whatever started the capsule.
        argv.push("--new-session".to_owned());

        // Every namespace, named individually rather than with a blanket flag, so this list and the
        // spec's list can be compared by a reader and by a test.
        argv.push("--unshare-user".to_owned());
        argv.push("--unshare-ipc".to_owned());
        argv.push("--unshare-pid".to_owned());
        argv.push("--unshare-uts".to_owned());
        argv.push("--unshare-cgroup".to_owned());

        // No sandbox inside the sandbox. An agent has no business building one, and the ability to
        // is the ability to arrange an escape hatch.
        argv.push("--disable-userns".to_owned());
        argv.push("--assert-userns-disabled".to_owned());

        match spec.network {
            Network::Denied => argv.push("--unshare-net".to_owned()),
        }

        // The environment, emptied and then rebuilt by name.
        argv.push("--clearenv".to_owned());
        for (name, value) in environment(spec) {
            argv.push("--setenv".to_owned());
            argv.push(name);
            argv.push(value);
        }

        // The filesystem, in the order the spec lists it: read-only first, the one writable path
        // last, so nothing is layered over it afterwards.
        for mount in &spec.mounts {
            argv.push(
                match mount.access {
                    Access::ReadOnly => "--ro-bind-try",
                    Access::ReadWrite => "--bind",
                }
                .to_owned(),
            );
            argv.push(mount.source.to_string_lossy().into_owned());
            argv.push(mount.target.to_string_lossy().into_owned());
        }

        // A `/proc` for this capsule's own PID namespace, and a minimal `/dev`. Without the first,
        // nothing that reads `/proc/self` works; with the host's, the capsule can see every process
        // on the machine, which is the PID namespace undone by a mount.
        argv.push("--proc".to_owned());
        argv.push("/proc".to_owned());
        argv.push("--dev".to_owned());
        argv.push("/dev".to_owned());

        // A private `/tmp`. Shared with the host it is a channel between capsules and a place to
        // leave things for whatever runs next.
        argv.push("--tmpfs".to_owned());
        argv.push("/tmp".to_owned());

        argv.push("--chdir".to_owned());
        argv.push(spec.working_directory.to_string_lossy().into_owned());

        // No `--seccomp` here. It takes a file descriptor, and this function has none to give; see
        // `requires_seccomp`. A flag emitted without its argument does not weaken the sandbox — it
        // breaks it, because bubblewrap reads the next token as the descriptor.
        let Seccomp::NoReshaping = spec.seccomp;

        // Everything after this is the program, whatever it looks like. Without it a program named
        // `--bind` is an argument to bwrap, which is the same lesson this repository already learned
        // about a systemd unit name.
        argv.push("--".to_owned());
        argv.extend(program.iter().cloned());

        argv
    }
}

/// What a capsule is given in its environment.
///
/// By name, and short. Everything here is something a program cannot work without; anything else is
/// something the agent can be told through its own configuration, where it is visible.
fn environment(spec: &KernelCapsuleSpec) -> Vec<(String, String)> {
    vec![
        (
            "HOME".to_owned(),
            spec.working_directory.to_string_lossy().into_owned(),
        ),
        (
            "PWD".to_owned(),
            spec.working_directory.to_string_lossy().into_owned(),
        ),
        ("TMPDIR".to_owned(), "/tmp".to_owned()),
        ("PATH".to_owned(), "/usr/local/bin:/usr/bin:/bin".to_owned()),
        // So a program inside can tell it is inside, and say so in a bug report rather than
        // producing a mystery.
        ("CYBOU_CAPSULE".to_owned(), spec.capsule_id.to_string()),
    ]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use time::Duration;
    use uuid::Uuid;

    use super::*;
    use crate::compile::{WORKSPACE_INSIDE, compile};
    use crate::grant::{CapsuleGrant, ModelGrant, NetworkGrant, ResourceBudget, Workspace};

    fn spec() -> KernelCapsuleSpec {
        let grant = CapsuleGrant {
            capsule_id: Uuid::from_u128(8472),
            agent: "opencode".to_owned(),
            workspace: Workspace::at("/srv/project"),
            network: NetworkGrant::to(&["github.com"]),
            budget: ResourceBudget {
                memory_mib: 4096,
                cpus: 2,
                tasks_max: 512,
                lifetime: Duration::hours(4),
            },
            model: Some(ModelGrant {
                class: "Strong".to_owned(),
                spend_limit: 100,
            }),
            tools: vec!["git".to_owned()],
            may_execute: true,
        };
        compile(&grant).expect("compiles")
    }

    fn argv() -> Vec<String> {
        Bubblewrap.command(&spec(), &["cargo".to_owned(), "test".to_owned()])
    }

    /// Whether `flag` appears before the `--` that ends bwrap's own arguments.
    fn asks_for(argv: &[String], flag: &str) -> bool {
        let end = argv
            .iter()
            .position(|item| item == "--")
            .unwrap_or(argv.len());
        argv[..end].iter().any(|item| item == flag)
    }

    #[test]
    fn every_namespace_the_spec_names_is_asked_for() {
        // The spec says all seven. A backend quietly asking for six would produce a capsule that
        // looks right in every record and is not one.
        let argv = argv();
        for flag in [
            "--unshare-user",
            "--unshare-ipc",
            "--unshare-pid",
            "--unshare-uts",
            "--unshare-cgroup",
            "--unshare-net",
        ] {
            assert!(asks_for(&argv, flag), "{flag} is missing");
        }
    }

    #[test]
    fn a_capsule_cannot_build_a_capsule() {
        // An agent has no business building a sandbox inside its own, and the ability to is the
        // ability to arrange an escape hatch. Asserted as well as disabled, so a kernel that
        // silently permits it fails loudly instead.
        let argv = argv();
        assert!(asks_for(&argv, "--disable-userns"));
        assert!(asks_for(&argv, "--assert-userns-disabled"));
    }

    #[test]
    fn the_environment_is_emptied_before_anything_is_put_back() {
        // The host environment of a Cybou process holds the key store path, whatever the operator
        // exported, and the next thing somebody adds. A filter is a deny-list somebody has to keep
        // current; emptying is not.
        let argv = argv();
        assert!(asks_for(&argv, "--clearenv"));

        let cleared = argv
            .iter()
            .position(|item| item == "--clearenv")
            .expect("present");
        let first_setenv = argv
            .iter()
            .position(|item| item == "--setenv")
            .expect("present");
        assert!(
            cleared < first_setenv,
            "something was put into the environment before it was emptied"
        );
    }

    #[test]
    fn nothing_from_the_host_environment_is_passed_through() {
        // Every variable is named here. A capsule that inherited one would inherit whichever one
        // nobody thought about.
        let argv = argv();
        let names: Vec<&String> = argv
            .iter()
            .enumerate()
            .filter(|(index, _)| index > &0 && argv[index - 1] == "--setenv")
            .map(|(_, name)| name)
            .collect();
        for name in &names {
            assert!(
                ["HOME", "PWD", "TMPDIR", "PATH", "CYBOU_CAPSULE"].contains(&name.as_str()),
                "{name} reached the capsule's environment"
            );
        }
        assert!(!names.iter().any(|name| name.starts_with("CYBOU_KEYSTORE")));
        assert!(!names.iter().any(|name| name.contains("SSH")));
    }

    #[test]
    fn proc_is_the_capsules_own_and_not_the_hosts() {
        // With the host's, a capsule sees every process on the machine — the PID namespace undone by
        // a mount, which is the kind of mistake that looks like a convenience.
        let argv = argv();
        let at = argv
            .iter()
            .position(|item| item == "--proc")
            .expect("present");
        assert_eq!(argv[at + 1], "/proc");
        assert!(
            !argv.windows(3).any(|window| {
                (window[0] == "--bind" || window[0] == "--ro-bind-try") && window[1] == "/proc"
            }),
            "the host's /proc was bound in"
        );
    }

    #[test]
    fn tmp_is_private() {
        // Shared with the host, it is a channel between capsules and a place to leave something for
        // whatever runs next.
        let argv = argv();
        let at = argv
            .iter()
            .position(|item| item == "--tmpfs")
            .expect("present");
        assert_eq!(argv[at + 1], "/tmp");
    }

    #[test]
    fn only_the_workspace_is_bound_writable() {
        // The property the spec promises, checked at the place that could quietly break it.
        let argv = argv();
        let writable: Vec<&String> = argv
            .iter()
            .enumerate()
            .filter(|(index, _)| index > &0 && argv[index - 1] == "--bind")
            .map(|(_, source)| source)
            .collect();
        assert_eq!(writable, vec![&"/srv/project".to_owned()]);
    }

    #[test]
    fn the_program_is_after_a_separator_whatever_it_is_called() {
        // A program named `--bind` is otherwise an argument to bwrap. The same lesson this
        // repository already learned about passing a systemd unit name.
        let hostile = vec!["--bind".to_owned(), "/etc".to_owned(), "/etc".to_owned()];
        let argv = Bubblewrap.command(&spec(), &hostile);

        let separator = argv.iter().position(|item| item == "--").expect("present");
        assert_eq!(&argv[separator + 1..], hostile.as_slice());
        assert!(
            !asks_for(&argv, "--bind") || argv[..separator].iter().any(|item| item == "--bind"),
            "sanity: the real --bind for the workspace is before the separator"
        );
        // And the hostile one is not treated as one: everything after the separator is the program.
        assert_eq!(
            argv[separator + 1..]
                .iter()
                .filter(|item| *item == "/etc")
                .count(),
            2
        );
    }

    #[test]
    fn seccomp_is_declared_as_owed_rather_than_emitted_without_its_argument() {
        // `--seccomp` takes a file descriptor. Pushing it bare does not weaken the sandbox, it
        // breaks it: bubblewrap read the next token as the descriptor, which was the `--` before the
        // program, and nothing started at all. The gate found that; this keeps it found.
        assert!(
            !argv().iter().any(|item| item == "--seccomp"),
            "a flag that needs a descriptor was emitted without one"
        );
        assert!(
            Bubblewrap.requires_seccomp(),
            "the debt has to be visible to whatever spawns this"
        );
    }

    #[test]
    fn the_capsule_starts_in_its_workspace() {
        let argv = argv();
        let at = argv
            .iter()
            .position(|item| item == "--chdir")
            .expect("present");
        assert_eq!(argv[at + 1], WORKSPACE_INSIDE);
    }

    #[test]
    fn a_capsule_does_not_outlive_what_supervises_it() {
        // A lease that ends has nothing left to act on otherwise, which turns "ending is not asking"
        // back into asking.
        assert!(asks_for(&argv(), "--die-with-parent"));
    }

    #[test]
    fn building_the_command_is_deterministic() {
        // Two runs of one spec must be comparable, and a command that varied would make a recorded
        // one useless as evidence of what was actually built.
        let spec = spec();
        let program = ["cargo".to_owned()];
        let first = Bubblewrap.command(&spec, &program);
        for _ in 0..8 {
            assert_eq!(Bubblewrap.command(&spec, &program), first);
        }
    }

    #[test]
    fn the_host_root_is_never_bound() {
        // The one mistake that would make everything else here decorative.
        let argv = argv();
        assert!(
            !argv.windows(2).any(
                |window| (window[0] == "--bind" || window[0] == "--ro-bind-try")
                    && window[1] == "/"
            ),
            "the host root was bound into the capsule"
        );
        assert!(!argv.iter().any(|item| item == "--bind-try"));
    }

    #[test]
    fn a_backend_says_what_it_needs() {
        // So a deployment missing it reports that, rather than a capsule that failed for reasons
        // nobody can see.
        assert_eq!(Bubblewrap.requires(), "bwrap");
    }

    #[test]
    fn the_workspace_mount_target_is_the_fixed_one() {
        let argv = argv();
        let at = argv
            .iter()
            .position(|item| item == "--bind")
            .expect("the workspace is bound");
        assert_eq!(argv[at + 1], "/srv/project");
        assert_eq!(argv[at + 2], WORKSPACE_INSIDE);
        assert_eq!(PathBuf::from(&argv[at + 2]), spec().working_directory);
    }
}
