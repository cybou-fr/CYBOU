// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Journal hash chain verification status and persistent checkpoint types.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// What the last incremental verification established about the Journal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationState {
    /// Last sequence whose hash chain has been replayed.
    pub verified_through: u64,
    /// Journal head at the moment of the check.
    pub head: u64,
    /// Sequence where the chain first failed, if it did.
    pub broken_at: Option<u64>,
    /// V3 payloads whose bytes were present and matched their commitment.
    pub content_verified: u64,
    /// Erased v3 payloads skipped while their metadata stayed verified.
    pub content_skipped: u64,
    /// RFC 3339 instant of the check.
    pub taken_at: String,
}

impl VerificationState {
    /// Whether the whole chain up to the head has been replayed without a break.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.broken_at.is_none() && self.verified_through >= self.head
    }
}

/// One page of a full re-verification.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FullSweepStep {
    /// Last sequence this sweep has replayed.
    pub verified_through: u64,
    /// Journal head when the page ran.
    pub head: u64,
    /// Whether rows remain in this sweep.
    pub has_more: bool,
    /// Sequence where the chain first failed, if it did.
    pub broken_at: Option<u64>,
}

/// The checkpoint as persisted between runs.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedCheckpoint {
    /// Schema format version.
    pub version: u8,
    /// Sequence number verified through.
    pub sequence: u64,
    /// Checkpoint hash as hexadecimal string.
    pub hash: String,
}

/// Format instant as RFC3339 string.
#[must_use]
pub fn format_instant(now: OffsetDateTime) -> String {
    now.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

/// Encode byte slice as lowercase hexadecimal string.
#[must_use]
pub fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// Decode hexadecimal string into byte vector.
#[must_use]
pub fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(text.get(index..index + 2)?, 16).ok())
        .collect()
}
