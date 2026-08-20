// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Read-only perception sources for the staged Rust replacement of `perceptiond`.
//!
//! Provides the production Linux/Debian system perception source (`linux.system`)
//! and the legacy NixOS system-generation source (`nixos.system`) as a migration oracle.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime, UtcOffset, format_description};

use cybou_protocol::observation::ObservationV1;

/// Production Linux system source identifier.
pub const LINUX_SYSTEM_SOURCE_ID: &str = "linux.system";
/// Production Linux system subject.
pub const LINUX_SYSTEM_SUBJECT: &str = "operating-system";

/// Legacy NixOS system generation source identifier (retained as migration oracle).
pub const NIXOS_SYSTEM_SOURCE_ID: &str = "nixos.system";
/// Legacy NixOS system generation subject.
pub const NIXOS_SYSTEM_SUBJECT: &str = "current-system";

/// Legacy alias for compatibility.
pub const SYSTEM_SOURCE_ID: &str = NIXOS_SYSTEM_SOURCE_ID;
/// Legacy alias for compatibility.
pub const SYSTEM_SUBJECT: &str = NIXOS_SYSTEM_SUBJECT;

const DEFAULT_FRESHNESS_SECONDS: i64 = 300;

/// Why one acquisition did or did not produce an observation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AcquisitionStatus {
    /// The source was read and produced a structurally valid observation.
    Acquired,
    /// The path is absent or is not accessible.
    SourceUnavailable,
    /// The source exists but cannot produce the required identity/structure.
    SourceMalformed,
}

impl AcquisitionStatus {
    /// Wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Acquired => "acquired",
            Self::SourceUnavailable => "source-unavailable",
            Self::SourceMalformed => "source-malformed",
        }
    }
}

/// What an observation reports: a fact stated in words, or a measured quantity.
#[derive(Clone, Debug, PartialEq)]
pub enum ObservedValue {
    /// A fact whose value is words: a kernel release, a hostname.
    Text(String),
    /// A measured quantity: a count, a size, a duration.
    Number(i64),
}

impl ObservedValue {
    /// The value as a person would read it.
    #[must_use]
    pub fn display(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Number(number) => number.to_string(),
        }
    }
}

impl From<ObservedValue> for ciborium::Value {
    fn from(value: ObservedValue) -> Self {
        match value {
            ObservedValue::Text(text) => Self::Text(text),
            ObservedValue::Number(number) => Self::Integer(number.into()),
        }
    }
}

/// One valid non-sensitive system observation.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemObservation {
    /// Stable source identifier, distinct from the producing organ.
    pub source_id: &'static str,
    /// Subject whose value was observed.
    pub subject: &'static str,
    /// Value representing the observed state.
    ///
    /// A count is a number and is carried as one. Rendering it as text made every consumer that
    /// reasons about quantities — forecasting, calibration, anything comparing two readings —
    /// unable to see it as a quantity at all, and the ones that only display it lose nothing.
    pub value: ObservedValue,
    /// Acquisition instant supplied by the caller's clock.
    pub acquired_at: OffsetDateTime,
    /// End of the observation's declared freshness horizon.
    pub freshness_until: OffsetDateTime,
    /// Human-readable local provenance.
    pub provenance: String,
}

impl SystemObservation {
    /// Convert the acquired value into the byte-proven protocol payload.
    ///
    /// # Errors
    ///
    /// Returns a formatting error only when the frozen timestamp format cannot be applied.
    pub fn into_protocol(self) -> Result<ObservationV1, time::error::Format> {
        Ok(ObservationV1 {
            source_id: self.source_id.into(),
            subject: self.subject.into(),
            value: self.value.into(),
            acquired_at: qt_utc_milliseconds(self.acquired_at)?,
            freshness_until: qt_utc_milliseconds(self.freshness_until)?,
            provenance: self.provenance,
        })
    }
}

fn qt_utc_milliseconds(value: OffsetDateTime) -> Result<String, time::error::Format> {
    let format = format_description::parse_borrowed::<2>(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z",
    )
    .expect("frozen timestamp format is valid");
    value.to_offset(UtcOffset::UTC).format(&format)
}

