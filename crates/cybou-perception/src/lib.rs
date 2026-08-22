// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Read-only perception sources for the staged Rust replacement of `perceptiond`.
//!
//! Provides the production Linux/Debian system perception source (`linux.system`)
//! and the legacy NixOS system-generation source (`nixos.system`) as a migration oracle.

pub mod sources;
pub mod types;

pub use sources::{
    LinuxHostSource, LinuxSystemSource, NixosSystemSource, SystemGenerationSource, parse_os_release,
};
pub use types::{
    AcquisitionResult, AcquisitionStatus, DEFAULT_FRESHNESS_SECONDS, LINUX_SYSTEM_SOURCE_ID,
    LINUX_SYSTEM_SUBJECT, NIXOS_SYSTEM_SOURCE_ID, NIXOS_SYSTEM_SUBJECT, ObservedValue,
    SYSTEM_SOURCE_ID, SYSTEM_SUBJECT, SystemObservation, qt_utc_milliseconds,
};

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::tempdir;
    use time::OffsetDateTime;

    use super::{
        AcquisitionStatus, LINUX_SYSTEM_SOURCE_ID, LINUX_SYSTEM_SUBJECT, LinuxHostSource,
        LinuxSystemSource, ObservedValue,
    };
    #[cfg(unix)]
    use super::{NIXOS_SYSTEM_SOURCE_ID, NIXOS_SYSTEM_SUBJECT, SystemGenerationSource};

    #[test]
    fn host_facts_that_cannot_be_read_produce_no_observation() {
        let dir = tempfile::tempdir().expect("temp dir");
        let kernel = dir.path().join("osrelease");
        let hostname = dir.path().join("hostname");
        let cpuinfo = dir.path().join("cpuinfo");
        let meminfo = dir.path().join("meminfo");

        std::fs::write(&kernel, "6.12.0-amd64\n").expect("write kernel version");
        std::fs::write(&cpuinfo, "processor\t: 0\nprocessor\t: 1\n").expect("write cpuinfo");
        std::fs::write(&meminfo, "MemTotal:        8039284 kB\nMemFree:  1 kB\n")
            .expect("write meminfo");
        // hostname is deliberately absent.

        let source = LinuxHostSource::new(kernel, hostname, cpuinfo, meminfo, 300);
        let observed = source.acquire(time::OffsetDateTime::UNIX_EPOCH);

        let subjects: Vec<_> = observed.iter().map(|o| o.subject).collect();
        assert_eq!(
            subjects,
            vec!["kernel-version", "cpu-count", "memory-total-kib"]
        );
        assert_eq!(
            observed[0].value,
            ObservedValue::Text("6.12.0-amd64".into())
        );
        assert_eq!(observed[1].value, ObservedValue::Number(2));
        assert_eq!(observed[2].value, ObservedValue::Number(8_039_284));
    }

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
            ObservedValue::Text("Debian GNU/Linux 13 (trixie)".into())
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
            ObservedValue::Text("abc-nixos-system-host-26.05".into())
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
