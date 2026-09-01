// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Owner-side cgroup v2 readings for one capsule unit.

use std::{
    fs::File,
    io::{Read, Result as IoResult},
    path::{Path, PathBuf},
    process::Command,
};

const CGROUP_ROOT: &str = "/sys/fs/cgroup";
const MAX_READING_BYTES: u64 = 1024 * 1024;

/// Runtime values established directly from one cgroup v2 directory.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CgroupReadings {
    /// Number of process identifiers listed in `cgroup.procs`.
    pub process_count: Option<u32>,
    /// Bytes currently charged to the cgroup.
    pub memory_current_bytes: Option<u64>,
    /// Kernel-enforced memory ceiling in bytes, absent when unbounded or unreadable.
    pub memory_max_bytes: Option<u64>,
    /// Cumulative CPU time charged to the cgroup.
    pub cpu_usage_usec: Option<u64>,
    /// Current number of tasks in the cgroup hierarchy.
    pub pids_current: Option<u32>,
    /// Kernel-enforced task ceiling, absent when unbounded or unreadable.
    pub pids_max: Option<u32>,
}

/// Kernel cgroup state requested through the owning user service manager.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FreezeState {
    /// Processes must be frozen.
    Frozen,
    /// Processes must be runnable.
    Running,
}

/// Ask the user's systemd owner where it placed the unit, then read its cgroup files.
#[must_use]
pub fn read_unit(unit: &str) -> CgroupReadings {
    control_group(unit).map_or_else(CgroupReadings::default, |control_group| {
        read_at(Path::new(CGROUP_ROOT), &control_group)
    })
}

/// Ask systemd to freeze or thaw a unit, then independently verify `cgroup.freeze`.
#[must_use]
pub fn set_freeze_state(unit: &str, state: FreezeState) -> bool {
    let verb = match state {
        FreezeState::Frozen => "freeze",
        FreezeState::Running => "thaw",
    };
    let Ok(status) = Command::new("systemctl")
        .args(["--user", verb, unit])
        .status()
    else {
        return false;
    };
    if !status.success() {
        return false;
    }
    freeze_state_is(unit, state)
}

/// Read the kernel state again without issuing another control request.
///
/// This is deliberately separate from [`set_freeze_state`]: Agent1 uses it immediately before
/// publishing a transition, while holding the capsule's control gate, so its projection describes
/// the state after every earlier request for that capsule.
#[must_use]
pub fn freeze_state_is(unit: &str, state: FreezeState) -> bool {
    let Some(control_group) = control_group(unit) else {
        return false;
    };
    let expected = match state {
        FreezeState::Frozen => "1",
        FreezeState::Running => "0",
    };
    let Some(relative) = safe_relative(&control_group) else {
        return false;
    };
    read_text(Path::new(CGROUP_ROOT).join(relative).join("cgroup.freeze"))
        .is_ok_and(|value| value.trim() == expected)
}

/// Stop a session's egress broker and verify that systemd reports it inactive or failed.
#[must_use]
pub fn revoke_egress(unit: &str) -> bool {
    let _ = Command::new("systemctl")
        .args(["--user", "stop", unit])
        .status();
    unit_inactive(unit, true)
}

/// Stop the system-owned model gateway, verify it is inactive, and verify systemd removed every
/// bearer surface from its runtime directory.
#[must_use]
pub fn revoke_model(unit: &str, artifacts: &[PathBuf]) -> bool {
    let _ = Command::new("systemctl").args(["stop", unit]).status();
    artifacts.len() == 2
        && unit_inactive(unit, false)
        && artifacts.iter().all(|artifact| !artifact.exists())
}

fn unit_inactive(unit: &str, user: bool) -> bool {
    let mut command = Command::new("systemctl");
    if user {
        command.arg("--user");
    }
    let Ok(output) = command
        .args(["show", "--property=ActiveState", "--value", unit])
        .output()
    else {
        return false;
    };
    if !output.status.success() || output.stdout.len() > 128 {
        return false;
    }
    std::str::from_utf8(&output.stdout)
        .is_ok_and(|state| matches!(state.trim(), "inactive" | "failed"))
}

