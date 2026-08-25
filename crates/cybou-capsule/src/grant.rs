// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The profile a person grants once, and everything it bounds.

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::Duration;

pub use cybou_protocol::model::SpendPolicy;
use uuid::Uuid;

/// The one directory an agent may change.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    /// The directory itself, as an absolute path.
    pub root: PathBuf,
}

impl Workspace {
    /// A workspace at this root.
    #[must_use]
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Whether `path` names something inside this workspace.
    ///
    /// # Where sandboxes actually fail
    ///
    /// Not on the obvious `/etc/shadow`, which every implementation catches. On
    /// `/workspace/project/../../etc/shadow`, which is inside the workspace by string prefix and
    /// outside it by meaning. So the path is resolved lexically first — `..` is applied, `.` is
    /// dropped — and only then compared.
    ///
    /// Lexical, deliberately, and this is a limit worth stating rather than hiding: it does not
    /// follow symlinks, because resolving those means touching a filesystem, and a decision that
    /// touches a filesystem is a decision whose answer depends on when it was asked. A symlink out
    /// of the workspace is the kernel's to refuse — it is a mount and a Landlock rule, not a string
    /// comparison — and this module answering it would be this module pretending to be the
    /// enforcement it says it is not.
    #[must_use]
    pub fn contains(&self, path: &Path) -> bool {
        let Some(resolved) = lexically_resolve(path) else {
            // A relative path has no meaning here. It resolves against whichever directory a
            // process happened to be in, which is not a thing anybody granted.
            return false;
        };
        let Some(root) = lexically_resolve(&self.root) else {
            return false;
        };
        resolved.starts_with(&root)
    }
}

/// Apply `.` and `..` without consulting a filesystem.
///
/// `None` for a path that is not absolute, and for one that climbs above the root — `/../etc` is
/// not a path, and treating it as `/etc` would silently answer a question nobody asked.
fn lexically_resolve(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    let mut depth = 0usize;
    for component in path.components() {
        match component {
            Component::RootDir => out.push("/"),
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                out.pop();
            }
            Component::Normal(part) => {
                depth += 1;
                out.push(part);
            }
        }
    }
    out.is_absolute().then_some(out)
}

/// Where a capsule may connect.
///
/// An allow-list of hosts, and nothing that resembles a pattern. `*.example.com` reads as a
/// convenience and is a rule nobody can check at a glance: it admits
/// `anything-at-all.example.com`, which is exactly what an exfiltration path looks like.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkGrant {
    /// The hosts this capsule may reach, exactly as written.
    pub hosts: Vec<String>,
}

impl NetworkGrant {
    /// A grant permitting these hosts and no others.
    #[must_use]
    pub fn to(hosts: &[&str]) -> Self {
        Self {
            hosts: hosts.iter().map(|host| (*host).to_owned()).collect(),
        }
    }

    /// Whether this grant permits reaching `host`.
    ///
    /// Compared without regard to case, because DNS is not case-sensitive and a grant that admitted
    /// `github.com` while refusing `GitHub.com` would be a rule about spelling.
    #[must_use]
    pub fn permits(&self, host: &str) -> bool {
        self.hosts
            .iter()
            .any(|permitted| permitted.eq_ignore_ascii_case(host))
    }
}

/// What a capsule may consume.
///
/// Ceilings, not hints. A budget nothing enforces is a number in a dialogue box, and the reason to
/// carry it here is so the thing that does enforce it has one place to read — a cgroup, for the
/// three that are physical.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBudget {
    /// Memory, in mebibytes.
    pub memory_mib: u32,
    /// How many CPUs' worth of time.
    pub cpus: u32,
    /// The most processes and threads this capsule may hold at once.
    ///
    /// A fork bomb is not an attack scenario here; it is an ordinary mistake by an autonomous agent
    /// running a build. Memory and CPU ceilings do not stop one — a machine with a full process
    /// table is unusable long before it is out of memory — so this is its own limit, and cgroup has
    /// carried it all along.
    pub tasks_max: u32,
    /// How long this capsule may exist.
    pub lifetime: Duration,
}

/// What a capsule may ask of a model, if anything.
///
/// Separate from the resource budget, and optional, because those are two different kinds of
/// ending. Running out of money for completions is a reason to refuse a completion; it is not a
/// reason to freeze a capsule that was compiling something.
///
/// `None` is a first-class state and the common one on an unplugged host: a capsule may have no
/// business with a model at all, and one using a local model has nothing to spend. Folding this
/// into the budget made a zero ceiling indistinguishable from an exhausted one, so a capsule that
/// wanted no model was dead before it started — including the one
/// [`CapsuleGrant::nothing_but`] hands out as the starting point for building a profile.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelGrant {
    /// Which class of model this capsule may use.
    ///
    /// A class rather than a model name: a name pins the capsule to one provider's naming, breaks
    /// when that name is retired, and routes around whatever the class encodes.
    pub class: String,
    /// What may be spent, and whether anything may be at all.
    ///
    /// A policy rather than a number, because a ceiling of zero could not say which of two opposite
    /// things a person meant: *use something free* or *you have spent everything*. See
    /// [`SpendPolicy`].
    pub spend: SpendPolicy,
}

