// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Journal storage inspection types, errors, and schema constants.

use thiserror::Error;

/// Current predecessor `SQLite` schema version.
pub const JOURNAL_SCHEMA_V2: i64 = 2;

/// Required tables in `SQLite` Journal v2.
pub const REQUIRED_TABLES: &[&str] = &["contribution", "contribution_evidence", "journal_meta"];

/// Required contribution columns in `SQLite` Journal v2.
pub const REQUIRED_CONTRIBUTION_COLUMNS: &[&str] = &[
    "seq",
    "message_id",
    "correlation_id",
    "causation_id",
    "origin_organ",
    "origin_node",
    "kind",
    "wall_time",
    "monotonic_time",
    "logical_clock",
    "confidence",
    "evidence",
    "payload",
    "privacy",
    "capability",
    "schema_version",
    "hash_version",
    "prev_hash",
    "hash",
    "commitment",
    "payload_commitment",
    "erased_at",
    "sealed",
    "key_domain",
    "key_epoch",
    "retention_class",
    "retention_policy",
    "retain_until",
    "sensitivity",
];

/// Verified immutable facts needed before a Rust Journal reader is attempted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JournalInspection {
    /// `SQLite` `user_version` accepted by this build.
    pub schema_version: i64,
    /// Number of canonical contribution rows, without decoding them yet.
    pub contribution_count: u64,
    /// Current erasure epoch from the singleton metadata row.
    pub erasure_epoch: u64,
    /// Highest backup-rotation declaration epoch.
    pub rotated_epoch: u64,
}

/// Trusted chain position from a previous successful verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JournalCheckpoint {
    /// Last verified sequence, or zero before the first row.
    pub sequence: u64,
    /// Stored hash at `sequence`, empty only when `sequence` is zero.
    pub hash: Vec<u8>,
}

/// Bounded verification facts for the suffix after a checkpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JournalVerification {
    /// Trusted checkpoint sequence supplied by the caller.
    pub verified_from: u64,
    /// Last sequence cryptographically replayed by this call.
    pub verified_through: u64,
    /// V3 payloads whose bytes were still present and matched their commitment.
    pub content_verified: u64,
    /// Erased v3 payloads skipped while their metadata remained verified.
    pub content_skipped: u64,
    /// Whether rows remain after this bounded page.
    pub has_more: bool,
    /// Checkpoint suitable for the next incremental verification.
    pub checkpoint: JournalCheckpoint,
}

/// Read-only compatibility refusal.
#[derive(Debug, Error)]
pub enum StorageError {
    /// The database cannot be opened read-only and must never be created implicitly.
    #[error("cannot open Journal read-only: {0}")]
    Open(#[source] rusqlite::Error),
    /// A query failed while inspecting immutable schema facts.
    #[error("cannot inspect Journal: {0}")]
    Query(#[source] rusqlite::Error),
    /// Only the frozen v2 schema is accepted by this first slice.
    #[error("unsupported Journal schema {received}; expected 2")]
    UnsupportedSchema {
        /// Version read from `PRAGMA user_version`.
        received: i64,
    },
    /// A required predecessor table or column is absent.
    #[error("Journal schema is missing {0}")]
    MissingSchema(String),
    /// A persisted non-negative counter cannot be represented safely.
    #[error("Journal contains an invalid persisted counter")]
    InvalidCounter,
    /// Stored chain links or hash material are structurally inconsistent.
    #[error("Journal hash chain is structurally invalid at sequence {sequence}: {reason}")]
    InvalidChain {
        /// First sequence at which the structural contract fails.
        sequence: u64,
        /// Stable diagnostic that does not expose payload contents.
        reason: &'static str,
    },
    /// A supplied checkpoint no longer names the same stored row.
    #[error("Journal checkpoint does not match sequence {sequence}")]
    CheckpointMismatch {
        /// Sequence whose stored hash differs or no longer exists.
        sequence: u64,
    },
    /// A paged replay must always make forward progress.
    #[error("Journal verification page size must be greater than zero")]
    InvalidPageSize,
}
