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

use std::path::{Path, PathBuf};

use crate::spec::{Access, KernelCapsuleSpec, ModelChannel, Network, Seccomp};

/// Where the entry program is bound inside a capsule.
///
/// On the capsule root, which bubblewrap makes as a tmpfs, and not beneath `/usr`: `/usr` is bound
/// read-only, so there is nowhere under it to create a mount point and bubblewrap refuses the whole
/// capsule rather than half of it. That refusal was the right one and it is worth keeping the reason
/// visible, because the obvious fix — binding `/usr` writable — would hand an agent the compiler it
/// is about to run.
///
/// It needs no Landlock rule of its own. It has already been executed by the time any rule is
/// applied, and it is bound read-only over a root the ruleset never grants.
pub const ENTRY_INSIDE: &str = "/.cybou-capsule-enter";

/// Where the capsule-local compatibility bridge is bound read-only.
pub const EGRESS_BRIDGE_INSIDE: &str = "/.cybou-egress-bridge";

/// Where the capsule-local model compatibility bridge is bound read-only.
pub const MODEL_BRIDGE_INSIDE: &str = "/.cybou-model-bridge";

/// Host paths chosen when one capsule is started, kept separate from the human grant and its
/// deterministic kernel spec.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CapsuleRuntimeBindings {
    /// The one broker socket created for this capsule.
    pub egress_socket_host: Option<PathBuf>,
    /// The model-gateway socket created for this capsule.
    pub model_socket_host: Option<PathBuf>,
    /// A private file containing only this capsule's ephemeral lease token.
    pub model_token_host: Option<PathBuf>,
}

/// Why a backend could not translate a complete spec into a command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendError {
    /// Brokered network was compiled but no bridge executable was installed.
    MissingEgressBridge,
    /// Brokered network was compiled but this capsule has no broker socket.
    MissingEgressSocket,
    /// A model grant was compiled but no model bridge executable was installed.
    MissingModelBridge,
    /// A model grant was compiled but this capsule has no gateway socket.
    MissingModelSocket,
    /// A model grant was compiled but this capsule has no lease-token file.
    MissingModelToken,
}

impl core::fmt::Display for BackendError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingEgressBridge => {
                write!(formatter, "brokered network needs an egress bridge")
            }
            Self::MissingEgressSocket => {
                write!(formatter, "brokered network needs a broker socket")
            }
            Self::MissingModelBridge => write!(formatter, "a model grant needs a model bridge"),
            Self::MissingModelSocket => write!(formatter, "a model grant needs a gateway socket"),
            Self::MissingModelToken => write!(formatter, "a model grant needs a lease-token file"),
        }
    }
}

impl core::error::Error for BackendError {}

/// Something that can build a capsule from a spec.
pub trait CapsuleBackend {
    /// The command and arguments that would build this capsule.
    ///
    /// Returned rather than run, so the decision and the doing stay separable and the first is
    /// testable. A backend that could only be checked by running it could only be checked on a
    /// machine willing to run it.
    ///
    /// # Errors
    ///
    /// Returns [`BackendError`] when a brokered spec lacks its bridge executable or per-capsule
    /// runtime socket binding.
    fn command(
        &self,
        spec: &KernelCapsuleSpec,
        bindings: &CapsuleRuntimeBindings,
        program: &[String],
    ) -> Result<Vec<String>, BackendError>;

    /// What this backend needs present to work.
    fn requires(&self) -> &'static str;

    /// Whether a seccomp filter still has to be applied by whatever spawns this.
    ///
    /// It is a separate question from [`Self::command`] for a reason worth keeping: bubblewrap takes
    /// its filter on a **file descriptor**, and a function that builds an argument vector cannot
    /// know a descriptor number. The first version pushed a bare `--seccomp` anyway; bubblewrap read
    /// the next token as the descriptor, which happened to be the `--` separating the program, and
    /// every capsule failed to start. The gate caught it, which is the only reason this note is
    /// about a fixed mistake rather than a shipped one.
    ///
    /// The debt was not paid by finding a descriptor. It was paid by the filter moving to where it
    /// always belonged: `cybou-capsule-enter` installs it on itself just before becoming the agent,
    /// which is also the only place Landlock can go. A backend that still needs one from outside
    /// says so here, and the default is the safe answer for a backend that has not thought about it.
    fn requires_seccomp(&self) -> bool {
        true
    }
}

