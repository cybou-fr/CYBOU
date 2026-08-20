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
use cybou_protocol::{Kind, admission::ErasureReason, canonical::CanonicalEnvelope};
use cybou_storage::{
    JournalCheckpoint,
    writer::{Appended, Erased, JournalWriter, WriteError},
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
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

/// Pure core domain logic for Event1 service, wrapping `JournalWriter`, `KeyStore`, and offsets.
pub struct EventCore {
    writer: Mutex<JournalWriter>,
    offsets_path: PathBuf,
    offsets: Mutex<HashMap<String, u64>>,
    checkpoint_path: PathBuf,
    journal_path: PathBuf,
    verification: Mutex<Option<VerificationState>>,
    full_sweep: Mutex<Option<JournalCheckpoint>>,
}

/// What the last incremental verification established about the Journal.
///
/// It carries how far verification reached and where the head is, because "verified" is only
/// meaningful against a position: a chain proven intact through row 200 of 400 says nothing about
/// the other 200, and reporting that as verified would be the kind of claim this system exists to
/// avoid.
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
struct PersistedCheckpoint {
    version: u8,
    sequence: u64,
    hash: String,
}

impl EventCore {
    /// Checkpoint file path.
    #[must_use]
    pub fn checkpoint_path(&self) -> &Path {
        &self.checkpoint_path
    }
    /// Open `EventCore` around a journal database file.
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
            journal_path: journal_path.to_path_buf(),
            verification: Mutex::new(None),
            full_sweep: Mutex::new(None),
        };

        core.load_offsets();
        Ok(core)
    }

    /// The most exposing sensitivity anything in the Journal carries, or `None` if that could not
    /// be established.
    ///
    /// The distinction is the whole value of the answer. Reporting the least alarming number for a
    /// Journal that could not be read tells a surface deciding what it may publish that publishing
    /// is safe, on the strength of a question that failed. A caller that cannot learn what it
    /// would be disclosing has to refuse, and it can only do that if it is told.
    #[must_use]
    pub fn highest_sensitivity(&self) -> Option<u8> {
        self.writer
            .lock()
            .ok()
            .and_then(|writer| writer.highest_sensitivity().ok())
    }

    /// The verification established by the last incremental pass, if one has run.
    #[must_use]
    pub fn verification(&self) -> Option<VerificationState> {
        self.verification
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }

    /// Replay a bounded page of the hash chain, continuing from the persisted checkpoint.
    ///
    /// Bounded on purpose: verifying a Journal is linear in its length, and an unbounded pass
    /// would grow without limit against a Journal that only ever grows. Each pass advances the
    /// checkpoint, so repeated calls catch up and then track the tail.
    ///
    /// A break is not recorded as a checkpoint: the trusted position must stay where the chain was
    /// last actually intact.
    pub fn verify_page(&self, max_rows: u64, now: OffsetDateTime) -> Option<VerificationState> {
        let checkpoint = self.load_checkpoint();
        let head = self.count();
        let outcome =
            cybou_storage::verify_journal_page(&self.journal_path, checkpoint.as_ref(), max_rows);

        let state = match outcome {
            Ok(verification) => {
                self.save_checkpoint(&verification.checkpoint);
                VerificationState {
                    verified_through: verification.verified_through,
                    head,
                    broken_at: None,
                    content_verified: verification.content_verified,
                    content_skipped: verification.content_skipped,
                    taken_at: format_instant(now),
                }
            }
            Err(_) => VerificationState {
                verified_through: checkpoint.as_ref().map_or(0, |c| c.sequence),
                head,
                broken_at: Some(checkpoint.as_ref().map_or(0, |c| c.sequence) + 1),
                content_verified: 0,
                content_skipped: 0,
                taken_at: format_instant(now),
            },
        };

        if let Ok(mut guard) = self.verification.lock() {
            *guard = Some(state.clone());
        }
        Some(state)
    }

    /// Advance a full re-verification of the chain by one page, starting over once it completes.
    ///
    /// The incremental pass trusts a checkpoint and never looks behind it, so a row that rots after
    /// it was verified is never questioned again. This sweep exists for that: it re-reads the whole
    /// chain from the beginning, and because it costs the length of the biography it belongs to a
    /// moment when nobody is waiting.
    ///
    /// It never writes the trusted checkpoint. Its own position lives only in memory, so a sweep
    /// abandoned halfway leaves nothing behind and the next one starts from the beginning, which is
    /// the only position a full sweep can honestly start from.
    pub fn verify_fully_step(&self, max_rows: u64) -> FullSweepStep {
        let resume = self.full_sweep.lock().ok().and_then(|guard| guard.clone());
        let head = self.count();

        let Ok(verification) =
            cybou_storage::verify_journal_page(&self.journal_path, resume.as_ref(), max_rows)
        else {
            // A sweep that failed has nowhere honest to resume from, so it starts over. The
            // trusted checkpoint is untouched either way: this sweep never writes it.
            let resumed_from = resume.as_ref().map_or(0, |checkpoint| checkpoint.sequence);
            if let Ok(mut guard) = self.full_sweep.lock() {
                *guard = None;
            }
            return FullSweepStep {
                verified_through: resumed_from,
                head,
                has_more: false,
                broken_at: Some(resumed_from + 1),
            };
        };

        if let Ok(mut guard) = self.full_sweep.lock() {
            *guard = verification
                .has_more
                .then(|| verification.checkpoint.clone());
        }
        FullSweepStep {
            verified_through: verification.verified_through,
            head,
            has_more: verification.has_more,
            broken_at: None,
        }
    }

    fn load_checkpoint(&self) -> Option<JournalCheckpoint> {
        let mut file = File::open(&self.checkpoint_path).ok()?;
        let mut raw = String::new();
        file.read_to_string(&mut raw).ok()?;
        let persisted: PersistedCheckpoint = serde_json::from_str(&raw).ok()?;
        if persisted.version != 1 {
            return None;
        }
        Some(JournalCheckpoint {
            sequence: persisted.sequence,
            hash: decode_hex(&persisted.hash)?,
        })
    }

    fn save_checkpoint(&self, checkpoint: &JournalCheckpoint) -> bool {
        let persisted = PersistedCheckpoint {
            version: 1,
            sequence: checkpoint.sequence,
            hash: encode_hex(&checkpoint.hash),
        };
        let Ok(json) = serde_json::to_string(&persisted) else {
            return false;
        };
        let temp_path = self.checkpoint_path.with_extension("tmp");
        if let Ok(mut file) = File::create(&temp_path)
            && file.write_all(json.as_bytes()).is_ok()
            && file.sync_all().is_ok()
        {
            return fs::rename(&temp_path, &self.checkpoint_path).is_ok();
        }
        false
    }

    /// Attach a `KeyStore` to the underlying `JournalWriter` for sensitive payload sealing.
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
        let kind = Kind::from_u16(envelope.kind).ok_or_else(|| {
            EventError::Storage(WriteError::Malformed("unknown contribution kind"))
        })?;
        if kind.is_erasure() {
            return Err(EventError::ErasureRefused);
        }

        let mut writer = self
            .writer
            .lock()
            .map_err(|_| EventError::Storage(WriteError::Malformed("lock poisoned")))?;
        writer.append(envelope).map_err(EventError::Storage)
    }

    /// Forget a contribution and everything derived from it.
    ///
    /// ADR-0028's three steps, in the order that makes every crash recoverable:
    ///
    /// 1. an `ErasureRequested` contribution, committed before anything irreversible happens, so
    ///    a crash leaves a Journal that can say what it was in the middle of and why;
    /// 2. the keys are destroyed and the payloads redacted, which is idempotent and therefore safe
    ///    to repeat after a crash;
    /// 3. an `ErasureApplied` contribution, which is what makes the pair complete.
    ///
    /// A request with no matching applied record is an erasure that was interrupted. It is
    /// resumed from step 2 rather than reported as done, which is what [`Self::resume_erasures`]
    /// is for.
    ///
    /// # Errors
    ///
    /// Returns [`EventError`] when the request cannot be recorded, the redaction cannot commit, or
    /// the target is not a contribution this Journal holds.
    pub fn request_erasure(
        &self,
        target: &uuid::Uuid,
        reason: ErasureReason,
    ) -> Result<Erased, EventError> {
        if self.find_by_message_id(target).is_none() {
            return Err(EventError::Decode(
                "the Journal does not hold that contribution".into(),
            ));
        }

        let requested = self.record_erasure_step(Kind::ErasureRequested, target, reason, None)?;
        let outcome = self.carry_out_erasure(target)?;
        self.record_erasure_step(Kind::ErasureApplied, target, reason, Some(requested))?;
        Ok(outcome)
    }

    /// Finish any erasure that was interrupted before it recorded that it had happened.
    ///
    /// Called at startup. An organ that only noticed on the next explicit request would leave a
    /// person believing something was forgotten when the process died halfway through forgetting
    /// it — and the request is on record precisely so that nobody has to remember.
    ///
    /// # Errors
    ///
    /// Returns [`EventError`] when the Journal cannot be read or a resumed erasure cannot finish.
    pub fn resume_erasures(&self) -> Result<usize, EventError> {
        let mut resumed = 0;
        for (target, reason, request_id) in self.unfinished_erasures() {
            self.carry_out_erasure(&target)?;
            self.record_erasure_step(Kind::ErasureApplied, &target, reason, Some(request_id))?;
            resumed += 1;
        }
        Ok(resumed)
    }

    /// Erasure requests with no applied record beside them.
    fn unfinished_erasures(&self) -> Vec<(uuid::Uuid, ErasureReason, uuid::Uuid)> {
        let mut requested = Vec::new();
        let mut applied = Vec::new();
        let mut after = 0_u64;
        loop {
            let page = self.replay(after, 512);
            if page.is_empty() {
                break;
            }
            for envelope in &page {
                match Kind::from_u16(envelope.kind) {
                    Some(Kind::ErasureRequested) => {
                        if let Some(record) = decode_erasure_record(envelope) {
                            requested.push((envelope.message_id, record));
                        }
                    }
                    Some(Kind::ErasureApplied) => applied.push(envelope.causation_id),
                    _ => {}
                }
            }
            after += page.len() as u64;
        }
        requested
            .into_iter()
            .filter(|(request_id, _)| !applied.contains(request_id))
            .map(|(request_id, (target, reason))| (target, reason, request_id))
            .collect()
    }

    /// Steps 2 and 3 of the sequence, which the writer performs as one operation.
    fn carry_out_erasure(&self, target: &uuid::Uuid) -> Result<Erased, EventError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| EventError::Storage(WriteError::Malformed("lock poisoned")))?;
        writer.apply_erasure(target).map_err(EventError::Storage)
    }

    /// Append one of the two erasure records.
    ///
    /// These do not go through `submit`, which refuses erasure kinds by design: destroying
    /// biography must never be reachable by the call that records a thought about it.
    fn record_erasure_step(
        &self,
        kind: Kind,
        target: &uuid::Uuid,
        reason: ErasureReason,
        caused_by: Option<uuid::Uuid>,
    ) -> Result<uuid::Uuid, EventError> {
        // An erasure record cites what it is about, so it inherits that contribution's classes:
        // the Journal refuses a derived contribution that is less restricted than its references,
        // and it is right to. That a person asked for something personal to be forgotten is itself
        // about the person, even though the record says nothing about what it was.
        let (privacy, sensitivity) = self
            .find_by_message_id(target)
            .map_or((1, 1), |envelope| (envelope.privacy, envelope.sensitivity));

        let mut payload = Vec::new();
        // The target and the reason, and nothing else. Whatever the payload said is exactly what
        // this record must not repeat.
        let record = ciborium::Value::Map(vec![
            (
                ciborium::Value::Text("target".into()),
                ciborium::Value::Text(target.to_string()),
            ),
            (
                ciborium::Value::Text("reason".into()),
                ciborium::Value::Text(reason.name().into()),
            ),
        ]);
        ciborium::into_writer(&record, &mut payload)
            .map_err(|error| EventError::Decode(error.to_string()))?;

        let envelope = CanonicalEnvelope {
            schema_version: 4,
            message_id: uuid::Uuid::new_v4(),
            correlation_id: *target,
            causation_id: caused_by.unwrap_or_else(uuid::Uuid::nil),
            origin_organ: "eventd".to_owned(),
            origin_node: String::new(),
            kind: kind as u16,
            wall_time_ms: cybou_protocol::unix_millis(time::OffsetDateTime::now_utc()),
            monotonic_time: 0,
            logical_clock: 1,
            confidence: 1.0,
            // The request cites what it is about. `ErasureRequested` is a derived kind and the
            // Journal refuses one that references nothing — rightly, because a request to forget
            // that named nothing would be unaccountable afterwards. The applied record cites the
            // request instead, through causation, which is what pairs the two.
            evidence: if caused_by.is_some() {
                Vec::new()
            } else {
                vec![*target]
            },
            payload,
            privacy,
            capability_scope: String::new(),
            sealed: false,
            key_domain_id: uuid::Uuid::nil(),
            key_epoch: 0,
            // An erasure record is never itself erasable, so it is not given a retention that
            // could expire it: a forgetting that could be forgotten would make the audit trail a
            // suggestion.
            retention_class: 0,
            retention_policy_version: 0,
            retain_until_ms: 0,
            sensitivity,
        };

        let mut writer = self
            .writer
            .lock()
            .map_err(|_| EventError::Storage(WriteError::Malformed("lock poisoned")))?;
        writer.append(&envelope).map_err(EventError::Storage)?;
        Ok(envelope.message_id)
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
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.head().ok())
            .flatten()
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
        let Ok(mut offsets) = self.offsets.lock() else {
            return false;
        };
        offsets
            .entry(consumer_id.to_string())
            .or_insert(initial_offset);
        self.save_offsets(&offsets)
    }

    /// Advance consumer offset.
    pub fn advance_consumer(&self, consumer_id: &str, offset: u64) -> bool {
        if !is_valid_consumer_id(consumer_id) {
            return false;
        }
        let Ok(mut offsets) = self.offsets.lock() else {
            return false;
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
        if let Ok(persisted) = serde_json::from_str::<PersistedOffsets>(&content)
            && persisted.version == 1
        {
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
        if let Ok(mut file) = File::create(&temp_path)
            && file.write_all(json.as_bytes()).is_ok()
            && file.flush().is_ok()
        {
            return fs::rename(&temp_path, &self.offsets_path).is_ok();
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

fn format_instant(now: OffsetDateTime) -> String {
    now.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(text.get(index..index + 2)?, 16).ok())
        .collect()
}

/// The target and reason an erasure record carries.
///
/// Returns `None` for a record this build cannot read, which is treated as an erasure nobody can
/// account for rather than one to guess at.
fn decode_erasure_record(envelope: &CanonicalEnvelope) -> Option<(uuid::Uuid, ErasureReason)> {
    let value: ciborium::Value = ciborium::from_reader(envelope.payload.as_slice()).ok()?;
    let map = value.as_map()?;
    let field = |name: &str| {
        map.iter()
            .find(|(key, _)| key.as_text() == Some(name))
            .and_then(|(_, value)| value.as_text())
    };
    let target = uuid::Uuid::parse_str(field("target")?).ok()?;
    let reason = ErasureReason::from_name(field("reason")?)?;
    Some((target, reason))
}

#[cfg(test)]
mod tests {
    /// A contribution caused by another, so the closure has a causation edge to travel.
    fn caused_by(cause: &CanonicalEnvelope, kind: Kind, text: &str) -> CanonicalEnvelope {
        let mut envelope = observation(text);
        envelope.kind = kind as u16;
        envelope.causation_id = cause.message_id;
        envelope
    }

    /// A contribution citing another as evidence, which is the other edge the closure travels.
    ///
    /// Its cause is something else entirely: evidence may not restate the cause, and this is the
    /// case that proves the closure does not only follow causation.
    fn citing(
        evidence: &CanonicalEnvelope,
        cause: &CanonicalEnvelope,
        kind: Kind,
        text: &str,
    ) -> CanonicalEnvelope {
        let mut envelope = observation(text);
        envelope.kind = kind as u16;
        envelope.causation_id = cause.message_id;
        envelope.evidence = vec![evidence.message_id];
        envelope
    }

    fn observation(text: &str) -> CanonicalEnvelope {
        let observation = cybou_protocol::observation::ObservationV1 {
            source_id: "test".into(),
            subject: "a-subject".into(),
            value: ciborium::Value::Text(text.into()),
            acquired_at: "2026-08-21T00:00:00.000Z".into(),
            freshness_until: "2026-08-22T00:00:00.000Z".into(),
            provenance: "a fixture".into(),
        };
        let mut payload = Vec::new();
        ciborium::into_writer(&observation, &mut payload).expect("encode");
        CanonicalEnvelope {
            schema_version: 4,
            message_id: Uuid::new_v4(),
            correlation_id: Uuid::new_v4(),
            causation_id: Uuid::nil(),
            origin_organ: "testd".into(),
            origin_node: String::new(),
            kind: Kind::Observation as u16,
            wall_time_ms: 0,
            monotonic_time: 0,
            logical_clock: 1,
            confidence: 1.0,
            evidence: Vec::new(),
            payload,
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

    fn core_in(dir: &std::path::Path) -> EventCore {
        EventCore::open(dir.join("journal.sqlite3")).expect("a journal")
    }

    #[test]
    fn forgetting_something_forgets_what_was_derived_from_it() {
        // ADR-0028 E7: erasing a diagnosis and keeping the reasoning that restates it would have
        // destroyed the record a person asked to forget and kept the sentence that repeats it.
        let dir = tempfile::tempdir().expect("temp dir");
        let core = core_in(dir.path());

        let root = observation("the thing to forget");
        core.submit(&root, None).expect("root accepted");
        let derived = caused_by(&root, Kind::Hypothesis, "because of the thing to forget");
        core.submit(&derived, None).expect("derived accepted");
        let unrelated = observation("nothing to do with it");
        core.submit(&unrelated, None).expect("unrelated accepted");
        let citing_it = citing(
            &root,
            &unrelated,
            Kind::Learning,
            "learned from the thing to forget",
        );
        core.submit(&citing_it, None).expect("citing accepted");

        let outcome = core
            .request_erasure(&root.message_id, ErasureReason::UserRequested)
            .expect("the erasure runs");

        assert!(outcome.closure.contains(&root.message_id));
        assert!(
            outcome.closure.contains(&derived.message_id),
            "a contribution caused by the target is part of what must be forgotten"
        );
        assert!(
            outcome.closure.contains(&citing_it.message_id),
            "a contribution citing the target as evidence is a dependent too"
        );
        assert!(
            !outcome.closure.contains(&unrelated.message_id),
            "something that merely happened afterwards is not a descendant"
        );

        // The payloads are gone and the rows are not.
        for id in [root.message_id, derived.message_id, citing_it.message_id] {
            let envelope = core.find_by_message_id(&id).expect("the row survives");
            assert!(envelope.payload.is_empty(), "the payload must be redacted");
            assert_eq!(envelope.origin_organ, "testd", "provenance must survive");
        }
        let untouched = core
            .find_by_message_id(&unrelated.message_id)
            .expect("the row survives");
        assert!(!untouched.payload.is_empty());
    }

    #[test]
    fn an_erasure_is_recorded_at_both_ends_and_says_why_without_saying_what() {
        let dir = tempfile::tempdir().expect("temp dir");
        let core = core_in(dir.path());
        let root = observation("something private");
        core.submit(&root, None).expect("accepted");

        let before = core.erasure_epoch();
        core.request_erasure(&root.message_id, ErasureReason::ConsentWithdrawn)
            .expect("the erasure runs");
        assert!(core.erasure_epoch() > before, "the epoch has to advance");

        let all = core.replay(0, 128);
        let requested: Vec<_> = all
            .iter()
            .filter(|e| Kind::from_u16(e.kind) == Some(Kind::ErasureRequested))
            .collect();
        let applied: Vec<_> = all
            .iter()
            .filter(|e| Kind::from_u16(e.kind) == Some(Kind::ErasureApplied))
            .collect();
        assert_eq!(requested.len(), 1);
        assert_eq!(applied.len(), 1);
        assert_eq!(
            applied[0].causation_id, requested[0].message_id,
            "the applied record has to name the request it completes"
        );

        // The reason is on the record and the erased content is not.
        let (target, reason) = decode_erasure_record(requested[0]).expect("a readable record");
        assert_eq!(target, root.message_id);
        assert_eq!(reason, ErasureReason::ConsentWithdrawn);
        let text = String::from_utf8_lossy(&requested[0].payload).to_string();
        assert!(
            !text.contains("something private"),
            "an erasure record must never restate what it erased"
        );
    }

    #[test]
    fn an_erasure_record_cannot_itself_be_erased() {
        // A forgetting that could be forgotten would make the audit trail a suggestion.
        let dir = tempfile::tempdir().expect("temp dir");
        let core = core_in(dir.path());
        let root = observation("something private");
        core.submit(&root, None).expect("accepted");
        core.request_erasure(&root.message_id, ErasureReason::UserRequested)
            .expect("the first erasure runs");

        let record = core
            .replay(0, 128)
            .into_iter()
            .find(|e| Kind::from_u16(e.kind) == Some(Kind::ErasureRequested))
            .expect("an erasure record");

        core.request_erasure(&record.message_id, ErasureReason::UserRequested)
            .expect("the request is accepted");
        let after = core
            .find_by_message_id(&record.message_id)
            .expect("the record survives");
        assert!(
            !after.payload.is_empty(),
            "an erasure record keeps its payload however often it is targeted"
        );
    }

    #[test]
    fn an_erasure_interrupted_before_it_finished_is_finished_on_the_next_start() {
        // ADR-0028 E4: a request with no applied record is resumed rather than reported as done.
        let dir = tempfile::tempdir().expect("temp dir");
        let core = core_in(dir.path());
        let root = observation("something private");
        core.submit(&root, None).expect("accepted");

        // Exactly what a crash between step 1 and step 3 leaves behind: the request is durable and
        // nothing else happened.
        core.record_erasure_step(
            Kind::ErasureRequested,
            &root.message_id,
            ErasureReason::UserRequested,
            None,
        )
        .expect("the request is recorded");
        assert!(
            !core
                .find_by_message_id(&root.message_id)
                .expect("the row")
                .payload
                .is_empty(),
            "nothing has been erased yet"
        );

        assert_eq!(core.resume_erasures().expect("resumption runs"), 1);
        assert!(
            core.find_by_message_id(&root.message_id)
                .expect("the row")
                .payload
                .is_empty(),
            "the interrupted erasure has to complete"
        );

        // And a second start finds nothing left to resume, because the pair is now complete.
        assert_eq!(core.resume_erasures().expect("resumption runs"), 0);
    }

    #[test]
    fn erasing_something_the_journal_does_not_hold_is_refused() {
        let dir = tempfile::tempdir().expect("temp dir");
        let core = core_in(dir.path());
        assert!(
            core.request_erasure(&Uuid::new_v4(), ErasureReason::UserRequested)
                .is_err(),
            "there is nothing to forget, and saying otherwise would be a false record"
        );
    }

    #[test]
    fn a_full_sweep_never_moves_the_trusted_checkpoint() {
        let dir = tempfile::tempdir().expect("temp dir");
        let db_path = dir.path().join("journal.sqlite3");
        let core = super::EventCore::open(&db_path).expect("open event core");

        // No trusted checkpoint exists yet, and a full sweep must not create one: its position is
        // its own, so an interrupted sweep can never be mistaken for verified history.
        let checkpoint_path = core.checkpoint_path().to_path_buf();
        let step = core.verify_fully_step(8);
        assert!(step.broken_at.is_none());
        assert!(
            !checkpoint_path.exists(),
            "a full sweep must not write the checkpoint the incremental pass trusts"
        );

        // The incremental pass is what establishes trust, and it still does.
        let _ = core.verify_page(8, time::OffsetDateTime::now_utc());
        assert!(checkpoint_path.exists());
    }

    #[test]
    fn hex_round_trips_the_checkpoint_hash() {
        let hash = vec![0x00, 0x0f, 0xa5, 0xff];
        let text = super::encode_hex(&hash);
        assert_eq!(text, "000fa5ff");
        assert_eq!(super::decode_hex(&text), Some(hash));
        assert_eq!(super::decode_hex("abc"), None);
        assert_eq!(super::decode_hex("zz"), None);
    }

    use cybou_protocol::unix_millis;
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
            wall_time_ms: unix_millis(OffsetDateTime::now_utc()),
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

        let found = core
            .find_by_message_id(&env1.message_id)
            .expect("found by id");
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

    #[test]
    fn self_assessment_and_learning_are_permitted_but_erasure_is_refused() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("journal.sqlite3");
        let core = EventCore::open(&db_path).expect("open event core");

        // Submit root observation first
        let root_obs = dummy_envelope(Uuid::new_v4(), "unreserved");
        let root_res = core.submit(&root_obs, None).expect("root observation");
        assert_eq!(root_res.sequence, 1);

        // Kind 13 (SelfAssessment) is permitted when citing evidence
        let mut env13 = dummy_envelope(Uuid::new_v4(), "selfd");
        env13.kind = Kind::SelfAssessment as u16;
        env13.evidence = vec![root_obs.message_id];
        let res13 = core
            .submit(&env13, Some("selfd"))
            .expect("SelfAssessment permitted");
        assert_eq!(res13.sequence, 2);

        // Kind 14 (Learning) is permitted (derived from SelfAssessment)
        let mut env14 = dummy_envelope(Uuid::new_v4(), "learning_organ");
        env14.kind = Kind::Learning as u16;
        env14.causation_id = env13.message_id;
        let res14 = core.submit(&env14, None).expect("Learning permitted");
        assert_eq!(res14.sequence, 3);

        // Kind 15 (ErasureRequested) is refused on normal Submit
        let mut env15 = dummy_envelope(Uuid::new_v4(), "admin");
        env15.kind = Kind::ErasureRequested as u16;
        let err15 = core
            .submit(&env15, None)
            .expect_err("ErasureRequested must be refused");
        assert!(matches!(err15, EventError::ErasureRefused));

        // Kind 16 (ErasureApplied) is refused on normal Submit
        let mut env16 = dummy_envelope(Uuid::new_v4(), "admin");
        env16.kind = Kind::ErasureApplied as u16;
        let err16 = core
            .submit(&env16, None)
            .expect_err("ErasureApplied must be refused");
        assert!(matches!(err16, EventError::ErasureRefused));
    }
}
