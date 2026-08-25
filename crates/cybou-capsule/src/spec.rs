// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The compiled shape of a capsule: what the kernel is asked for, and nothing else.
//!
//! A [`crate::grant::CapsuleGrant`] is what a person decided. This is what that becomes before any
//! backend touches a syscall — a value, inspectable and comparable, that a test can assert against
//! without a Linux kernel in the room.
//!
//! ## Compiled once, not consulted per call
//!
//! ```text
//! CapsuleGrant  ->  KernelCapsuleSpec  ->  namespaces · mounts · Landlock · cgroup · seccomp
//! ```
//!
//! The tempting alternative has a good-sounding shape: route every `open` through a policy decision
//! in Rust. That rebuilds the runtime permission mediator this whole design replaces, puts the
//! boundary back inside a conversation, and makes the enforcement only as available as the process
//! answering. [`crate::reach::Reach`] stays what it was written for — explaining, telemetry, audit,
//! and naming a boundary crossing — and is never asked about a syscall.
//!
//! ## What this type cannot say
//!
//! As much as possible is made unrepresentable rather than merely unused, because an unused
//! possibility is a possibility somebody uses later for a good reason.
//!
//! - There is no way to express *do not unshare this namespace*. [`Namespaces`] has no fields; a
//!   capsule gets all of them or is not a capsule.
//! - There is no way to express *and no new privileges is off*. It is not a field.
//! - [`Network`] says only whether a brokered channel exists. Host names remain in the grant and
//!   are decided by the broker; duplicating them here would create two owners of network policy.
//! - A mount is read-only unless something asked for otherwise, and the compiler is the only thing
//!   that builds the list.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The namespaces a capsule always gets.
///
/// A struct with no fields, on purpose. There is no configuration here and there should not be: a
/// capsule missing its PID namespace can see and signal host processes, and one missing its mount
/// namespace has no filesystem of its own. Something that could be turned off would eventually be
/// turned off by somebody debugging.
///
/// It exists as a named type rather than being implied so that the backend's argument list says what
/// it is asking for, and so this document has somewhere to live.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Namespaces;

impl Namespaces {
    /// The ones every capsule is built with, for a backend to translate.
    ///
    /// `user` is what makes the rest available without privilege. `cgroup` keeps the capsule from
    /// reading the host's hierarchy, which is a map of everything else running.
    pub const ALL: [&'static str; 7] = ["user", "mount", "pid", "ipc", "uts", "net", "cgroup"];
}

/// How a path is exposed inside a capsule.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Access {
    /// Visible, and not writable.
    ReadOnly,
    /// Visible and writable.
    ///
    /// One of these per capsule in this version: the workspace. A second would need a reason, and
    /// the reason would need to survive the question *what does the agent do with it that the
    /// workspace cannot*.
    ReadWrite,
}

/// One path made visible inside a capsule.
///
/// The list starts empty and is built up. Not a host root with things removed — that is a deny-list,
/// and a deny-list is a list somebody forgets to extend. Every entry here is something a person's
/// grant asked for, or something an agent cannot run without.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mount {
    /// Where it comes from on the host.
    pub source: PathBuf,
    /// Where it appears inside.
    pub target: PathBuf,
    /// Whether the capsule may write it.
    pub access: Access,
}

/// A Landlock rule, the second barrier.
///
/// The mount namespace says a path is not there. This says that even if it somehow were, there are
/// no rights to it. Defence in depth and not a replacement: a design relying on Landlock alone would
/// be one where a mount mistake is invisible.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathRule {
    /// The path the rule is about, as seen inside the capsule.
    pub path: PathBuf,
    /// What is permitted on it.
    pub access: Access,
}

/// What a capsule may reach on the network.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Network {
    /// A fresh network namespace with loopback and no route.
    ///
    /// The only thing this version can express, and the first acceptance gate. What a grant permits
    /// arrives later through an egress broker rather than a firewall allow-list: a grant names
    /// `github.com` and a firewall works in addresses, and turning one into the other means owning a
    /// policy engine for DNS lifetime and rebinding — where being wrong is silent.
    Denied,
    /// A fresh network namespace with one capsule-local compatibility listener.
    ///
    /// This is plumbing, not policy. The pathname reaches exactly one broker and the port lets
    /// ordinary clients use `HTTPS_PROXY`; neither field says which hosts may be reached.
    Brokered {
        /// TCP port on capsule loopback.
        proxy_port: u16,
        /// Pathname Unix socket as seen inside the capsule.
        socket_inside: PathBuf,
    },
}

/// The one model endpoint made visible inside a capsule.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelChannel {
    /// No model authority and no model transport.
    #[default]
    Denied,
    /// A loopback compatibility listener backed by one host pathname socket.
    ///
    /// The token pathname contains only the ephemeral lease authority. Provider credentials stay
    /// outside the capsule and outside this type.
    Brokered {
        /// TCP port on capsule loopback.
        proxy_port: u16,
        /// Pathname Unix socket as seen inside the capsule.
        socket_inside: PathBuf,
        /// Read-only file containing the ephemeral lease token.
        token_inside: PathBuf,
    },
}

/// What the capsule may consume, as the kernel counts it.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CgroupLimits {
    /// Memory ceiling, in mebibytes.
    pub memory_mib: u32,
    /// CPU quota, in whole CPUs.
    pub cpus: u32,
    /// The most processes and threads at once.
    ///
    /// Neither of the other two stops a fork bomb: a machine with a full process table is unusable
    /// long before it is out of memory, and a fork bomb uses very little of either.
    pub tasks_max: u32,
    /// How long the capsule may run, in seconds.
    ///
    /// Belongs to the unit rather than to a timer inside Mind. A lifetime enforced by something that
    /// has to still be running is a lifetime that ends when that thing does not.
    pub runtime_max_seconds: u64,
}