/// The first backend: bubblewrap, entered through this project's own entry program.
///
/// It carries the entry program's location on the host and there is no constructor without one.
/// Two of a capsule's ten parts — Landlock and seccomp — are restrictions a process applies to
/// itself just before `exec`, which no command line can express. A `Bubblewrap` that could be built
/// without knowing where that program lives would be a type able to describe a capsule with half its
/// barriers, and the half it was missing would not appear anywhere in the command it produced.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bubblewrap {
    entry: PathBuf,
    egress_bridge: Option<PathBuf>,
    model_bridge: Option<PathBuf>,
}

impl Bubblewrap {
    /// A backend that enters through the entry program at this path on the host.
    #[must_use]
    pub fn entering_through(entry: impl Into<PathBuf>) -> Self {
        Self {
            entry: entry.into(),
            egress_bridge: None,
            model_bridge: None,
        }
    }

    /// Install the capsule-local transport used only by brokered specs.
    #[must_use]
    pub fn with_egress_bridge(mut self, bridge: impl Into<PathBuf>) -> Self {
        self.egress_bridge = Some(bridge.into());
        self
    }

    /// Install the capsule-local transport used only by model-granted specs.
    #[must_use]
    pub fn with_model_bridge(mut self, bridge: impl Into<PathBuf>) -> Self {
        self.model_bridge = Some(bridge.into());
        self
    }

    /// Where the entry program is on the host.
    #[must_use]
    pub fn entry(&self) -> &Path {
        &self.entry
    }
}

impl CapsuleBackend for Bubblewrap {
    fn requires_seccomp(&self) -> bool {
        // The entry program this backend runs installs the filter itself, on itself, before it
        // becomes the agent — so there is nothing left for a spawning layer to add. This is only
        // allowed to be false because the gate kills a capsule that calls `unshare` and reads the
        // signal, rather than because the code above says so.
        false
    }

