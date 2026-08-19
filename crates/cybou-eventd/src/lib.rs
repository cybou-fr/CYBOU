// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Canonical Journal writer and Event1 D-Bus service daemon for Cybou.
//!
//! Provides the single authoritative Event1 ownership boundary, durable-before-visible
//! transaction guarantees, origin authentication, and consumer offset tracking.

use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

use cybou_crypto::{KeyDomain, KeyStore};
use cybou_protocol::canonical::CanonicalEnvelope;
use cybou_storage::writer::{Appended, JournalWriter, WriteError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[cfg(target_os = "linux")]
pub mod service;

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

/// Errors occurring within EventCore.
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

/// Consumer offsets persisted schema.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct PersistedOffsets {
    version: u32,
    offsets: HashMap<String, String>,
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

/// Pure core domain logic for Event1 service, wrapping JournalWriter, KeyStore, and offsets.
pub struct EventCore {
    writer: Mutex<JournalWriter>,
    offsets_path: PathBuf,
    offsets: Mutex<HashMap<String, u64>>,
    checkpoint_path: PathBuf,
}

impl EventCore {
    /// Checkpoint file path.
    #[must_use]
    pub fn checkpoint_path(&self) -> &Path {
        &self.checkpoint_path
    }
    /// Open EventCore around a journal database file.
    ///
    /// # Errors
    ///
    /// Returns [`EventError`] if the journal cannot be opened or created.
    pub fn open(journal_path: impl AsRef<Path>) -> Result<Self, EventError> {
        let journal_path = journal_path.as_ref();
        let parent_dir = journal_path.parent().unwrap_or_else(|| Path::new("."));
        let offsets_path = parent_dir.join("consumer-offsets.json");
        let checkpoint_path = parent_dir.join("verification-checkpoint.json");

        let writer = JournalWriter::open(journal_path).map_err(EventError::Storage)?;

        let core = Self {
            writer: Mutex::new(writer),
            offsets_path,
            offsets: Mutex::new(HashMap::new()),
            checkpoint_path,
        };

        core.load_offsets();
        Ok(core)
    }

    /// Attach a KeyStore to the underlying JournalWriter for sensitive payload sealing.
    pub fn set_key_store(&self, key_store: KeyStore, kek: [u8; 32], key_domain: KeyDomain) {
        if let Ok(mut writer) = self.writer.lock() {
            writer.set_key_store(key_store, kek, key_domain);
        }
    }

    /// Submit a contribution with caller origin checking.
    ///
    /// # Errors
    ///
    /// Returns [`EventError`] if admission or write fails.
    pub fn submit(
        &self,
        envelope: &CanonicalEnvelope,
        caller_organ: Option<&str>,
    ) -> Result<Appended, EventError> {
        // Validate origin provenance
        if let Some(caller) = caller_organ {
            if envelope.origin_organ != caller {
                return Err(EventError::OriginUnauthentic(envelope.origin_organ.clone()));
            }
        } else if is_reserved_organ(&envelope.origin_organ) {
            return Err(EventError::OriginUnauthentic(envelope.origin_organ.clone()));
        }

        // ADR-0028: submitting a contribution never authorizes an erasure.
        if envelope.kind == 13 || envelope.kind == 14 {
            // ErasurePlanned (13) or ErasureExecuted (14)
            return Err(EventError::ErasureRefused);
        }

        let mut writer = self
            .writer
            .lock()
            .map_err(|_| EventError::Storage(WriteError::Malformed("lock poisoned")))?;
        writer.append(envelope).map_err(EventError::Storage)
    }

    /// Return total contribution count in the Journal.
    pub fn count(&self) -> u64 {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.count().ok())
            .unwrap_or(0)
    }

    /// Return head envelope, if any.
    pub fn head(&self) -> Option<CanonicalEnvelope> {
        self.writer.lock().ok().and_then(|w| w.head().ok()).flatten()
    }

    /// Retrieve an envelope at a specific sequence.
    pub fn at_sequence(&self, sequence: u64) -> Option<CanonicalEnvelope> {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.at_sequence(sequence).ok())
            .flatten()
    }

    /// Replay contributions strictly after `after_sequence` up to `limit`.
    pub fn replay(&self, after_sequence: u64, limit: usize) -> Vec<CanonicalEnvelope> {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.replay(after_sequence, limit).ok())
            .unwrap_or_default()
    }

    /// Return recent contributions up to `limit`.
    pub fn recent(&self, limit: usize) -> Vec<CanonicalEnvelope> {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.recent(limit).ok())
            .unwrap_or_default()
    }

    /// Find an envelope by message ID.
    pub fn find_by_message_id(&self, message_id: &Uuid) -> Option<CanonicalEnvelope> {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.find_by_message_id(message_id).ok())
            .flatten()
    }

    /// Find all envelopes in an episode by correlation ID.
    pub fn find_by_correlation_id(&self, correlation_id: &Uuid) -> Vec<CanonicalEnvelope> {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.find_by_correlation_id(correlation_id).ok())
            .unwrap_or_default()
    }

    /// Check whether a terminal outcome exists for a cause and organ.
    pub fn has_outcome_for(&self, cause_id: &Uuid, origin_organ: &str) -> bool {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.has_outcome_for(cause_id, origin_organ).ok())
            .unwrap_or(false)
    }

    /// Evidence UUIDs for a contribution message ID.
    pub fn evidence_for(&self, message_id: &Uuid) -> Vec<Uuid> {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.evidence_for(message_id).ok())
            .unwrap_or_default()
    }

    /// Current erasure epoch.
    pub fn erasure_epoch(&self) -> u64 {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.erasure_epoch().ok())
            .unwrap_or(0)
    }

    /// Ensure consumer offset registration.
    pub fn ensure_consumer(&self, consumer_id: &str, initial_offset: u64) -> bool {
        if !is_valid_consumer_id(consumer_id) {
            return false;
        }
        let mut offsets = match self.offsets.lock() {
            Ok(guard) => guard,
            Err(_) => return false,
        };
        offsets.entry(consumer_id.to_string()).or_insert(initial_offset);
        self.save_offsets(&offsets)
    }

    /// Advance consumer offset.
    pub fn advance_consumer(&self, consumer_id: &str, offset: u64) -> bool {
        if !is_valid_consumer_id(consumer_id) {
            return false;
        }
        let mut offsets = match self.offsets.lock() {
            Ok(guard) => guard,
            Err(_) => return false,
        };
        let head = self.count();
        if offset > head {
            return false;
        }
        offsets.insert(consumer_id.to_string(), offset);
        self.save_offsets(&offsets)
    }

    /// Calculate consumer backlog.
    pub fn consumer_backlog(&self, consumer_id: &str) -> Option<u64> {
        let offsets = self.offsets.lock().ok()?;
        let current = offsets.get(consumer_id).copied()?;
        let head = self.count();
        Some(head.saturating_sub(current))
    }

    fn load_offsets(&self) {
        if !self.offsets_path.exists() {
            return;
        }
        let Ok(mut file) = File::open(&self.offsets_path) else {
            return;
        };
        let mut content = String::new();
        if file.read_to_string(&mut content).is_err() {
            return;
        }
        if let Ok(persisted) = serde_json::from_str::<PersistedOffsets>(&content) {
            if persisted.version == 1 {
                let mut map = HashMap::new();
                for (k, v) in persisted.offsets {
                    if let Ok(offset) = v.parse::<u64>() {
                        map.insert(k, offset);
                    }
                }
                if let Ok(mut offsets) = self.offsets.lock() {
                    *offsets = map;
                }
            }
        }
    }

    fn save_offsets(&self, offsets: &HashMap<String, u64>) -> bool {
        let mut map = HashMap::new();
        for (k, v) in offsets {
            map.insert(k.clone(), v.to_string());
        }
        let persisted = PersistedOffsets {
            version: 1,
            offsets: map,
        };
        let Ok(json) = serde_json::to_string(&persisted) else {
            return false;
        };
        let temp_path = self.offsets_path.with_extension("tmp");
        if let Ok(mut file) = File::create(&temp_path) {
            if file.write_all(json.as_bytes()).is_ok() && file.flush().is_ok() {
                return fs::rename(&temp_path, &self.offsets_path).is_ok();
            }
        }
        false
    }
}

