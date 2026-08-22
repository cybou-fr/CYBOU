// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Legacy read-only adapter for the `/run/current-system` NixOS symlink contract (migration oracle).

use std::{fs, path::PathBuf};
use time::{Duration, OffsetDateTime};

use crate::types::{
    AcquisitionResult, AcquisitionStatus, DEFAULT_FRESHNESS_SECONDS, NIXOS_SYSTEM_SOURCE_ID,
    NIXOS_SYSTEM_SUBJECT, ObservedValue, SystemObservation,
};

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
