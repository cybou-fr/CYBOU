// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Error types, submission results, and origin authentication validation.

use std::path::PathBuf;

use cybou_storage::writer::WriteError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Organ identities that only the corresponding Mind process may claim as a contribution origin.
pub const RESERVED_ORGAN_IDENTITIES: &[&str] = &[
    "eventd",
    "healthd",
    "lifecycled",
    "identityd",
    "intentiond",
    "predictord",
    "selfd",
    "workspaced",
    "presenced",
    "perceptiond",
    "epistemicd",
    "contextd",
];

/// Check whether an organ name is in the reserved list.
#[must_use]
pub fn is_reserved_organ(origin: &str) -> bool {
    RESERVED_ORGAN_IDENTITIES.contains(&origin)
}

/// Errors occurring within `EventCore`.
#[derive(Debug, Error)]
pub enum EventError {
    /// Storage / Journal writer failure.
    #[error("storage error: {0}")]
    Storage(#[from] WriteError),
    /// Decoding failure.
    #[error("decode error: {0}")]
    Decode(String),
    /// Unauthentic origin.
    #[error("origin '{0}' does not belong to the calling process")]
    OriginUnauthentic(String),
    /// Erasure submission refused.
    #[error("erasure is not a contribution; it must be requested explicitly")]
    ErasureRefused,
    /// Invalid consumer ID format.
    #[error("invalid consumer ID '{0}'")]
    InvalidConsumerId(String),
    /// I/O failure.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// File path.
        path: PathBuf,
        /// Underlying error.
        source: std::io::Error,
    },
}

/// Result of submitting a contribution to Event1.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitResult {
    /// Assigned sequence number as string (matching Qt wire spelling).
    pub sequence: String,
    /// Error message, or empty string on success.
    pub error: String,
}

impl SubmitResult {
    /// Construct a success result with sequence number.
    #[must_use]
    pub fn success(sequence: u64) -> Self {
        Self {
            sequence: sequence.to_string(),
            error: String::new(),
        }
    }

    /// Construct a failure result.
    #[must_use]
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            sequence: "0".to_string(),
            error: error.into(),
        }
    }

    /// Encode the submit result into CBOR.
    #[must_use]
    pub fn to_cbor(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(self, &mut buf);
        buf
    }
}