    fn requires(&self) -> &'static str {
        "bwrap, and this project's own cybou-capsule-enter"
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the security-relevant bwrap argument order is intentionally visible in one translation"
    )]
    fn command(
        &self,
        spec: &KernelCapsuleSpec,
        bindings: &CapsuleRuntimeBindings,
        program: &[String],
    ) -> Result<Vec<String>, BackendError> {
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

        // Both modes get a fresh namespace with no route. Brokered means one loopback listener can
        // reach one pathname socket; it never means sharing or routing the host network.
        argv.push("--unshare-net".to_owned());

        // The environment, emptied and then rebuilt by name.
        argv.push("--clearenv".to_owned());
        for (name, value) in environment(spec) {
            argv.push("--setenv".to_owned());
            argv.push(name);
            argv.push(value);
        }

        let brokered = match &spec.network {
            Network::Denied => None,
            Network::Brokered {
                proxy_port,
                socket_inside,
            } => Some((
                *proxy_port,
                socket_inside,
                self.egress_bridge
                    .as_ref()
                    .ok_or(BackendError::MissingEgressBridge)?,
                bindings
                    .egress_socket_host
                    .as_ref()
                    .ok_or(BackendError::MissingEgressSocket)?,
            )),
        };
        let model = match &spec.model {
            ModelChannel::Denied => None,
            ModelChannel::Brokered {
                proxy_port,
                socket_inside,
                token_inside,
            } => Some((
                *proxy_port,
                socket_inside,
                token_inside,
                self.model_bridge
                    .as_ref()
                    .ok_or(BackendError::MissingModelBridge)?,
                bindings
                    .model_socket_host
                    .as_ref()
                    .ok_or(BackendError::MissingModelSocket)?,
                bindings
                    .model_token_host
                    .as_ref()
                    .ok_or(BackendError::MissingModelToken)?,
            )),
        };

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

        if brokered.is_some() || model.is_some() {
            argv.push("--dir".to_owned());
            argv.push("/run".to_owned());
            argv.push("--dir".to_owned());
            argv.push("/run/cybou".to_owned());
        }
        if let Some((_, socket_inside, _, socket_host)) = brokered {
            argv.push("--bind".to_owned());
            argv.push(socket_host.to_string_lossy().into_owned());
            argv.push(socket_inside.to_string_lossy().into_owned());
        }
        if let Some((_, socket_inside, token_inside, _, socket_host, token_host)) = model {
            argv.push("--bind".to_owned());
            argv.push(socket_host.to_string_lossy().into_owned());
            argv.push(socket_inside.to_string_lossy().into_owned());
            argv.push("--ro-bind".to_owned());
            argv.push(token_host.to_string_lossy().into_owned());
            argv.push(token_inside.to_string_lossy().into_owned());
        }

        // The entry program, read-only, layered over the `/usr` bound above. Last of the binds so
        // nothing the spec lists can be placed on top of it.
        argv.push("--ro-bind".to_owned());
        argv.push(self.entry.to_string_lossy().into_owned());
        argv.push(ENTRY_INSIDE.to_owned());

        if let Some((_, _, bridge, _)) = brokered {
            argv.push("--ro-bind".to_owned());
            argv.push(bridge.to_string_lossy().into_owned());
            argv.push(EGRESS_BRIDGE_INSIDE.to_owned());
        }
        if let Some((_, _, _, bridge, _, _)) = model {
            argv.push("--ro-bind".to_owned());
            argv.push(bridge.to_string_lossy().into_owned());
            argv.push(MODEL_BRIDGE_INSIDE.to_owned());
        }

        argv.push("--chdir".to_owned());
        argv.push(spec.working_directory.to_string_lossy().into_owned());

        // No `--seccomp` here. It takes a file descriptor, and this function has none to give; see
        // `requires_seccomp`. A flag emitted without its argument does not weaken the sandbox — it
        // breaks it, because bubblewrap reads the next token as the descriptor.
        let Seccomp::NoReshaping = spec.seccomp;

        // Everything after this is what bubblewrap runs, whatever it looks like. Without it a
        // program named `--bind` is an argument to bwrap, which is the same lesson this repository
        // already learned about a systemd unit name.
        argv.push("--".to_owned());

        // And what it runs is the entry program, not the agent. The agent comes after the entry
        // program's own separator, which is why there are two of them on this line: the first ends
        // bubblewrap's arguments, the second ends the entry program's.
        argv.push(ENTRY_INSIDE.to_owned());
        for rule in &spec.landlock {
            argv.push(
                match rule.access {
                    Access::ReadOnly => "--ro",
                    Access::ReadWrite => "--rw",
                }
                .to_owned(),
            );
            argv.push(rule.path.to_string_lossy().into_owned());
        }
        if let Some((proxy_port, socket_inside, _, _)) = brokered {
            argv.push("--ro".to_owned());
            argv.push(EGRESS_BRIDGE_INSIDE.to_owned());
            argv.push("--egress-bridge".to_owned());
            argv.push(EGRESS_BRIDGE_INSIDE.to_owned());
            argv.push("--egress-socket".to_owned());
            argv.push(socket_inside.to_string_lossy().into_owned());
            argv.push("--egress-port".to_owned());
            argv.push(proxy_port.to_string());
        }
        if let Some((proxy_port, socket_inside, _, _, _, _)) = model {
            argv.push("--ro".to_owned());
            argv.push(MODEL_BRIDGE_INSIDE.to_owned());
            argv.push("--model-bridge".to_owned());
            argv.push(MODEL_BRIDGE_INSIDE.to_owned());
            argv.push("--model-socket".to_owned());
            argv.push(socket_inside.to_string_lossy().into_owned());
            argv.push("--model-port".to_owned());
            argv.push(proxy_port.to_string());
        }
        argv.push("--".to_owned());
        argv.extend(program.iter().cloned());

        Ok(argv)
    }
}

/// What a capsule is given in its environment.
///
/// By name, and short. Everything here is something a program cannot work without; anything else is
/// something the agent can be told through its own configuration, where it is visible.
fn environment(spec: &KernelCapsuleSpec) -> Vec<(String, String)> {
    let mut environment = vec![
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
    ];
    if let Network::Brokered { proxy_port, .. } = &spec.network {
        let proxy = format!("http://127.0.0.1:{proxy_port}");
        environment.push(("HTTPS_PROXY".to_owned(), proxy.clone()));
        environment.push(("HTTP_PROXY".to_owned(), proxy));
        environment.push(("NO_PROXY".to_owned(), "127.0.0.1,localhost".to_owned()));
    }
    environment
}

