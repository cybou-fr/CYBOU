// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Turning what a person granted into what the kernel is asked for.
//!
//! One function, total and deterministic: the same grant always compiles to the same spec, so a
//! capsule can be inspected before it exists and two runs can be compared.
//!
//! ## The filesystem is built up, never pruned down
//!
//! The mount list starts empty. Nothing here removes anything from a host root, because removing is
//! a deny-list and a deny-list is a list somebody forgets to extend — and the thing forgotten is
//! discovered by an agent, not by a reviewer.
//!
//! ## What this refuses
//!
//! A grant is written by a person, and a person can write something that would take the capsule
//! apart. Naming `/` as a workspace, or `/var/lib/cybou`, or the key store, would produce a perfectly
//! valid-looking spec that hands over everything the capsule exists to withhold. Those are refused
//! here, with a reason, rather than compiled into something that runs.
//!
//! The refusals are about *what a mount would expose*, not about what an agent might do with it.
//! This module has no opinion on the second and no way to form one.

use std::path::{Path, PathBuf};

use crate::grant::CapsuleGrant;
use crate::spec::{
    Access, CgroupLimits, KernelCapsuleSpec, ModelChannel, Mount, Namespaces, Network, PathRule,
    Seccomp,
};

/// Where the workspace appears inside a capsule.
///
/// Fixed rather than mirroring the host path. An agent that could see where its workspace really
/// lives has been told something about the machine, and a fixed name means a profile moved between
/// hosts produces the same environment.
pub const WORKSPACE_INSIDE: &str = "/workspace";

/// Fixed capsule-side endpoint for its one broker.
pub const EGRESS_SOCKET_INSIDE: &str = "/run/cybou/egress.sock";

/// Ordinary proxy clients are pointed at this capsule-local compatibility listener.
pub const EGRESS_PROXY_PORT: u16 = 3128;

/// Fixed capsule-side endpoint for the lease-bound model gateway.
pub const MODEL_SOCKET_INSIDE: &str = "/run/cybou/model.sock";

/// File containing only this capsule's ephemeral model lease token.
pub const MODEL_TOKEN_INSIDE: &str = "/run/cybou/model-token";

/// OpenAI-compatible endpoint used by agent packs inside the network namespace.
pub const MODEL_PROXY_PORT: u16 = 3130;

/// Paths a workspace may not be, or be inside, or contain.
///
/// Not a general filesystem policy — the mount list is built up, so nothing else is exposed anyway.
/// This exists because the workspace is the one path a person supplies, and it is therefore the one
/// place a grant can ask for something that undoes the rest.
const NEVER_A_WORKSPACE: [&str; 6] = [
    "/",
    "/etc",
    "/var/lib/cybou",
    "/usr/libexec/cybou",
    "/proc",
    "/sys",
];

/// Why a grant cannot be compiled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CannotCompile {
    /// The workspace is not an absolute path.
    ///
    /// A relative one resolves against whichever directory a process happened to be in, which is not
    /// a thing anybody granted.
    WorkspaceIsNotAbsolute(PathBuf),
    /// The workspace is, contains, or sits inside somewhere no capsule may have.
    WorkspaceWouldExposeTooMuch {
        /// What was asked for.
        workspace: PathBuf,
        /// What it collides with.
        forbidden: PathBuf,
    },
    /// A budget that permits nothing cannot run anything.
    ///
    /// Refused rather than compiled, because a capsule that is killed the instant it starts is
    /// indistinguishable from one that crashed, and an operator would go looking for the wrong
    /// thing.
    BudgetPermitsNothing(&'static str),
}

impl core::fmt::Display for CannotCompile {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::WorkspaceIsNotAbsolute(path) => {
                write!(
                    formatter,
                    "the workspace '{}' is not absolute",
                    path.display()
                )
            }
            Self::WorkspaceWouldExposeTooMuch {
                workspace,
                forbidden,
            } => write!(
                formatter,
                "a workspace at '{}' would expose '{}', which no capsule may have",
                workspace.display(),
                forbidden.display()
            ),
            Self::BudgetPermitsNothing(what) => {
                write!(
                    formatter,
                    "the budget permits no {what}, so nothing could run"
                )
            }
        }
    }
}

impl core::error::Error for CannotCompile {}