/// What a capsule may not do to its own shape.
///
/// Not an allow-list of syscalls. A development agent runs compilers, linkers, package managers and
/// whatever a build script felt like, and an allow-list against that is either enormous or breaks on
/// a Tuesday. What is denied is the small set that would let the sandbox rearrange itself.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Seccomp {
    /// Refuse the calls that change the sandbox rather than work inside it.
    NoReshaping,
}

impl Seccomp {
    /// The families a backend must refuse, named rather than numbered.
    ///
    /// Numbers differ by architecture, and a list of numbers is a list nobody can review. The
    /// backend maps these to whatever its platform calls them.
    pub const DENIED: [&'static str; 6] = [
        // A capsule that can make a namespace can make one it is root in, which is the escape hatch
        // ADR-0042 refuses by name.
        "unshare",
        "clone-newuser",
        "setns",
        // Rearranging the filesystem the mounts were carefully built to shape.
        "mount-family",
        // Host control that has no business inside a capsule at all.
        "kernel-module",
        "reboot-kexec",
    ];
}

/// Everything a backend needs to build one capsule, and nothing it does not.
///
/// Serializable so it can be recorded beside what the capsule then did. A record of the environment
/// an agent ran in is the other half of a record of what it tried.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelCapsuleSpec {
    /// Which capsule.
    pub capsule_id: uuid::Uuid,
    /// The namespaces to unshare. All of them.
    pub namespaces: Namespaces,
    /// The filesystem, built up from nothing, in the order a backend should apply it.
    pub mounts: Vec<Mount>,
    /// The second barrier over the same paths.
    pub landlock: Vec<PathRule>,
    /// What the capsule may not do to its own shape.
    pub seccomp: Seccomp,
    /// What it may consume.
    pub cgroup: CgroupLimits,
    /// What it may reach.
    pub network: Network,
    /// How it reaches the lease-bound model gateway, if a model was granted.
    #[serde(default)]
    pub model: ModelChannel,
    /// Where the agent starts, inside the capsule.
    pub working_directory: PathBuf,
}

impl KernelCapsuleSpec {
    /// Whether anything in this spec is writable other than the working directory and `/tmp`.
    ///
    /// A question a gate asks rather than a thing the type prevents, because the answer is about the
    /// combination and not about any one mount. A spec that ever answers true has grown a second
    /// writable path, and that should be somebody's decision rather than a diff nobody read.
    #[must_use]
    pub fn writable_outside_the_workspace(&self) -> Vec<&PathBuf> {
        self.mounts
            .iter()
            .filter(|mount| mount.access == Access::ReadWrite)
            .map(|mount| &mount.target)
            .filter(|target| *target != &self.working_directory && !target.starts_with("/tmp"))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_namespace_a_capsule_needs_is_named_and_none_is_optional() {
        // A capsule missing its PID namespace can see and signal host processes; one missing its
        // mount namespace has no filesystem of its own. Something that could be turned off would
        // eventually be turned off by somebody debugging.
        for needed in ["user", "mount", "pid", "ipc", "uts", "net", "cgroup"] {
            assert!(
                Namespaces::ALL.contains(&needed),
                "{needed} is not in the set every capsule gets"
            );
        }
        assert_eq!(Namespaces::ALL.len(), 7);
    }

    #[test]
    fn the_denied_syscall_families_are_about_the_sandbox_and_not_about_work() {
        // An allow-list against a development agent is either enormous or breaks on a Tuesday. What
        // is denied is the small set that would let the sandbox rearrange itself.
        assert!(Seccomp::DENIED.contains(&"setns"));
        assert!(Seccomp::DENIED.contains(&"mount-family"));
        assert!(Seccomp::DENIED.contains(&"clone-newuser"));
        assert!(
            !Seccomp::DENIED.iter().any(|name| name.contains("open")),
            "a filesystem call reached the seccomp list, which is the mount namespace's job"
        );
    }

    #[test]
    fn brokered_network_names_plumbing_and_not_policy() {
        let network = Network::Brokered {
            proxy_port: 3128,
            socket_inside: PathBuf::from("/run/cybou/egress.sock"),
        };
        let written = format!("{network:?}");
        assert!(!written.contains("github.com"));
        assert!(written.contains("3128"));
    }

    #[test]
    fn a_spec_survives_the_wire() {
        // It is recorded beside what the capsule then did: a record of the environment an agent ran
        // in is the other half of a record of what it tried.
        let spec = KernelCapsuleSpec {
            capsule_id: uuid::Uuid::from_u128(8472),
            namespaces: Namespaces,
            mounts: vec![Mount {
                source: PathBuf::from("/srv/project"),
                target: PathBuf::from("/workspace"),
                access: Access::ReadWrite,
            }],
            landlock: vec![PathRule {
                path: PathBuf::from("/workspace"),
                access: Access::ReadWrite,
            }],
            seccomp: Seccomp::NoReshaping,
            cgroup: CgroupLimits {
                memory_mib: 4096,
                cpus: 2,
                tasks_max: 512,
                runtime_max_seconds: 14_400,
            },
            network: Network::Denied,
            model: ModelChannel::Denied,
            working_directory: PathBuf::from("/workspace"),
        };

        let mut encoded = Vec::new();
        ciborium::into_writer(&spec, &mut encoded).expect("encodes");
        let decoded: KernelCapsuleSpec =
            ciborium::from_reader(encoded.as_slice()).expect("decodes");
        assert_eq!(decoded, spec);
    }
}
