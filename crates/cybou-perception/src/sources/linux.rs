// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Production Linux and Debian host perception sources.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use time::{Duration, OffsetDateTime};

use crate::types::{
    AcquisitionResult, AcquisitionStatus, DEFAULT_FRESHNESS_SECONDS, LINUX_SYSTEM_SOURCE_ID,
    LINUX_SYSTEM_SUBJECT, ObservedValue, SystemObservation,
};

/// Production read-only adapter for standard Linux/Debian system identity (`/etc/os-release`, `/etc/machine-id`).
#[derive(Clone, Debug)]
pub struct LinuxSystemSource {
    os_release_path: PathBuf,
    machine_id_path: Option<PathBuf>,
    freshness_seconds: i64,
}

impl LinuxSystemSource {
    /// Construct a new Linux system perception source with standard default paths.
    #[must_use]
    pub fn new_standard(freshness_seconds: i64) -> Self {
        Self::new(
            PathBuf::from("/etc/os-release"),
            Some(PathBuf::from("/etc/machine-id")),
            freshness_seconds,
        )
    }

    /// Construct an injectable source with custom paths (useful for testing).
    #[must_use]
    pub fn new(
        os_release_path: PathBuf,
        machine_id_path: Option<PathBuf>,
        freshness_seconds: i64,
    ) -> Self {
        Self {
            os_release_path,
            machine_id_path,
            freshness_seconds: if freshness_seconds > 0 {
                freshness_seconds
            } else {
                DEFAULT_FRESHNESS_SECONDS
            },
        }
    }

    /// Read the system identity once without mutating the observed system.
    #[must_use]
    pub fn acquire(&self, now: OffsetDateTime) -> AcquisitionResult {
        let Ok(content) = fs::read_to_string(&self.os_release_path) else {
            return self.unavailable("cannot be read");
        };

        let parsed = parse_os_release(&content);
        let Some(pretty_or_name) = parsed.get("PRETTY_NAME").or_else(|| parsed.get("NAME")) else {
            return self.malformed("contains no PRETTY_NAME or NAME field");
        };

        if pretty_or_name.trim().is_empty() {
            return self.malformed("has empty PRETTY_NAME / NAME");
        }

        let machine_id = self.machine_id_path.as_ref().and_then(|p| {
            fs::read_to_string(p)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        });

        let provenance = match machine_id {
            Some(mid) => format!(
                "os-release from {} (machine-id: {mid})",
                self.os_release_path.display()
            ),
            None => format!("os-release from {}", self.os_release_path.display()),
        };

        AcquisitionResult {
            status: AcquisitionStatus::Acquired,
            observation: Some(SystemObservation {
                source_id: LINUX_SYSTEM_SOURCE_ID,
                subject: LINUX_SYSTEM_SUBJECT,
                value: ObservedValue::Text(pretty_or_name.trim().to_string()),
                acquired_at: now,
                freshness_until: now + Duration::seconds(self.freshness_seconds),
                provenance,
            }),
            detail: None,
        }
    }

    fn unavailable(&self, reason: &str) -> AcquisitionResult {
        AcquisitionResult {
            status: AcquisitionStatus::SourceUnavailable,
            observation: None,
            detail: Some(format!("{} {reason}", self.os_release_path.display())),
        }
    }

    fn malformed(&self, reason: &str) -> AcquisitionResult {
        AcquisitionResult {
            status: AcquisitionStatus::SourceMalformed,
            observation: None,
            detail: Some(format!("{} {reason}", self.os_release_path.display())),
        }
    }
}

/// Simple parser for standard `os-release` key-value format.
#[must_use]
pub fn parse_os_release(content: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_string();
            let mut val = value.trim().to_string();
            if ((val.starts_with('"') && val.ends_with('"'))
                || (val.starts_with('\'') && val.ends_with('\'')))
                && val.len() >= 2
            {
                val = val[1..val.len() - 1].to_string();
            }
            map.insert(key, val);
        }
    }
    map
}