#[cfg(test)]
mod tests {
    /// A backend for the tests, told where the entry program is. There is no constructor without
    /// one, which is the point: a capsule missing Landlock and seccomp should not be expressible.
    fn entering() -> Bubblewrap {
        Bubblewrap::entering_through("/opt/cybou/libexec/cybou-capsule-enter")
            .with_egress_bridge("/opt/cybou/libexec/cybou-egress-bridge")
            .with_model_bridge("/opt/cybou/libexec/cybou-model-bridge")
    }

    fn bindings() -> CapsuleRuntimeBindings {
        CapsuleRuntimeBindings {
            egress_socket_host: Some(PathBuf::from("/run/user/1000/cybou/egress.sock")),
            model_socket_host: Some(PathBuf::from("/run/user/1000/cybou/model.sock")),
            model_token_host: Some(PathBuf::from("/run/user/1000/cybou/model-token")),
        }
    }

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
        entering()
            .command(
                &spec(),
                &bindings(),
                &["cargo".to_owned(), "test".to_owned()],
            )
            .expect("builds")
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
                [
                    "HOME",
                    "PWD",
                    "TMPDIR",
                    "PATH",
                    "CYBOU_CAPSULE",
                    "HTTPS_PROXY",
                    "HTTP_PROXY",
                    "NO_PROXY",
                ]
                .contains(&name.as_str()),
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
        assert_eq!(
            writable,
            vec![
                &"/srv/project".to_owned(),
                &"/run/user/1000/cybou/egress.sock".to_owned(),
                &"/run/user/1000/cybou/model.sock".to_owned()
            ]
        );
        assert!(argv.windows(3).any(|window| {
            window[0] == "--ro-bind"
                && window[1] == "/run/user/1000/cybou/model-token"
                && window[2] == "/run/cybou/model-token"
        }));
    }

    #[test]
    fn the_program_is_after_a_separator_whatever_it_is_called() {
        // A program named `--bind` is otherwise an argument to bwrap. The same lesson this
        // repository already learned about passing a systemd unit name.
        let hostile = vec!["--bind".to_owned(), "/etc".to_owned(), "/etc".to_owned()];
        let argv = entering()
            .command(&spec(), &bindings(), &hostile)
            .expect("builds");

        // The last separator, not the first. There are two now: bubblewrap's arguments end at the
        // first, the entry program's at the second, and the agent is what follows the second. A
        // version of this test that kept looking at the first would have read the entry program's
        // own flags as the agent's and passed on a command line that ran the wrong thing.
        let separator = argv.iter().rposition(|item| item == "--").expect("present");
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
            !entering().requires_seccomp(),
            "the entry program installs the filter, so nothing outside has to"
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
        let first = entering().command(&spec, &bindings(), &program);
        for _ in 0..8 {
            assert_eq!(entering().command(&spec, &bindings(), &program), first);
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
    fn bubblewrap_runs_the_entry_program_and_the_entry_program_runs_the_agent() {
        // Landlock and seccomp are restrictions a process applies to itself just before exec, so
        // there is a hop between the sandbox being built and the agent starting. If bubblewrap ran
        // the agent directly, the capsule would be missing two of its ten parts and the command
        // line would look entirely correct.
        let argv = argv();
        let first = argv.iter().position(|item| item == "--").expect("present");
        assert_eq!(argv[first + 1], ENTRY_INSIDE);
    }

    #[test]
    fn the_entry_program_is_told_every_path_the_spec_names() {
        // Landlock denies what it was not told about, so a rule dropped here is not a tighter
        // capsule — it is an agent that cannot open /dev/null.
        let spec = spec();
        let argv = entering()
            .command(&spec, &bindings(), &["sh".to_owned()])
            .expect("builds");
        for rule in &spec.landlock {
            let path = rule.path.to_string_lossy().into_owned();
            assert!(
                argv.contains(&path),
                "{path} is in the spec and not on the command line"
            );
        }
    }

    #[test]
    fn the_entry_program_is_read_only_inside_the_capsule() {
        // An agent that could rewrite it could arrange for the next capsule to be entered by
        // something of its own choosing.
        let argv = argv();
        let at = argv
            .iter()
            .position(|item| item == ENTRY_INSIDE)
            .expect("bound");
        assert_eq!(argv[at - 2], "--ro-bind");
    }

    #[test]
    fn a_backend_says_what_it_needs() {
        // So a deployment missing it reports that, rather than a capsule that failed for reasons
        // nobody can see.
        assert_eq!(
            entering().requires(),
            "bwrap, and this project's own cybou-capsule-enter"
        );
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
