// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Refusing the calls that change the sandbox rather than work inside it.
//!
//! Not an allow-list. A development agent runs compilers, linkers, package managers and whatever a
//! build script felt like that morning, and an allow-list against that is either enormous or breaks
//! on a Tuesday — and a sandbox that breaks a legitimate build gets switched off, which is the least
//! secure outcome available. What is denied is the small set that would let a capsule rearrange
//! itself: `cybou_capsule::spec::Seccomp::DENIED`, named there and numbered here.
//!
//! ## Killed, not refused
//!
//! A matched call kills the process rather than returning `EPERM`. Two reasons, and the second is
//! the one that decided it.
//!
//! A program that receives `EPERM` from `mount` carries on and does something else, which is exactly
//! right for a program that tried a nice-to-have — and exactly wrong for one trying to escape. There
//! is no legitimate call to `unshare` from inside a capsule, so there is nothing to be graceful
//! about.
//!
//! And `EPERM` is what these calls already return to an unprivileged process in a user namespace.
//! Had the filter returned it too, no test could tell a build with the filter from a build without
//! one: the gate would have passed identically on a capsule where this file was never called, which
//! is the shape of failure this repository keeps finding. A killed process is a fact a gate can see.
//!
//! ## `clone3` is the exception, and it has to be
//!
//! Seccomp can read a syscall's arguments but never memory they point at, and `clone3` takes a
//! pointer to a struct — so its flags are invisible to any filter. Killing it outright would be
//! honest and would also break everything: modern glibc creates threads and processes with it.
//!
//! It returns `ENOSYS` instead, which is not a fudge but the documented handshake — glibc probes
//! `clone3`, sees "this kernel does not have it", and falls back to `clone`, whose flags *are* an
//! argument and are checked. The capsule ends up with every fork inspected, at the price of one
//! wasted syscall per process.
//!
//! ## Two filters, in order
//!
//! One filter has one action for everything it matches, and this needs two. Seccomp evaluates every
//! installed filter and returns the most severe answer, so a kill filter and an `ENOSYS` filter
//! applied in turn give each call the answer meant for it.

use std::collections::BTreeMap;

use seccompiler::{
    SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter, SeccompRule,
    TargetArch,
};

/// The architecture this was built for.
///
/// Syscall numbers differ between architectures, which is why the spec names families and this file
/// numbers them. A filter built for the wrong architecture denies whatever those numbers happen to
/// mean elsewhere, which could be anything.
#[cfg(target_arch = "x86_64")]
const ARCHITECTURE: TargetArch = TargetArch::x86_64;
#[cfg(target_arch = "aarch64")]
const ARCHITECTURE: TargetArch = TargetArch::aarch64;

/// Install the filters on this process, so what it becomes inherits them.
pub fn refuse_reshaping() -> Result<(), String> {
    apply(killed(), SeccompAction::KillProcess)?;
    apply(
        pretended_missing(),
        SeccompAction::Errno(libc::ENOSYS as u32),
    )?;
    Ok(())
}

/// Build one filter and put it in place.
fn apply(rules: BTreeMap<i64, Vec<SeccompRule>>, on_match: SeccompAction) -> Result<(), String> {
    // Everything not named is allowed. The alternative is an allow-list, refused at the top of this
    // file for reasons that are about whether the sandbox survives contact with real work.
    let filter = SeccompFilter::new(rules, SeccompAction::Allow, on_match, ARCHITECTURE)
        .map_err(|why| format!("could not build a seccomp filter: {why}"))?;
    let program: seccompiler::BpfProgram = filter
        .try_into()
        .map_err(|why| format!("could not compile a seccomp filter: {why}"))?;
    seccompiler::apply_filter(&program)
        .map_err(|why| format!("could not install a seccomp filter: {why}"))
}