/// Typed result that never turns inability to observe into an observed empty value.
#[derive(Clone, Debug, PartialEq)]
pub struct AcquisitionResult {
    /// Acquisition classification.
    pub status: AcquisitionStatus,
    /// Present only for [`AcquisitionStatus::Acquired`].
    pub observation: Option<SystemObservation>,
    /// Diagnostic for unavailable or malformed sources.
    pub detail: Option<String>,
}

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
fn parse_os_release(content: &str) -> std::collections::BTreeMap<String, String> {
    let mut map = std::collections::BTreeMap::new();
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

/// Legacy read-only adapter for the `/run/current-system` NixOS symlink contract (migration oracle).
#[derive(Clone, Debug)]
pub struct SystemGenerationSource {
    system_link_path: PathBuf,
    freshness_seconds: i64,
}

/// Alias for [`SystemGenerationSource`] clarifying its legacy oracle status.
pub type NixosSystemSource = SystemGenerationSource;

impl SystemGenerationSource {
    /// Construct an injectable source. Non-positive freshness uses the predecessor's 300 seconds.
    #[must_use]
    pub fn new(system_link_path: PathBuf, freshness_seconds: i64) -> Self {
        Self {
            system_link_path,
            freshness_seconds: if freshness_seconds > 0 {
                freshness_seconds
            } else {
                DEFAULT_FRESHNESS_SECONDS
            },
        }
    }

    /// Read the source once without mutating the observed system.
    #[must_use]
    pub fn acquire(&self, now: OffsetDateTime) -> AcquisitionResult {
        let metadata = match fs::symlink_metadata(&self.system_link_path) {
            Ok(metadata) if metadata.file_type().is_symlink() => metadata,
            Ok(_) => return self.unavailable("is not a symbolic link"),
            Err(_) => return self.unavailable("does not exist"),
        };
        debug_assert!(metadata.file_type().is_symlink());
        let Ok(target) = fs::read_link(&self.system_link_path) else {
            return self.malformed("resolves to nothing");
        };
        let Some(build_identity) = target.file_name().and_then(|name| name.to_str()) else {
            return self.malformed("has no final component");
        };
        if build_identity.is_empty() {
            return self.malformed("has no final component");
        }
        AcquisitionResult {
            status: AcquisitionStatus::Acquired,
            observation: Some(SystemObservation {
                source_id: NIXOS_SYSTEM_SOURCE_ID,
                subject: NIXOS_SYSTEM_SUBJECT,
                value: ObservedValue::Text(build_identity.to_owned()),
                acquired_at: now,
                freshness_until: now + Duration::seconds(self.freshness_seconds),
                provenance: format!(
                    "symlink target of {} resolved to {}",
                    self.system_link_path.display(),
                    target.display()
                ),
            }),
            detail: None,
        }
    }

    fn unavailable(&self, reason: &str) -> AcquisitionResult {
        AcquisitionResult {
            status: AcquisitionStatus::SourceUnavailable,
            observation: None,
            detail: Some(format!("{} {reason}", self.system_link_path.display())),
        }
    }

    fn malformed(&self, reason: &str) -> AcquisitionResult {
        AcquisitionResult {
            status: AcquisitionStatus::SourceMalformed,
            observation: None,
            detail: Some(format!("{} {reason}", self.system_link_path.display())),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn host_facts_that_cannot_be_read_produce_no_observation() {
        let dir = tempfile::tempdir().expect("temp dir");
        let kernel = dir.path().join("osrelease");
        let hostname = dir.path().join("hostname");
        let cpuinfo = dir.path().join("cpuinfo");
        let meminfo = dir.path().join("meminfo");

        std::fs::write(
            &kernel,
            "6.12.0-amd64
",
        )
        .expect("write kernel version");
        std::fs::write(
            &cpuinfo,
            "processor	: 0
processor	: 1
",
        )
        .expect("write cpuinfo");
        std::fs::write(
            &meminfo,
            "MemTotal:        8039284 kB
MemFree:  1 kB
",
        )
        .expect("write meminfo");
        // hostname is deliberately absent.

        let source = super::LinuxHostSource::new(kernel, hostname, cpuinfo, meminfo, 300);
        let observed = source.acquire(time::OffsetDateTime::UNIX_EPOCH);

        let subjects: Vec<_> = observed.iter().map(|o| o.subject).collect();
        assert_eq!(
            subjects,
            vec!["kernel-version", "cpu-count", "memory-total-kib"]
        );
        assert_eq!(
            observed[0].value,
            super::ObservedValue::Text("6.12.0-amd64".into())
        );
        // Counts and sizes are carried as numbers, which is what makes them comparable to the
        // next reading rather than two strings that happen to differ.
        assert_eq!(observed[1].value, super::ObservedValue::Number(2));
        assert_eq!(observed[2].value, super::ObservedValue::Number(8_039_284));
    }

    use std::fs;
    use tempfile::tempdir;
    use time::OffsetDateTime;

    use super::{
        AcquisitionStatus, LINUX_SYSTEM_SOURCE_ID, LINUX_SYSTEM_SUBJECT, LinuxSystemSource,
    };
    #[cfg(unix)]
    use super::{NIXOS_SYSTEM_SOURCE_ID, NIXOS_SYSTEM_SUBJECT, SystemGenerationSource};

    #[test]
    fn linux_system_source_parses_debian_os_release() {
        let root = tempdir().expect("temporary directory");
        let os_release_file = root.path().join("os-release");
        let machine_id_file = root.path().join("machine-id");

        let os_release_content = r#"
PRETTY_NAME="Debian GNU/Linux 13 (trixie)"
NAME="Debian GNU/Linux"
VERSION_ID="13"
VERSION="13 (trixie)"
ID=debian
"#;
        fs::write(&os_release_file, os_release_content).expect("write os-release");
        fs::write(&machine_id_file, "a1b2c3d4e5f60718293a4b5c6d7e8f90\n")
            .expect("write machine-id");

        let now = OffsetDateTime::from_unix_timestamp(1_787_090_000).expect("fixed clock");
        let source = LinuxSystemSource::new(os_release_file, Some(machine_id_file), 300);
        let result = source.acquire(now);

        assert_eq!(result.status, AcquisitionStatus::Acquired);
        let observation = result.observation.expect("typed observation");
        assert_eq!(observation.source_id, LINUX_SYSTEM_SOURCE_ID);
        assert_eq!(observation.subject, LINUX_SYSTEM_SUBJECT);
        assert_eq!(
            observation.value,
            super::ObservedValue::Text("Debian GNU/Linux 13 (trixie)".into())
        );
        assert_eq!((observation.freshness_until - now).whole_seconds(), 300);
        assert!(
            observation
                .provenance
                .contains("machine-id: a1b2c3d4e5f60718293a4b5c6d7e8f90")
        );

        let protocol = observation.into_protocol().expect("protocol observation");
        assert_eq!(protocol.source_id, LINUX_SYSTEM_SOURCE_ID);
        assert_eq!(protocol.subject, LINUX_SYSTEM_SUBJECT);
        assert!(protocol.encode().is_ok());
        assert!(protocol.message_id().is_ok());
    }

    #[test]
    fn linux_system_source_absent_path_is_unavailable() {
        let root = tempdir().expect("temporary directory");
        let absent_file = root.path().join("absent-os-release");
        let now = OffsetDateTime::UNIX_EPOCH;
        let source = LinuxSystemSource::new(absent_file, None, 300);
        let result = source.acquire(now);

        assert_eq!(result.status, AcquisitionStatus::SourceUnavailable);
        assert!(result.observation.is_none());
    }

    #[test]
    fn linux_system_source_malformed_when_no_name() {
        let root = tempdir().expect("temporary directory");
        let malformed_file = root.path().join("os-release");
        fs::write(&malformed_file, "FOO=BAR\nBAZ=QUX\n").expect("write malformed");
        let now = OffsetDateTime::UNIX_EPOCH;
        let source = LinuxSystemSource::new(malformed_file, None, 300);
        let result = source.acquire(now);

        assert_eq!(result.status, AcquisitionStatus::SourceMalformed);
        assert!(result.observation.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn nixos_legacy_oracle_symlink_produces_predecessor_contract() {
        use std::os::unix::fs::symlink;

        let root = tempdir().expect("temporary source tree");
        let target = root.path().join("abc-nixos-system-host-26.05");
        fs::create_dir(&target).expect("system generation");
        let link = root.path().join("current-system");
        symlink(&target, &link).expect("current system link");
        let now = OffsetDateTime::from_unix_timestamp(1_787_090_000).expect("fixed clock");

        let result = SystemGenerationSource::new(link, 300).acquire(now);

        assert_eq!(result.status, AcquisitionStatus::Acquired);
        let observation = result.observation.expect("typed observation");
        assert_eq!(observation.source_id, NIXOS_SYSTEM_SOURCE_ID);
        assert_eq!(observation.subject, NIXOS_SYSTEM_SUBJECT);
        assert_eq!(
            observation.value,
            super::ObservedValue::Text("abc-nixos-system-host-26.05".into())
        );
        assert_eq!((observation.freshness_until - now).whole_seconds(), 300);

        let protocol = observation.into_protocol().expect("protocol observation");
        assert_eq!(protocol.source_id, NIXOS_SYSTEM_SOURCE_ID);
        assert_eq!(protocol.subject, NIXOS_SYSTEM_SUBJECT);
        assert_eq!(protocol.acquired_at, "2026-08-18T21:53:20.000Z");
        assert!(protocol.encode().is_ok());
        assert!(protocol.message_id().is_ok());
    }
}

/// Read-only adapter for the host facts that describe the machine rather than measure it.
///
/// Deliberately not telemetry. Load, free memory and temperature change every time they are read,
/// and a Journal that is a biography should not fill with the fact that a number moved. These are
/// the facts that stay put and mean something when they do change: a kernel upgrade, a rename, a
/// resized machine. Each is a separate subject, so two of them changing in one sweep is a real
/// co-occurrence rather than an artefact of reading them together.
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
    ///
    /// A source that cannot be read yields no observation for that subject rather than an
    /// observation of nothing, so an absent file never becomes an asserted value.
    #[must_use]
    pub fn acquire(&self, now: OffsetDateTime) -> Vec<SystemObservation> {
        let freshness_until = now + time::Duration::seconds(self.freshness_seconds);
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

fn read_trimmed(path: &Path) -> Option<String> {
    let value = fs::read_to_string(path).ok()?.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

fn read_cpu_count(path: &Path) -> Option<usize> {
    let content = fs::read_to_string(path).ok()?;
    let count = content
        .lines()
        .filter(|line| line.starts_with("processor"))
        .count();
    (count > 0).then_some(count)
}

fn read_total_memory_kib(path: &Path) -> Option<u64> {
    let content = fs::read_to_string(path).ok()?;
    content
        .lines()
        .find_map(|line| line.strip_prefix("MemTotal:"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}