/// Compile a grant into the shape a backend builds.
///
/// # Errors
///
/// Returns [`CannotCompile`] for a workspace that is not absolute, one that would expose something
/// no capsule may have, or a budget under which nothing could run.
#[allow(
    clippy::too_many_lines,
    reason = "the complete deterministic grant-to-kernel translation stays reviewable in one place"
)]
pub fn compile(grant: &CapsuleGrant) -> Result<KernelCapsuleSpec, CannotCompile> {
    let workspace = &grant.workspace.root;
    if !workspace.is_absolute() {
        return Err(CannotCompile::WorkspaceIsNotAbsolute(workspace.clone()));
    }
    if let Some(forbidden) = collides_with_forbidden(workspace) {
        return Err(CannotCompile::WorkspaceWouldExposeTooMuch {
            workspace: workspace.clone(),
            forbidden,
        });
    }
    if grant.budget.memory_mib == 0 {
        return Err(CannotCompile::BudgetPermitsNothing("memory"));
    }
    if grant.budget.tasks_max == 0 {
        return Err(CannotCompile::BudgetPermitsNothing("processes"));
    }
    let infrastructure_tasks =
        u32::from(!grant.network.hosts.is_empty()) + u32::from(grant.model.is_some());
    if grant.budget.tasks_max <= infrastructure_tasks {
        return Err(CannotCompile::BudgetPermitsNothing(
            "processes after reserving capsule bridge tasks",
        ));
    }
    // A CPU quota of zero is not a small share, it is none: the cgroup would hold the capsule at a
    // standstill, which looks exactly like a capsule that hung.
    if grant.budget.cpus == 0 {
        return Err(CannotCompile::BudgetPermitsNothing("CPU"));
    }
    if grant.budget.lifetime <= time::Duration::ZERO {
        return Err(CannotCompile::BudgetPermitsNothing("time"));
    }

    let inside = PathBuf::from(WORKSPACE_INSIDE);

    // Read-only, and then the one writable path. Order is what a backend applies, and the writable
    // mount is last so nothing can be layered over it afterwards.
    let mut mounts = vec![
        // What a program needs to be a program. Read-only: an agent that could write these could
        // replace the compiler it is about to run.
        read_only("/usr"),
        read_only("/lib"),
        read_only("/lib64"),
        read_only("/bin"),
        read_only("/sbin"),
        // Resolver configuration and certificate authorities, so a granted egress works later
        // without the capsule being handed the whole of /etc.
        read_only("/etc/ssl"),
        read_only("/etc/resolv.conf"),
    ];
    mounts.push(Mount {
        source: workspace.clone(),
        target: inside.clone(),
        access: Access::ReadWrite,
    });

    // Landlock over the same shape. Belt and braces, in that order: the mount says a path is not
    // there, this says that even if it were, there are no rights to it.
    let mut landlock: Vec<PathRule> = mounts
        .iter()
        .map(|mount| PathRule {
            path: mount.target.clone(),
            access: mount.access,
        })
        .collect();
    // Three paths a backend makes rather than mounts, so a Landlock list built from the mounts alone
    // does not know they exist — and Landlock denies what it was not told about. Left out, the first
    // thing to break is `/dev/null`: every redirection in every script the agent runs fails with a
    // permission error, on a capsule whose mounts are perfectly correct.
    for made_by_the_backend in ["/tmp", "/dev", "/proc"] {
        landlock.push(PathRule {
            path: PathBuf::from(made_by_the_backend),
            access: Access::ReadWrite,
        });
    }
    if !grant.network.hosts.is_empty() {
        landlock.push(PathRule {
            path: PathBuf::from(EGRESS_SOCKET_INSIDE),
            access: Access::ReadWrite,
        });
    }
    if grant.model.is_some() {
        landlock.push(PathRule {
            path: PathBuf::from(MODEL_SOCKET_INSIDE),
            access: Access::ReadWrite,
        });
        landlock.push(PathRule {
            path: PathBuf::from(MODEL_TOKEN_INSIDE),
            access: Access::ReadOnly,
        });
    }

    Ok(KernelCapsuleSpec {
        capsule_id: grant.capsule_id,
        namespaces: Namespaces,
        mounts,
        landlock,
        seccomp: Seccomp::NoReshaping,
        cgroup: CgroupLimits {
            memory_mib: grant.budget.memory_mib,
            cpus: grant.budget.cpus,
            tasks_max: grant.budget.tasks_max,
            runtime_max_seconds: grant
                .budget
                .lifetime
                .whole_seconds()
                .try_into()
                .unwrap_or(u64::MAX),
        },
        network: if grant.network.hosts.is_empty() {
            Network::Denied
        } else {
            Network::Brokered {
                proxy_port: EGRESS_PROXY_PORT,
                socket_inside: PathBuf::from(EGRESS_SOCKET_INSIDE),
            }
        },
        model: if grant.model.is_some() {
            ModelChannel::Brokered {
                proxy_port: MODEL_PROXY_PORT,
                socket_inside: PathBuf::from(MODEL_SOCKET_INSIDE),
                token_inside: PathBuf::from(MODEL_TOKEN_INSIDE),
            }
        } else {
            ModelChannel::Denied
        },
        working_directory: inside,
    })
}