/// The calls that end the process that made them.
fn killed() -> BTreeMap<i64, Vec<SeccompRule>> {
    let mut rules: BTreeMap<i64, Vec<SeccompRule>> = BTreeMap::new();

    // An empty rule list means the call is matched whatever its arguments are.
    for always in [
        // A capsule that can make a namespace can make one it is root in.
        libc::SYS_unshare,
        // Or join somebody else's, which is the same escape by a different door.
        libc::SYS_setns,
        // Rearranging the filesystem the mounts were built to shape.
        libc::SYS_mount,
        libc::SYS_umount2,
        libc::SYS_pivot_root,
        libc::SYS_move_mount,
        libc::SYS_open_tree,
        libc::SYS_fsopen,
        libc::SYS_fsconfig,
        libc::SYS_fsmount,
        libc::SYS_mount_setattr,
        // Host control that has no business inside a capsule at all.
        libc::SYS_init_module,
        libc::SYS_finit_module,
        libc::SYS_delete_module,
        libc::SYS_reboot,
        libc::SYS_kexec_load,
        libc::SYS_kexec_file_load,
    ] {
        rules.insert(always, Vec::new());
    }

    // `clone` is how every process on the machine is made, so it is not denied — only the one flag
    // that would make a new user namespace. The flags are the first argument, which seccomp can read
    // because it is a number rather than something a pointer leads to.
    if let Ok(condition) = SeccompCondition::new(
        0,
        SeccompCmpArgLen::Qword,
        SeccompCmpOp::MaskedEq(libc::CLONE_NEWUSER as u64),
        libc::CLONE_NEWUSER as u64,
    ) && let Ok(rule) = SeccompRule::new(vec![condition])
    {
        rules.insert(libc::SYS_clone, vec![rule]);
    }

    rules
}

/// The calls answered with "this kernel does not have it".
fn pretended_missing() -> BTreeMap<i64, Vec<SeccompRule>> {
    let mut rules: BTreeMap<i64, Vec<SeccompRule>> = BTreeMap::new();
    // See the note above: its flags live behind a pointer, so no filter can read them, and glibc
    // falls back to `clone` when told it is missing.
    rules.insert(libc::SYS_clone3, Vec::new());
    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_family_the_spec_names_is_numbered_here() {
        // The spec names families because numbers differ by architecture and a list of numbers is a
        // list nobody can review. This is where they become numbers, and a family that arrived in
        // the spec without arriving here would be a denial that exists only in prose.
        let killed = killed();
        assert!(killed.contains_key(&libc::SYS_unshare), "unshare");
        assert!(killed.contains_key(&libc::SYS_setns), "setns");
        assert!(killed.contains_key(&libc::SYS_mount), "mount-family");
        assert!(killed.contains_key(&libc::SYS_pivot_root), "mount-family");
        assert!(killed.contains_key(&libc::SYS_init_module), "kernel-module");
        assert!(killed.contains_key(&libc::SYS_reboot), "reboot-kexec");
        assert!(killed.contains_key(&libc::SYS_kexec_load), "reboot-kexec");
        assert!(killed.contains_key(&libc::SYS_clone), "clone-newuser");
    }

    #[test]
    fn making_a_process_is_not_denied_and_making_a_user_namespace_is() {
        // A filter that denied `clone` outright would deny every program that starts another one,
        // which is every shell, compiler and package manager an agent runs.
        let killed = killed();
        let conditions = killed.get(&libc::SYS_clone).expect("clone is listed");
        assert_eq!(
            conditions.len(),
            1,
            "clone must be denied conditionally, never outright"
        );
    }

    #[test]
    fn nothing_a_program_needs_to_do_ordinary_work_is_on_the_list() {
        // The failure this guards against is a sandbox tight enough to break a legitimate build,
        // because a sandbox that breaks builds gets switched off.
        let killed = killed();
        for ordinary in [
            libc::SYS_openat,
            libc::SYS_read,
            libc::SYS_write,
            libc::SYS_execve,
            libc::SYS_mmap,
            libc::SYS_socket,
        ] {
            assert!(
                !killed.contains_key(&ordinary),
                "{ordinary} is ordinary work"
            );
        }
    }

    #[test]
    fn clone3_is_answered_rather_than_killed() {
        // Its flags are behind a pointer, so no filter can read them. Killing it would break modern
        // glibc; ENOSYS makes glibc fall back to `clone`, whose flags are checked.
        assert!(pretended_missing().contains_key(&libc::SYS_clone3));
        assert!(!killed().contains_key(&libc::SYS_clone3));
    }
}
