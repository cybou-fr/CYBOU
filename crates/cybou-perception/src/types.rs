// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Perception acquisition types, values, statuses, and timestamp formatting.

use cybou_protocol::observation::ObservationV1;
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, UtcOffset, format_description};

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

/// Default freshness window in seconds.
pub const DEFAULT_FRESHNESS_SECONDS: i64 = 300;

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

/// Format UTC datetime in Qt ISO 8601 milliseconds format.
///
/// # Errors
///
/// Returns format error if formatting fails.
///
/// # Panics
///
/// Panics if the static format description string is invalid.
pub fn qt_utc_milliseconds(value: OffsetDateTime) -> Result<String, time::error::Format> {
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