/// A read-only mount that appears at the same path it comes from.
fn read_only(path: &str) -> Mount {
    Mount {
        source: PathBuf::from(path),
        target: PathBuf::from(path),
        access: Access::ReadOnly,
    }
}

/// Whichever forbidden path this workspace collides with, if any.
///
/// Both directions, and the root needs care. A workspace *inside* `/etc` exposes part of it; a
/// workspace *containing* `/etc` exposes all of it, and checking only the first direction catches
/// the obvious request and misses a workspace at `/var/lib`.
///
/// The root is the exception, and the first version of this got it wrong in a way worth keeping a
/// note about: every absolute path starts with `/`, so the "inside" test made every workspace
/// collide with the root and nothing compiled at all. Wrong in the refusing direction, which is the
/// safe way to be wrong and still wrong. `/` is forbidden as a workspace exactly, and by containing
/// everything else — never by containing the workspace, which it always does.
fn collides_with_forbidden(workspace: &Path) -> Option<PathBuf> {
    let root = Path::new("/");
    NEVER_A_WORKSPACE
        .iter()
        .map(PathBuf::from)
        .find(|forbidden| {
            let inside_it = forbidden != root && workspace.starts_with(forbidden);
            let contains_it = forbidden.starts_with(workspace);
            inside_it || contains_it
        })
}

#[cfg(test)]
mod tests {
    use time::Duration;
    use uuid::Uuid;

    use super::*;
    use crate::grant::{ModelGrant, NetworkGrant, ResourceBudget, Workspace};
    use crate::spec::Access;

    fn grant_at(root: &str) -> CapsuleGrant {
        CapsuleGrant {
            capsule_id: Uuid::from_u128(8472),
            agent: "opencode".to_owned(),
            workspace: Workspace::at(root),
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
        }
    }

    #[test]
    fn an_ordinary_grant_compiles_to_one_writable_path_and_no_others() {
        // The property a gate will assert on a live capsule, checked here where it is cheap.
        let spec = compile(&grant_at("/srv/project")).expect("compiles");
        assert_eq!(
            spec.writable_outside_the_workspace(),
            Vec::<&PathBuf>::new(),
            "something other than the workspace is writable"
        );

        let writable: Vec<&PathBuf> = spec
            .mounts
            .iter()
            .filter(|mount| mount.access == Access::ReadWrite)
            .map(|mount| &mount.source)
            .collect();
        assert_eq!(writable, vec![&PathBuf::from("/srv/project")]);
    }

    #[test]
    fn the_workspace_appears_at_a_fixed_place_and_not_at_its_host_path() {
        // An agent that could see where its workspace really lives has been told something about the
        // machine, and a fixed name means a profile moved between hosts produces the same
        // environment.
        let spec = compile(&grant_at("/srv/project")).expect("compiles");
        assert_eq!(spec.working_directory, PathBuf::from(WORKSPACE_INSIDE));
        assert!(
            spec.mounts
                .iter()
                .any(|mount| mount.target.as_path() == Path::new(WORKSPACE_INSIDE))
        );
        assert!(
            !spec
                .mounts
                .iter()
                .any(|mount| mount.target.as_path() == Path::new("/srv/project")),
            "the host path leaked into the capsule's view"
        );
    }

    #[test]
    fn a_workspace_at_the_root_is_refused_rather_than_compiled() {
        // It would produce a perfectly valid-looking spec that hands over everything the capsule
        // exists to withhold.
        let refusal = compile(&grant_at("/")).expect_err("must not compile");
        assert!(
            matches!(refusal, CannotCompile::WorkspaceWouldExposeTooMuch { .. }),
            "{refusal:?}"
        );
    }

    #[test]
    fn a_workspace_that_contains_something_forbidden_is_refused_too() {
        // Both directions. A workspace inside /etc exposes part of it; one containing /etc exposes
        // all of it, and checking only the first catches the obvious request and misses /var.
        for root in ["/etc", "/etc/ssh", "/var/lib/cybou", "/var/lib", "/proc"] {
            let refusal = compile(&grant_at(root)).expect_err(root);
            assert!(
                matches!(refusal, CannotCompile::WorkspaceWouldExposeTooMuch { .. }),
                "{root} compiled: {refusal:?}"
            );
        }
    }