/// Read-only adapter for the host facts that describe the machine rather than measure it.
#[derive(Clone, Debug)]
pub struct LinuxHostSource {
    kernel_version_path: PathBuf,
    hostname_path: PathBuf,
    cpuinfo_path: PathBuf,
    meminfo_path: PathBuf,
    freshness_seconds: i64,
}

impl LinuxHostSource {
    /// Construct a source over the standard Linux paths.
    #[must_use]
    pub fn new_standard(freshness_seconds: i64) -> Self {
        Self::new(
            PathBuf::from("/proc/sys/kernel/osrelease"),
            PathBuf::from("/proc/sys/kernel/hostname"),
            PathBuf::from("/proc/cpuinfo"),
            PathBuf::from("/proc/meminfo"),
            freshness_seconds,
        )
    }

    /// Construct an injectable source with custom paths.
    #[must_use]
    pub const fn new(
        kernel_version_path: PathBuf,
        hostname_path: PathBuf,
        cpuinfo_path: PathBuf,
        meminfo_path: PathBuf,
        freshness_seconds: i64,
    ) -> Self {
        Self {
            kernel_version_path,
            hostname_path,
            cpuinfo_path,
            meminfo_path,
            freshness_seconds,
        }
    }

    /// Read every host fact that can be read, and none that cannot.
    #[must_use]
    pub fn acquire(&self, now: OffsetDateTime) -> Vec<SystemObservation> {
        let freshness_until = now + Duration::seconds(self.freshness_seconds);
        let mut observations = Vec::new();

        if let Some(value) = read_trimmed(&self.kernel_version_path) {
            observations.push(self.observe(
                "kernel-version",
                ObservedValue::Text(value),
                &self.kernel_version_path,
                now,
                freshness_until,
            ));
        }
        if let Some(value) = read_trimmed(&self.hostname_path) {
            observations.push(self.observe(
                "hostname",
                ObservedValue::Text(value),
                &self.hostname_path,
                now,
                freshness_until,
            ));
        }
        if let Some(count) = read_cpu_count(&self.cpuinfo_path) {
            observations.push(self.observe(
                "cpu-count",
                ObservedValue::Number(i64::try_from(count).unwrap_or(i64::MAX)),
                &self.cpuinfo_path,
                now,
                freshness_until,
            ));
        }
        if let Some(kib) = read_total_memory_kib(&self.meminfo_path) {
            observations.push(self.observe(
                "memory-total-kib",
                ObservedValue::Number(i64::try_from(kib).unwrap_or(i64::MAX)),
                &self.meminfo_path,
                now,
                freshness_until,
            ));
        }

        observations
    }

    fn observe(
        &self,
        subject: &'static str,
        value: ObservedValue,
        path: &Path,
        acquired_at: OffsetDateTime,
        freshness_until: OffsetDateTime,
    ) -> SystemObservation {
        let _ = self;
        SystemObservation {
            source_id: "linux.host",
            subject,
            value,
            acquired_at,
            freshness_until,
            provenance: format!("read from {}", path.display()),
        }
    }
}

/// Read trimmed string from file, returning `None` if absent or empty.
#[must_use]
pub fn read_trimmed(path: &Path) -> Option<String> {
    let value = fs::read_to_string(path).ok()?.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

/// Read processor count from `/proc/cpuinfo`.
#[must_use]
pub fn read_cpu_count(path: &Path) -> Option<usize> {
    let content = fs::read_to_string(path).ok()?;
    let count = content
        .lines()
        .filter(|line| line.starts_with("processor"))
        .count();
    (count > 0).then_some(count)
}

/// Read total memory in KiB from `/proc/meminfo`.
#[must_use]
pub fn read_total_memory_kib(path: &Path) -> Option<u64> {
    let content = fs::read_to_string(path).ok()?;
    content
        .lines()
        .find_map(|line| line.strip_prefix("MemTotal:"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}