fn is_valid_consumer_id(consumer_id: &str) -> bool {
    if consumer_id.is_empty() || consumer_id.len() > 64 {
        return false;
    }
    let first = consumer_id.chars().next().unwrap_or('\0');
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    consumer_id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;

    fn dummy_envelope(message_id: Uuid, origin_organ: &str) -> CanonicalEnvelope {
        CanonicalEnvelope {
            schema_version: 3,
            message_id,
            correlation_id: Uuid::new_v4(),
            causation_id: Uuid::nil(),
            origin_organ: origin_organ.to_string(),
            origin_node: String::new(),
            kind: 1, // Observation
            wall_time_ms: OffsetDateTime::now_utc().unix_timestamp_nanos() as i64 / 1_000_000,
            monotonic_time: 100,
            logical_clock: 1,
            confidence: 1.0,
            evidence: vec![],
            payload: vec![1, 2, 3, 4],
            privacy: 1,
            capability_scope: String::new(),
            sealed: false,
            key_domain_id: Uuid::nil(),
            key_epoch: 0,
            retention_class: 2,
            retention_policy_version: 0,
            retain_until_ms: 0,
            sensitivity: 1,
        }
    }

    #[test]
    fn submit_and_query_lifecycle() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("journal.sqlite3");
        let core = EventCore::open(&db_path).expect("open event core");

        assert_eq!(core.count(), 0);
        assert_eq!(core.head(), None);

        let env1 = dummy_envelope(Uuid::new_v4(), "unreserved_client");
        let res = core.submit(&env1, None).expect("submit");
        assert_eq!(res.sequence, 1);

        assert_eq!(core.count(), 1);
        let head = core.head().expect("head exists");
        assert_eq!(head.message_id, env1.message_id);

        let retrieved = core.at_sequence(1).expect("at sequence 1");
        assert_eq!(retrieved.message_id, env1.message_id);

        let found = core.find_by_message_id(&env1.message_id).expect("found by id");
        assert_eq!(found.message_id, env1.message_id);
    }

    #[test]
    fn unauthenticated_reserved_origin_is_refused() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("journal.sqlite3");
        let core = EventCore::open(&db_path).expect("open event core");

        // Attempting to claim reserved organ 'identityd' without proof
        let env = dummy_envelope(Uuid::new_v4(), "identityd");
        let err = core.submit(&env, None).expect_err("should refuse");
        assert!(matches!(err, EventError::OriginUnauthentic(_)));
    }
}
