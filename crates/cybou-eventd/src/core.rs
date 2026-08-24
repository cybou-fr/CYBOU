// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Pure core domain logic for Event1 service, wrapping `JournalWriter`, `KeyStore`, and offsets.

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
use time::OffsetDateTime;
use uuid::Uuid;

use crate::erasure::decode_erasure_record;
use crate::error::{EventError, is_reserved_organ};
use crate::offsets::{PersistedOffsets, is_valid_consumer_id};
use crate::verification::{
    FullSweepStep, PersistedCheckpoint, VerificationState, decode_hex, encode_hex, format_instant,
};
use cybou_protocol::admission::BackupState;

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

    /// The most exposing sensitivity anything in the Journal carries, or `None` if that could not be established.
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
    pub fn verify_fully_step(&self, max_rows: u64) -> FullSweepStep {
        let resume = self.full_sweep.lock().ok().and_then(|guard| guard.clone());
        let head = self.count();

        let Ok(verification) =
            cybou_storage::verify_journal_page(&self.journal_path, resume.as_ref(), max_rows)
        else {
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
        if let Some(caller) = caller_organ {
            if envelope.origin_organ != caller {
                return Err(EventError::OriginUnauthentic(envelope.origin_organ.clone()));
            }
        } else if is_reserved_organ(&envelope.origin_organ) {
            return Err(EventError::OriginUnauthentic(envelope.origin_organ.clone()));
        }

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
    /// # Errors
    ///
    /// Returns [`EventError`] when the request cannot be recorded, the redaction cannot commit, or
    /// the target is not a contribution this Journal holds.
    pub fn request_erasure(
        &self,
        target: &Uuid,
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

    fn unfinished_erasures(&self) -> Vec<(Uuid, ErasureReason, Uuid)> {
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
            #[allow(clippy::cast_possible_truncation)]
            {
                after += page.len() as u64;
            }
        }
        requested
            .into_iter()
            .filter(|(request_id, _)| !applied.contains(request_id))
            .map(|(request_id, (target, reason))| (target, reason, request_id))
            .collect()
    }

    fn carry_out_erasure(&self, target: &Uuid) -> Result<Erased, EventError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| EventError::Storage(WriteError::Malformed("lock poisoned")))?;
        writer.apply_erasure(target).map_err(EventError::Storage)
    }

    /// What this deployment declared about the backups it keeps.
    ///
    /// `CYBOU_BACKUP_ROTATION_DAYS=0` means it keeps none. A positive number is how long a copy
    /// stays in rotation. Unset means nothing was declared, which is not the same as none: silence
    /// about backups is not evidence that none exist, and an erasure reporting completeness on the
    /// strength of nobody having mentioned a copy would be stating what nobody established.
    fn declared_backup_rotation() -> Option<u32> {
        std::env::var("CYBOU_BACKUP_ROTATION_DAYS")
            .ok()
            .and_then(|value| value.trim().parse().ok())
    }

    pub(crate) fn record_erasure_step(
        &self,
        kind: Kind,
        target: &Uuid,
        reason: ErasureReason,
        caused_by: Option<Uuid>,
    ) -> Result<Uuid, EventError> {
        let (privacy, sensitivity) = self
            .find_by_message_id(target)
            .map_or((1, 1), |envelope| (envelope.privacy, envelope.sensitivity));

        let now_ms = cybou_protocol::unix_millis(OffsetDateTime::now_utc());
        let mut record = vec![
            (
                ciborium::Value::Text("target".into()),
                ciborium::Value::Text(target.to_string()),
            ),
            (
                ciborium::Value::Text("reason".into()),
                ciborium::Value::Text(reason.name().into()),
            ),
        ];

        // Only the terminal step carries a completion state. A request that has not been carried
        // out has achieved nothing yet, and saying anything about backups there would be a claim
        // about work not done.
        if kind == Kind::ErasureApplied {
            let backups =
                BackupState::from_rotation(Self::declared_backup_rotation(), now_ms, now_ms);
            record.push((
                ciborium::Value::Text("backupState".into()),
                ciborium::Value::Text(backups.name().into()),
            ));
            if let BackupState::PendingRotation { complete_after_ms } = backups {
                record.push((
                    ciborium::Value::Text("backupsCompleteAfterMs".into()),
                    ciborium::Value::Integer(complete_after_ms.into()),
                ));
            }
        }

        let mut payload = Vec::new();
        let record = ciborium::Value::Map(record);
        ciborium::into_writer(&record, &mut payload)
            .map_err(|error| EventError::Decode(error.to_string()))?;

        let envelope = CanonicalEnvelope {
            schema_version: 4,
            message_id: Uuid::new_v4(),
            correlation_id: *target,
            causation_id: caused_by.unwrap_or_else(Uuid::nil),
            origin_organ: "eventd".to_owned(),
            origin_node: String::new(),
            kind: kind as u16,
            wall_time_ms: cybou_protocol::unix_millis(OffsetDateTime::now_utc()),
            monotonic_time: 0,
            logical_clock: 1,
            confidence: 1.0,
            evidence: if caused_by.is_some() {
                Vec::new()
            } else {
                vec![*target]
            },
            payload,
            privacy,
            capability_scope: String::new(),
            sealed: false,
            key_domain_id: Uuid::nil(),
            key_epoch: 0,
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
    #[must_use]
    pub fn count(&self) -> u64 {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.count().ok())
            .unwrap_or(0)
    }

    /// Write a consistent copy of the Journal to `target`.
    ///
    /// Offered by the only writer, because it is the only party that can produce one. A backup
    /// script copying `journal.sqlite3` from outside gets a file that opens cleanly and is missing
    /// whatever is still in the write-ahead log — a backup that restores and looks right.
    ///
    /// The copy holds ciphertext and no keys. That is what makes it a copy an erasure still
    /// reaches: destroying a data key makes the record unreadable here too, provided whoever keeps
    /// this file did not also keep the key store, which now lives in a different directory.
    ///
    /// This takes a snapshot. It does not schedule one, keep a rotation, or remove anything —
    /// deciding what to delete and when is an operator's policy, and `BackupState` exists so a
    /// deployment can declare theirs rather than have one assumed for them.
    ///
    /// # Errors
    ///
    /// Returns [`EventError::Storage`] if anything already sits at `target` or the copy fails.
    pub fn snapshot_into(&self, target: &std::path::Path) -> Result<(), EventError> {
        let writer = self
            .writer
            .lock()
            .map_err(|_| EventError::Storage(WriteError::Malformed("lock poisoned")))?;
        writer.snapshot_into(target).map_err(EventError::Storage)
    }

    /// Return head envelope, if any.
    #[must_use]
    pub fn head(&self) -> Option<CanonicalEnvelope> {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.head().ok())
            .flatten()
    }

    /// Retrieve an envelope at a specific sequence.
    #[must_use]
    pub fn at_sequence(&self, sequence: u64) -> Option<CanonicalEnvelope> {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.at_sequence(sequence).ok())
            .flatten()
    }

    /// Replay contributions strictly after `after_sequence` up to `limit`.
    #[must_use]
    pub fn replay(&self, after_sequence: u64, limit: usize) -> Vec<CanonicalEnvelope> {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.replay(after_sequence, limit).ok())
            .unwrap_or_default()
    }

    /// Return recent contributions up to `limit`.
    #[must_use]
    pub fn recent(&self, limit: usize) -> Vec<CanonicalEnvelope> {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.recent(limit).ok())
            .unwrap_or_default()
    }

    /// Find an envelope by message ID.
    #[must_use]
    pub fn find_by_message_id(&self, message_id: &Uuid) -> Option<CanonicalEnvelope> {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.find_by_message_id(message_id).ok())
            .flatten()
    }

    /// Find all envelopes in an episode by correlation ID.
    #[must_use]
    pub fn find_by_correlation_id(&self, correlation_id: &Uuid) -> Vec<CanonicalEnvelope> {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.find_by_correlation_id(correlation_id).ok())
            .unwrap_or_default()
    }

    /// Check whether a terminal outcome exists for a cause and organ.
    #[must_use]
    pub fn has_outcome_for(&self, cause_id: &Uuid, origin_organ: &str) -> bool {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.has_outcome_for(cause_id, origin_organ).ok())
            .unwrap_or(false)
    }

    /// Evidence UUIDs for a contribution message ID.
    #[must_use]
    pub fn evidence_for(&self, message_id: &Uuid) -> Vec<Uuid> {
        self.writer
            .lock()
            .ok()
            .and_then(|w| w.evidence_for(message_id).ok())
            .unwrap_or_default()
    }

    /// Current erasure epoch.
    #[must_use]
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
    #[must_use]
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