fn control_group(unit: &str) -> Option<String> {
    let Ok(output) = Command::new("systemctl")
        .args(["--user", "show", "--property=ControlGroup", "--value", unit])
        .output()
    else {
        return None;
    };
    if !output.status.success() || output.stdout.len() > 4096 {
        return None;
    }
    let Ok(control_group) = std::str::from_utf8(&output.stdout) else {
        return None;
    };
    let control_group = control_group.trim();
    (!control_group.is_empty()).then(|| control_group.to_owned())
}

#[must_use]
fn read_at(root: &Path, control_group: &str) -> CgroupReadings {
    let Some(relative) = safe_relative(control_group) else {
        return CgroupReadings::default();
    };
    let directory = root.join(relative);
    CgroupReadings {
        process_count: read_text(directory.join("cgroup.procs"))
            .ok()
            .and_then(|text| {
                u32::try_from(text.lines().filter(|line| !line.is_empty()).count()).ok()
            }),
        memory_current_bytes: read_number(directory.join("memory.current")),
        memory_max_bytes: read_limit(directory.join("memory.max")),
        cpu_usage_usec: read_text(directory.join("cpu.stat"))
            .ok()
            .and_then(|text| keyed_number(&text, "usage_usec")),
        pids_current: read_number(directory.join("pids.current"))
            .and_then(|value| u32::try_from(value).ok()),
        pids_max: read_limit(directory.join("pids.max"))
            .and_then(|value| u32::try_from(value).ok()),
    }
}

fn safe_relative(control_group: &str) -> Option<&str> {
    let relative = control_group.strip_prefix('/')?;
    (!relative.split('/').any(|part| part == "..")).then_some(relative)
}

fn read_text(path: PathBuf) -> IoResult<String> {
    let mut text = String::new();
    File::open(path)?
        .take(MAX_READING_BYTES)
        .read_to_string(&mut text)?;
    Ok(text)
}

fn read_number(path: PathBuf) -> Option<u64> {
    read_text(path).ok()?.trim().parse().ok()
}

fn read_limit(path: PathBuf) -> Option<u64> {
    let text = read_text(path).ok()?;
    let value = text.trim();
    (value != "max").then(|| value.parse().ok()).flatten()
}

fn keyed_number(text: &str, key: &str) -> Option<u64> {
    text.lines().find_map(|line| {
        let (found, value) = line.split_once(' ')?;
        (found == key).then(|| value.trim().parse().ok()).flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cgroup_v2_files_become_typed_readings() {
        let root = std::env::temp_dir().join(format!("cybou-cgroup-{}", uuid::Uuid::new_v4()));
        let group = root.join("user.slice/cybou-test.service");
        std::fs::create_dir_all(&group).expect("fixture directory");
        std::fs::write(group.join("cgroup.procs"), "101\n202\n").expect("processes");
        std::fs::write(group.join("memory.current"), "67108864\n").expect("memory current");
        std::fs::write(group.join("memory.max"), "536870912\n").expect("memory max");
        std::fs::write(
            group.join("cpu.stat"),
            "usage_usec 42000\nuser_usec 30000\n",
        )
        .expect("cpu stat");
        std::fs::write(group.join("pids.current"), "3\n").expect("pids current");
        std::fs::write(group.join("pids.max"), "512\n").expect("pids max");

        let read = read_at(&root, "/user.slice/cybou-test.service");
        assert_eq!(read.process_count, Some(2));
        assert_eq!(read.memory_current_bytes, Some(67_108_864));
        assert_eq!(read.memory_max_bytes, Some(536_870_912));
        assert_eq!(read.cpu_usage_usec, Some(42_000));
        assert_eq!(read.pids_current, Some(3));
        assert_eq!(read.pids_max, Some(512));
        std::fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn unbounded_and_escaping_values_are_not_invented() {
        let root = std::env::temp_dir().join(format!("cybou-cgroup-{}", uuid::Uuid::new_v4()));
        let group = root.join("safe");
        std::fs::create_dir_all(&group).expect("fixture directory");
        std::fs::write(group.join("memory.max"), "max\n").expect("unbounded memory");
        assert_eq!(read_at(&root, "/safe").memory_max_bytes, None);
        assert_eq!(read_at(&root, "/../outside"), CgroupReadings::default());
        std::fs::remove_dir_all(root).expect("remove fixture");
    }
}