    #[test]
    fn the_journal_and_its_keys_are_never_a_workspace() {
        // The whole erasure guarantee is that a destroyed key makes a record unreadable. An agent
        // holding either is that guarantee ending at the least trusted process on the machine.
        assert!(compile(&grant_at("/var/lib/cybou")).is_err());
        assert!(compile(&grant_at("/var/lib/cybou/keys")).is_err());
    }

    #[test]
    fn a_relative_workspace_is_refused() {
        // It resolves against whichever directory a process happened to be in, which is not a thing
        // anybody granted.
        let refusal = compile(&grant_at("project")).expect_err("must not compile");
        assert!(
            matches!(refusal, CannotCompile::WorkspaceIsNotAbsolute(_)),
            "{refusal:?}"
        );
    }

    #[test]
    fn a_budget_under_which_nothing_could_run_is_refused_rather_than_compiled() {
        // A capsule killed the instant it starts is indistinguishable from one that crashed, and an
        // operator would go looking for the wrong thing.
        for (adjust, expected) in [
            (
                Box::new(|g: &mut CapsuleGrant| g.budget.memory_mib = 0) as Box<dyn Fn(&mut _)>,
                "memory",
            ),
            (
                Box::new(|g: &mut CapsuleGrant| g.budget.tasks_max = 0),
                "processes",
            ),
            (
                Box::new(|g: &mut CapsuleGrant| g.budget.lifetime = Duration::ZERO),
                "time",
            ),
        ] {
            let mut grant = grant_at("/srv/project");
            adjust(&mut grant);
            assert_eq!(
                compile(&grant),
                Err(CannotCompile::BudgetPermitsNothing(expected)),
                "a budget with no {expected} compiled"
            );
        }
    }

    #[test]
    fn a_network_grant_compiles_to_one_channel_and_not_a_second_policy() {
        let mut generous = grant_at("/srv/project");
        generous.network = NetworkGrant::to(&["github.com", "example.com", "anything.at.all"]);
        let spec = compile(&generous).expect("compiles");
        assert_eq!(
            spec.network,
            Network::Brokered {
                proxy_port: EGRESS_PROXY_PORT,
                socket_inside: PathBuf::from(EGRESS_SOCKET_INSIDE),
            }
        );
        assert!(!format!("{:?}", spec.network).contains("github.com"));
    }

    #[test]
    fn the_bridge_is_counted_before_a_brokered_capsule_is_built() {
        let mut grant = grant_at("/srv/project");
        grant.budget.tasks_max = 1;
        assert_eq!(
            compile(&grant),
            Err(CannotCompile::BudgetPermitsNothing(
                "processes after reserving capsule bridge tasks"
            ))
        );

        grant.network = NetworkGrant::default();
        grant.model = None;
        assert!(compile(&grant).is_ok(), "one task can hold one agent");
    }

    #[test]
    fn a_model_grant_compiles_to_transport_and_ephemeral_authority_paths() {
        let spec = compile(&grant_at("/srv/project")).expect("compiles");
        assert_eq!(
            spec.model,
            ModelChannel::Brokered {
                proxy_port: MODEL_PROXY_PORT,
                socket_inside: PathBuf::from(MODEL_SOCKET_INSIDE),
                token_inside: PathBuf::from(MODEL_TOKEN_INSIDE),
            }
        );
        assert!(!format!("{spec:?}").contains("provider-key"));
    }

    #[test]
    fn compiling_is_deterministic() {
        // A capsule can be inspected before it exists, and two runs can be compared. A spec that
        // varied between compilations would make both useless.
        let grant = grant_at("/srv/project");
        let first = compile(&grant).expect("compiles");
        for _ in 0..8 {
            assert_eq!(compile(&grant).expect("compiles"), first);
        }
    }

    #[test]
    fn nothing_a_program_needs_is_writable() {
        // An agent that could write these could replace the compiler it is about to run, which is
        // the least noisy way to end up executing something nobody chose.
        let spec = compile(&grant_at("/srv/project")).expect("compiles");
        for mount in &spec.mounts {
            if mount.target.as_path() == Path::new(WORKSPACE_INSIDE) {
                continue;
            }
            assert_eq!(
                mount.access,
                Access::ReadOnly,
                "{} is writable",
                mount.target.display()
            );
        }
    }
}