/// The profile a person grants once.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapsuleGrant {
    /// This capsule.
    pub capsule_id: Uuid,
    /// Which agent it runs.
    pub agent: String,
    /// The one directory it may change.
    pub workspace: Workspace,
    /// Where it may connect.
    pub network: NetworkGrant,
    /// What it may consume.
    pub budget: ResourceBudget,
    /// What it may ask of a model, if anything.
    pub model: Option<ModelGrant>,
    /// Which tools it may reach, by name.
    ///
    /// Mediated by the host rather than configured inside the agent. An agent that configures its
    /// own tool access has granted itself capabilities.
    pub tools: Vec<String>,
    /// Whether the agent may run arbitrary programs inside its own namespaces.
    ///
    /// True for a development profile, and it is not the alarming field it looks like: inside a
    /// capsule the agent already owns its processes, its filesystem view and its network view.
    /// Denying this makes an agent that cannot run a compiler, not an agent that is safer.
    pub may_execute: bool,
}

impl CapsuleGrant {
    /// A grant that permits nothing beyond an empty workspace.
    ///
    /// The starting point for building one, so that every capability a profile has is a line
    /// somebody wrote. A default that granted network or execution would make the interesting
    /// fields the ones nobody had to think about.
    #[must_use]
    pub fn nothing_but(workspace: Workspace, capsule_id: Uuid, agent: impl Into<String>) -> Self {
        Self {
            capsule_id,
            agent: agent.into(),
            workspace,
            network: NetworkGrant::default(),
            budget: ResourceBudget {
                memory_mib: 0,
                cpus: 0,
                tasks_max: 0,
                lifetime: Duration::ZERO,
            },
            model: None,
            tools: Vec::new(),
            may_execute: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace() -> Workspace {
        Workspace::at("/srv/project")
    }

    #[test]
    fn a_path_that_climbs_out_of_the_workspace_is_not_in_it() {
        // Where sandboxes actually fail. This is inside by string prefix and outside by meaning,
        // and an implementation comparing prefixes admits it.
        assert!(!workspace().contains(Path::new("/srv/project/../../etc/shadow")));
        assert!(!workspace().contains(Path::new("/srv/project/src/../../../etc/shadow")));
    }

    #[test]
    fn ordinary_paths_inside_and_outside_are_answered_the_obvious_way() {
        assert!(workspace().contains(Path::new("/srv/project/src/main.rs")));
        assert!(workspace().contains(Path::new("/srv/project")));
        assert!(!workspace().contains(Path::new("/etc/shadow")));
        assert!(!workspace().contains(Path::new("/srv/project-other/file")));
    }

    #[test]
    fn a_climb_that_comes_back_is_still_inside() {
        // The other half. Refusing this would make a build system that writes `src/../target` look
        // like an escape attempt, and an alarm that fires on ordinary work is an alarm nobody reads.
        assert!(workspace().contains(Path::new("/srv/project/src/../target/debug")));
        assert!(workspace().contains(Path::new("/srv/project/./src")));
    }

    #[test]
    fn a_relative_path_is_not_inside_anything() {
        // It resolves against whichever directory a process happened to be in, which is not a thing
        // anybody granted.
        assert!(!workspace().contains(Path::new("src/main.rs")));
        assert!(!workspace().contains(Path::new("../etc/shadow")));
    }

    #[test]
    fn a_path_that_climbs_above_the_root_is_refused_rather_than_clamped() {
        // `/../etc` is not a path. Treating it as `/etc` would silently answer a question nobody
        // asked, and would do it in the permissive direction.
        assert_eq!(lexically_resolve(Path::new("/../etc")), None);
        assert!(!Workspace::at("/").contains(Path::new("/../etc")));
    }

    #[test]
    fn a_network_grant_permits_what_it_names_and_nothing_adjacent() {
        let grant = NetworkGrant::to(&["github.com", "registry.npmjs.org"]);
        assert!(grant.permits("github.com"));
        assert!(grant.permits("GitHub.com"), "DNS is not case-sensitive");
        assert!(!grant.permits("evil.github.com.attacker.net"));
        assert!(!grant.permits("github.com.attacker.net"));
        assert!(
            !grant.permits("api.github.com"),
            "a subdomain is a different host"
        );
    }

    #[test]
    fn a_grant_built_from_nothing_grants_nothing() {
        // Every capability a profile has should be a line somebody wrote. A default with network or
        // execution in it would make the interesting fields the ones nobody had to think about.
        let grant = CapsuleGrant::nothing_but(workspace(), Uuid::from_u128(1), "opencode");
        assert!(grant.network.hosts.is_empty());
        assert!(grant.tools.is_empty());
        assert!(!grant.may_execute);
        assert_eq!(
            grant.model, None,
            "a starting point does not come with a model"
        );
        assert_eq!(grant.budget.tasks_max, 0);
    }
}
