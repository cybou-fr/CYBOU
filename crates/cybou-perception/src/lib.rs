// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Read-only perception sources for the staged Rust replacement of `perceptiond`.

use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

/// Stable identity of the first non-sensitive local source.
pub const SYSTEM_SOURCE_ID: &str = "nixos.system";
/// Stable subject described by the system-generation source.
pub const SYSTEM_SUBJECT: &str = "current-system";
const DEFAULT_FRESHNESS_SECONDS: i64 = 300;

/// Why one acquisition did or did not produce an observation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AcquisitionStatus {
    /// The symlink was read and produced a structurally valid observation.
    Acquired,
    /// The path is absent or is not a symbolic link.
    SourceUnavailable,
    /// A symbolic link exists but cannot produce the required build identity.
    SourceMalformed,
}

impl AcquisitionStatus {
    /// Frozen predecessor wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Acquired => "acquired",
            Self::SourceUnavailable => "source-unavailable",
            Self::SourceMalformed => "source-malformed",
        }
    }
}

/// One valid non-sensitive system-generation observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemObservation {
    /// Stable source identifier, distinct from the producing organ.
    pub source_id: &'static str,
    /// Subject whose value was observed.
    pub subject: &'static str,
    /// Final component of the current-system symlink target.
    pub value: String,
    /// Acquisition instant supplied by the caller's clock.
    pub acquired_at: OffsetDateTime,
    /// End of the observation's declared freshness horizon.
    pub freshness_until: OffsetDateTime,
    /// Human-readable local provenance.
    pub provenance: String,
}

/// Typed result that never turns inability to observe into an observed empty value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcquisitionResult {
    /// Acquisition classification.
    pub status: AcquisitionStatus,
    /// Present only for [`AcquisitionStatus::Acquired`].
    pub observation: Option<SystemObservation>,
    /// Diagnostic for unavailable or malformed sources.
    pub detail: Option<String>,
}

/// Read-only adapter for the `/run/current-system` symlink contract.
#[derive(Clone, Debug)]
pub struct SystemGenerationSource {
    system_link_path: PathBuf,
    freshness_seconds: i64,
}

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
                source_id: SYSTEM_SOURCE_ID,
                subject: SYSTEM_SUBJECT,
                value: build_identity.to_owned(),
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
    use std::{fs, os::unix::fs::symlink};

    use tempfile::tempdir;
    use time::OffsetDateTime;

    use super::{AcquisitionStatus, SYSTEM_SOURCE_ID, SYSTEM_SUBJECT, SystemGenerationSource};

    #[test]
    fn valid_symlink_produces_the_predecessor_observation_contract() {
        let root = tempdir().expect("temporary source tree");
        let target = root.path().join("abc-nixos-system-host-26.05");
        fs::create_dir(&target).expect("system generation");
        let link = root.path().join("current-system");
        symlink(&target, &link).expect("current system link");
        let now = OffsetDateTime::from_unix_timestamp(1_787_090_000).expect("fixed clock");

        let result = SystemGenerationSource::new(link, 300).acquire(now);

        assert_eq!(result.status, AcquisitionStatus::Acquired);
        let observation = result.observation.expect("typed observation");
        assert_eq!(observation.source_id, SYSTEM_SOURCE_ID);
        assert_eq!(observation.subject, SYSTEM_SUBJECT);
        assert_eq!(observation.value, "abc-nixos-system-host-26.05");
        assert_eq!((observation.freshness_until - now).whole_seconds(), 300);
    }

    #[test]
    fn absent_or_regular_path_is_unavailable_without_an_observation() {
        let root = tempdir().expect("temporary source tree");
        let absent = root.path().join("absent");
        let now = OffsetDateTime::UNIX_EPOCH;
        let result = SystemGenerationSource::new(absent, 300).acquire(now);
        assert_eq!(result.status, AcquisitionStatus::SourceUnavailable);
        assert!(result.observation.is_none());

        let regular = root.path().join("current-system");
        fs::write(&regular, b"not a link").expect("regular file");
        let result = SystemGenerationSource::new(regular, 300).acquire(now);
        assert_eq!(result.status, AcquisitionStatus::SourceUnavailable);
        assert!(result.observation.is_none());
    }

    #[test]
    fn non_positive_freshness_uses_the_frozen_default() {
        let root = tempdir().expect("temporary source tree");
        let target = root.path().join("generation");
        fs::create_dir(&target).expect("system generation");
        let link = root.path().join("current-system");
        symlink(target, &link).expect("current system link");
        let now = OffsetDateTime::UNIX_EPOCH;
        let observation = SystemGenerationSource::new(link, 0)
            .acquire(now)
            .observation
            .expect("typed observation");
        assert_eq!((observation.freshness_until - now).whole_seconds(), 300);
    }
}
