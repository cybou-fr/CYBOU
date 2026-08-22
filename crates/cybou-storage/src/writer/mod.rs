// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The Rust Journal writer, in the two slices that do not need a key store.

pub mod append;
pub mod error;
pub mod schema;

use cybou_protocol::admission::Kind;
use cybou_protocol::canonical::CanonicalEnvelope;
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;
use time::OffsetDateTime;
use uuid::Uuid;

pub use append::{Appended, Erased, qt_instant};
pub use error::{JOURNAL_HASH_V3, WriteError, write_error};
pub use schema::{EVIDENCE_TABLE_DDL, migrate_v1_to_v2, v2_indexes_ddl};

use crate::JOURNAL_SCHEMA_V2;
use append::append_within_transaction;
use schema::{ensure_durability, open_for_write};

/// A read-write Journal connection that can append contributions.
#[derive(Debug)]
pub struct JournalWriter {
    connection: Connection,
    key_store: Option<cybou_crypto::KeyStore>,
    kek: Option<[u8; 32]>,
    key_domain: Option<cybou_crypto::KeyDomain>,
}

impl JournalWriter {
    /// Open an existing Journal for writing, or create schema v2 when the file is new.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] when the file cannot be opened, the durability pragmas did not take,
    /// the schema is one this build may not write to, or schema creation fails.
    pub fn open(path: &Path) -> Result<Self, WriteError> {
        let connection = open_for_write(path)?;
        ensure_durability(&connection)?;
        let writer = Self {
            connection,
            key_store: None,
            kek: None,
            key_domain: None,
        };
        writer.ensure_schema()?;
        Ok(writer)
    }

    /// Attach an active `KeyStore`, key encryption key (KEK), and key domain for sealing sensitive contributions.
    pub fn set_key_store(
        &mut self,
        key_store: cybou_crypto::KeyStore,
        kek: [u8; 32],
        key_domain: cybou_crypto::KeyDomain,
    ) {
        self.key_store = Some(key_store);
        self.kek = Some(kek);
        self.key_domain = Some(key_domain);
    }

    fn user_version(&self) -> Result<i64, WriteError> {
        self.connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(WriteError::Query)
    }

    fn has_contribution_table(&self) -> Result<bool, WriteError> {
        self.connection
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='contribution' LIMIT 1",
                [],
                |_| Ok(()),
            )
            .optional()
            .map(|found| found.is_some())
            .map_err(WriteError::Query)
    }

    fn ensure_schema(&self) -> Result<(), WriteError> {
        let version = self.user_version()?;
        let has_table = self.has_contribution_table()?;

        if !has_table {
            if version != 0 {
                return Err(WriteError::InconsistentSchema(version));
            }
            return self.create_schema_v2();
        }

        if version == JOURNAL_SCHEMA_V2 {
            return Ok(());
        }
        Err(WriteError::UnsupportedSchema {
            received: version,
            expected: JOURNAL_SCHEMA_V2,
        })
    }

    fn create_schema_v2(&self) -> Result<(), WriteError> {
        self.connection
            .execute_batch(&format!(
                "BEGIN IMMEDIATE;
                 CREATE TABLE contribution (
                     seq            INTEGER PRIMARY KEY AUTOINCREMENT,
                     message_id     TEXT    NOT NULL UNIQUE,
                     correlation_id TEXT    NOT NULL,
                     causation_id   TEXT,
                     origin_organ   TEXT    NOT NULL,
                     origin_node    TEXT    NOT NULL DEFAULT '',
                     kind           INTEGER NOT NULL,
                     wall_time      TEXT    NOT NULL,
                     monotonic_time INTEGER NOT NULL,
                     logical_clock  INTEGER NOT NULL,
                     confidence     REAL    NOT NULL,
                     evidence       TEXT,
                     payload        BLOB,
                     privacy        INTEGER NOT NULL,
                     capability     TEXT,
                     schema_version INTEGER NOT NULL,
                     hash_version   INTEGER NOT NULL,
                     prev_hash      BLOB,
                     hash           BLOB    NOT NULL,
                     commitment     BLOB,
                     payload_commitment BLOB,
                     erased_at      TEXT,
                     sealed         INTEGER NOT NULL DEFAULT 0,
                     key_domain     TEXT,
                     key_epoch      INTEGER NOT NULL DEFAULT 0,
                     retention_class INTEGER NOT NULL DEFAULT 2,
                     retention_policy INTEGER NOT NULL DEFAULT 0,
                     retain_until   TEXT,
                     sensitivity    INTEGER NOT NULL DEFAULT 1
                 );
                 CREATE TABLE journal_meta (
                     id             INTEGER PRIMARY KEY CHECK (id = 1),
                     erasure_epoch  INTEGER NOT NULL DEFAULT 0,
                     rotated_epoch  INTEGER NOT NULL DEFAULT 0
                 );
                 INSERT OR IGNORE INTO journal_meta (id) VALUES (1);
                 {evidence}
                 {indexes}
                 PRAGMA user_version = {version};
                 COMMIT;",
                evidence = EVIDENCE_TABLE_DDL,
                indexes = v2_indexes_ddl(),
                version = JOURNAL_SCHEMA_V2,
            ))
            .map_err(write_error)
    }

    /// Append one contribution, or refuse it.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError::Refused`] when a rule declines the contribution, leaving the Journal
    /// exactly as it was, or another [`WriteError`] when the database itself failed.
    pub fn append(&mut self, envelope: &CanonicalEnvelope) -> Result<Appended, WriteError> {
        let mut key_to_clean = None;

        let envelope_to_write;
        let target_envelope = if envelope.sealed {
            let (Some(store), Some(kek), Some(domain)) =
                (&self.key_store, &self.kek, &self.key_domain)
            else {
                return Err(WriteError::SealedWithoutKeyStore);
            };
            let data_key = store.create_key_for(&envelope.message_id, kek)?;
            key_to_clean = Some(envelope.message_id);
            let sealed = cybou_crypto::Seal::seal(&envelope.payload, &data_key)?;
            let mut stored = envelope.clone();
            let mut payload_bytes =
                Vec::with_capacity(sealed.nonce.len() + sealed.ciphertext.len());
            payload_bytes.extend_from_slice(&sealed.nonce);
            payload_bytes.extend_from_slice(&sealed.ciphertext);
            stored.payload = payload_bytes;
            stored.key_domain_id = domain.key_domain_id;
            stored.key_epoch = domain.key_epoch;
            envelope_to_write = stored;
            &envelope_to_write
        } else {
            envelope
        };

        let result = self.append_committed(target_envelope);
        if result.is_err()
            && let Some(contribution_id) = key_to_clean
            && let Some(store) = &self.key_store
        {
            let _ = store.destroy_key_for(&contribution_id);
        }
        result
    }

    fn append_committed(&mut self, envelope: &CanonicalEnvelope) -> Result<Appended, WriteError> {
        let transaction = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(write_error)?;
        let appended = append_within_transaction(&transaction, envelope)?;
        transaction.commit().map_err(write_error)?;
        Ok(appended)
    }

    /// Return the most exposing sensitivity anything in the Journal carries.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] on database query failure.
    pub fn highest_sensitivity(&self) -> Result<u8, WriteError> {
        let highest: Option<i64> = self
            .connection
            .query_row("SELECT MAX(sensitivity) FROM contribution", [], |row| {
                row.get(0)
            })
            .map_err(WriteError::Query)?;
        Ok(u8::try_from(highest.unwrap_or(0)).unwrap_or(u8::MAX))
    }

    /// Return the count of contributions stored in the Journal.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] on database query failure.
    pub fn count(&self) -> Result<u64, WriteError> {
        let count: i64 = self
            .connection
            .query_row("SELECT COUNT(*) FROM contribution", [], |row| row.get(0))
            .map_err(WriteError::Query)?;
        u64::try_from(count).map_err(|_| WriteError::Malformed("negative count"))
    }

    /// Return the latest (head) envelope, or `None` if the Journal is empty.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] on database query failure or decoding error.
    pub fn head(&self) -> Result<Option<CanonicalEnvelope>, WriteError> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT seq, prev_hash, schema_version, message_id, correlation_id, causation_id, \
                 origin_organ, origin_node, kind, wall_time, monotonic_time, logical_clock, \
                 confidence, payload, privacy, capability, sealed, key_domain, key_epoch, \
                 retention_class, retention_policy, retain_until, sensitivity \
                 FROM contribution ORDER BY seq DESC LIMIT 1",
            )
            .map_err(WriteError::Query)?;
        let mut rows = stmt.query([]).map_err(WriteError::Query)?;
        if let Some(row) = rows.next().map_err(WriteError::Query)? {
            let seq: i64 = row.get(0).map_err(WriteError::Query)?;
            let envelope =
                crate::decode_envelope(&self.connection, row, u64::try_from(seq).unwrap_or(0))
                    .map_err(WriteError::from)?;
            Ok(Some(envelope))
        } else {
            Ok(None)
        }
    }

    /// Retrieve the envelope at a specific sequence number.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] on database query failure or decoding error.
    pub fn at_sequence(&self, sequence: u64) -> Result<Option<CanonicalEnvelope>, WriteError> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT seq, prev_hash, schema_version, message_id, correlation_id, causation_id, \
                 origin_organ, origin_node, kind, wall_time, monotonic_time, logical_clock, \
                 confidence, payload, privacy, capability, sealed, key_domain, key_epoch, \
                 retention_class, retention_policy, retain_until, sensitivity \
                 FROM contribution WHERE seq=?1 LIMIT 1",
            )
            .map_err(WriteError::Query)?;
        let seq_i64 =
            i64::try_from(sequence).map_err(|_| WriteError::Malformed("sequence overflow"))?;
        let mut rows = stmt.query([seq_i64]).map_err(WriteError::Query)?;
        if let Some(row) = rows.next().map_err(WriteError::Query)? {
            let envelope = crate::decode_envelope(&self.connection, row, sequence)
                .map_err(WriteError::from)?;
            Ok(Some(envelope))
        } else {
            Ok(None)
        }
    }

    /// Retrieve a list of recent contributions, ordered oldest to newest.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] on database query failure or decoding error.
    pub fn recent(&self, limit: usize) -> Result<Vec<CanonicalEnvelope>, WriteError> {
        let limit_clause = if limit > 0 {
            format!("ORDER BY seq DESC LIMIT {limit}")
        } else {
            "ORDER BY seq ASC".to_string()
        };
        let query = format!(
            "SELECT seq, prev_hash, schema_version, message_id, correlation_id, causation_id, \
             origin_organ, origin_node, kind, wall_time, monotonic_time, logical_clock, \
             confidence, payload, privacy, capability, sealed, key_domain, key_epoch, \
             retention_class, retention_policy, retain_until, sensitivity \
             FROM contribution {limit_clause}"
        );
        let mut stmt = self.connection.prepare(&query).map_err(WriteError::Query)?;
        let mut rows = stmt.query([]).map_err(WriteError::Query)?;
        let mut result = Vec::new();
        while let Some(row) = rows.next().map_err(WriteError::Query)? {
            let seq: i64 = row.get(0).map_err(WriteError::Query)?;
            let envelope =
                crate::decode_envelope(&self.connection, row, u64::try_from(seq).unwrap_or(0))
                    .map_err(WriteError::from)?;
            result.push(envelope);
        }
        if limit > 0 {
            result.reverse();
        }
        Ok(result)
    }

    /// Paged replay of contributions strictly after `after_sequence`.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] on database query failure or decoding error.
    pub fn replay(
        &self,
        after_sequence: u64,
        limit: usize,
    ) -> Result<Vec<CanonicalEnvelope>, WriteError> {
        let after_i64 = i64::try_from(after_sequence)
            .map_err(|_| WriteError::Malformed("sequence overflow"))?;
        let limit_clause = if limit > 0 {
            format!("LIMIT {limit}")
        } else {
            String::new()
        };
        let query = format!(
            "SELECT seq, prev_hash, schema_version, message_id, correlation_id, causation_id, \
             origin_organ, origin_node, kind, wall_time, monotonic_time, logical_clock, \
             confidence, payload, privacy, capability, sealed, key_domain, key_epoch, \
             retention_class, retention_policy, retain_until, sensitivity \
             FROM contribution WHERE seq > ?1 ORDER BY seq ASC {limit_clause}"
        );
        let mut stmt = self.connection.prepare(&query).map_err(WriteError::Query)?;
        let mut rows = stmt.query([after_i64]).map_err(WriteError::Query)?;
        let mut result = Vec::new();
        while let Some(row) = rows.next().map_err(WriteError::Query)? {
            let seq: i64 = row.get(0).map_err(WriteError::Query)?;
            let envelope =
                crate::decode_envelope(&self.connection, row, u64::try_from(seq).unwrap_or(0))
                    .map_err(WriteError::from)?;
            result.push(envelope);
        }
        Ok(result)
    }

    /// Find an envelope by message ID.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] on database query failure or decoding error.
    pub fn find_by_message_id(
        &self,
        message_id: &Uuid,
    ) -> Result<Option<CanonicalEnvelope>, WriteError> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT seq, prev_hash, schema_version, message_id, correlation_id, causation_id, \
                 origin_organ, origin_node, kind, wall_time, monotonic_time, logical_clock, \
                 confidence, payload, privacy, capability, sealed, key_domain, key_epoch, \
                 retention_class, retention_policy, retain_until, sensitivity \
                 FROM contribution WHERE message_id=?1 LIMIT 1",
            )
            .map_err(WriteError::Query)?;
        let mut rows = stmt
            .query([message_id.hyphenated().to_string()])
            .map_err(WriteError::Query)?;
        if let Some(row) = rows.next().map_err(WriteError::Query)? {
            let seq: i64 = row.get(0).map_err(WriteError::Query)?;
            let envelope =
                crate::decode_envelope(&self.connection, row, u64::try_from(seq).unwrap_or(0))
                    .map_err(WriteError::from)?;
            Ok(Some(envelope))
        } else {
            Ok(None)
        }
    }

    /// Find envelopes by correlation ID (episode).
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] on database query failure or decoding error.
    pub fn find_by_correlation_id(
        &self,
        correlation_id: &Uuid,
    ) -> Result<Vec<CanonicalEnvelope>, WriteError> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT seq, prev_hash, schema_version, message_id, correlation_id, causation_id, \
                 origin_organ, origin_node, kind, wall_time, monotonic_time, logical_clock, \
                 confidence, payload, privacy, capability, sealed, key_domain, key_epoch, \
                 retention_class, retention_policy, retain_until, sensitivity \
                 FROM contribution WHERE correlation_id=?1 ORDER BY seq ASC",
            )
            .map_err(WriteError::Query)?;
        let mut rows = stmt
            .query([correlation_id.hyphenated().to_string()])
            .map_err(WriteError::Query)?;
        let mut result = Vec::new();
        while let Some(row) = rows.next().map_err(WriteError::Query)? {
            let seq: i64 = row.get(0).map_err(WriteError::Query)?;
            let envelope =
                crate::decode_envelope(&self.connection, row, u64::try_from(seq).unwrap_or(0))
                    .map_err(WriteError::from)?;
            result.push(envelope);
        }
        Ok(result)
    }

    /// Check if a terminal outcome exists for a cause and organ.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] on database query failure.
    pub fn has_outcome_for(&self, cause_id: &Uuid, origin_organ: &str) -> Result<bool, WriteError> {
        let exists: bool = self
            .connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM contribution WHERE causation_id=?1 AND \
                 origin_organ=?2 AND kind=15)",
                (cause_id.hyphenated().to_string(), origin_organ),
                |row| row.get(0),
            )
            .map_err(WriteError::Query)?;
        Ok(exists)
    }

    /// Retrieve evidence UUIDs for a message ID.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] on database query failure.
    pub fn evidence_for(&self, message_id: &Uuid) -> Result<Vec<Uuid>, WriteError> {
        let mut stmt = self
            .connection
            .prepare(
                "SELECT evidence_id FROM contribution_evidence WHERE contribution_id=?1 ORDER BY \
                 ordinal",
            )
            .map_err(WriteError::Query)?;
        let rows = stmt
            .query_map([message_id.hyphenated().to_string()], |row| {
                row.get::<_, String>(0)
            })
            .map_err(WriteError::Query)?;
        let mut list = Vec::new();
        for id_str in rows {
            let s = id_str.map_err(WriteError::Query)?;
            if let Ok(u) = Uuid::parse_str(&s) {
                list.push(u);
            }
        }
        Ok(list)
    }

    /// Current erasure epoch from `journal_meta`.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] on database query failure.
    pub fn erasure_epoch(&self) -> Result<u64, WriteError> {
        let epoch: i64 = self
            .connection
            .query_row(
                "SELECT erasure_epoch FROM journal_meta WHERE id=1",
                [],
                |row| row.get(0),
            )
            .map_err(WriteError::Query)?;
        u64::try_from(epoch).map_err(|_| WriteError::Malformed("negative erasure epoch"))
    }

    /// Every contribution that must be forgotten along with `target`.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] on database query failure.
    pub fn retention_closure(&self, target: &Uuid) -> Result<Vec<Uuid>, WriteError> {
        let mut closure = vec![*target];
        let mut frontier = vec![*target];

        while let Some(current) = frontier.pop() {
            let mut statement = self
                .connection
                .prepare(
                    "SELECT message_id FROM contribution WHERE causation_id = ?1 \
                     UNION \
                     SELECT c.message_id FROM contribution c \
                     JOIN contribution_evidence e ON e.contribution_id = c.message_id \
                     WHERE e.evidence_id = ?2",
                )
                .map_err(WriteError::Query)?;
            let current_text = current.to_string();
            let mut rows = statement
                .query(rusqlite::params![current_text, current_text])
                .map_err(WriteError::Query)?;
            while let Some(row) = rows.next().map_err(WriteError::Query)? {
                let raw: String = row.get(0).map_err(WriteError::Query)?;
                let Ok(dependent) = Uuid::parse_str(&raw) else {
                    continue;
                };
                if closure.contains(&dependent) {
                    continue;
                }
                closure.push(dependent);
                frontier.push(dependent);
            }
        }

        Ok(closure)
    }

    /// Redact the payloads of a closure, destroy their keys, and advance the erasure epoch.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError`] when the closure cannot be read, a key cannot be destroyed, or the
    /// transaction cannot commit.
    pub fn apply_erasure(&mut self, target: &Uuid) -> Result<Erased, WriteError> {
        let closure = self.retention_closure(target)?;

        let mut erasable = Vec::new();
        for id in &closure {
            let kind: Option<i64> = self
                .connection
                .query_row(
                    "SELECT kind FROM contribution WHERE message_id = ?1",
                    rusqlite::params![id.to_string()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(WriteError::Query)?;
            let Some(kind) = kind else {
                continue;
            };
            let Ok(kind) = u16::try_from(kind) else {
                continue;
            };
            if Kind::from_u16(kind).is_some_and(Kind::is_erasure) {
                continue;
            }
            erasable.push(*id);
        }

        if let Some(store) = &self.key_store {
            for id in &erasable {
                store
                    .destroy_key_for(id)
                    .map_err(|_| WriteError::Malformed("a data key could not be destroyed"))?;
            }
        }

        let redacted_at = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|_| WriteError::Malformed("the current instant is not representable"))?;

        let transaction = self.connection.transaction().map_err(write_error)?;
        let mut redacted = Vec::new();
        for id in &erasable {
            let changed = transaction
                .execute(
                    "UPDATE contribution SET payload = NULL, erased_at = ?1 \
                     WHERE message_id = ?2 AND hash_version = 3 AND erased_at IS NULL",
                    rusqlite::params![redacted_at, id.to_string()],
                )
                .map_err(write_error)?;
            if changed == 1 {
                redacted.push(*id);
            }
        }

        transaction
            .execute(
                "UPDATE journal_meta SET erasure_epoch = erasure_epoch + 1 WHERE id = 1",
                [],
            )
            .map_err(write_error)?;
        let epoch: i64 = transaction
            .query_row(
                "SELECT erasure_epoch FROM journal_meta WHERE id=1",
                [],
                |row| row.get(0),
            )
            .map_err(WriteError::Query)?;
        transaction.commit().map_err(write_error)?;

        Ok(Erased {
            closure,
            redacted,
            epoch: u64::try_from(epoch).map_err(|_| WriteError::Malformed("negative epoch"))?,
        })
    }

    /// Append many contributions under one transaction, returning the last accepted position.
    ///
    /// # Errors
    ///
    /// Returns the first refusal, having rolled the whole batch back. An empty batch is not an
    /// error and appends nothing.
    pub fn append_batch(
        &mut self,
        envelopes: &[CanonicalEnvelope],
    ) -> Result<Option<Appended>, WriteError> {
        if envelopes.iter().any(|envelope| envelope.sealed) {
            return Err(WriteError::SealedWithoutKeyStore);
        }
        if envelopes.is_empty() {
            return Ok(None);
        }

        let transaction = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(write_error)?;
        let mut last = None;
        for envelope in envelopes {
            last = Some(append_within_transaction(&transaction, envelope)?);
        }
        transaction.commit().map_err(write_error)?;
        Ok(last)
    }
}

#[cfg(test)]
mod tests {
    use super::{JournalWriter, WriteError, qt_instant};
    use cybou_protocol::admission::{Kind, Privacy, Rejection, Sensitivity};
    use cybou_protocol::canonical::CanonicalEnvelope;
    use uuid::Uuid;

    fn uuid(byte: u8) -> Uuid {
        Uuid::from_bytes([byte; 16])
    }

    fn observation(message: u8) -> CanonicalEnvelope {
        CanonicalEnvelope {
            schema_version: 2,
            message_id: uuid(message),
            correlation_id: uuid(200),
            causation_id: Uuid::nil(),
            origin_organ: "perceptiond".into(),
            origin_node: String::new(),
            kind: Kind::Observation as u16,
            wall_time_ms: 1_760_000_000_123,
            monotonic_time: 42,
            logical_clock: 7,
            confidence: 1.0,
            evidence: Vec::new(),
            payload: vec![0xa1, 0x01, 0x02],
            privacy: Privacy::Local as u8,
            capability_scope: String::new(),
            sealed: false,
            key_domain_id: Uuid::nil(),
            key_epoch: 0,
            retention_class: 2,
            retention_policy_version: 1,
            retain_until_ms: 0,
            sensitivity: Sensitivity::Ordinary as u8,
        }
    }

    fn writer() -> (tempfile::TempDir, JournalWriter) {
        let directory = tempfile::tempdir().expect("temporary directory");
        let writer = JournalWriter::open(&directory.path().join("journal.db")).expect("open");
        (directory, writer)
    }

    #[test]
    fn a_new_journal_is_created_at_schema_v2() {
        let (_directory, writer) = writer();
        assert_eq!(writer.user_version().expect("user_version"), 2);
        assert!(writer.has_contribution_table().expect("table"));
    }

    #[test]
    fn the_chain_starts_at_one_and_links_forward() {
        let (_directory, mut writer) = writer();
        let first = writer.append(&observation(1)).expect("first");
        assert_eq!(first.sequence, 1);

        let second = writer.append(&observation(2)).expect("second");
        assert_eq!(second.sequence, 2);
        assert_ne!(first.hash, second.hash);

        let stored: Vec<u8> = writer
            .connection
            .query_row(
                "SELECT prev_hash FROM contribution WHERE seq=2",
                [],
                |row| row.get(0),
            )
            .expect("prev_hash");
        assert_eq!(stored, first.hash.to_vec());

        let head: Option<Vec<u8>> = writer
            .connection
            .query_row(
                "SELECT prev_hash FROM contribution WHERE seq=1",
                [],
                |row| row.get(0),
            )
            .expect("prev_hash");
        assert_eq!(head, None);
    }

    #[test]
    fn a_written_row_verifies_against_the_reader() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("journal.db");
        {
            let mut writer = JournalWriter::open(&path).expect("open");
            writer.append(&observation(1)).expect("first");
            writer.append(&observation(2)).expect("second");
        }

        let inspection = crate::inspect_journal(&path).expect("inspect");
        assert_eq!(inspection.contribution_count, 2);

        let verification = crate::verify_journal_from(&path, None).expect("verify");
        assert_eq!(verification.verified_through, 2);
        assert_eq!(verification.content_verified, 2);
        assert_eq!(verification.content_skipped, 0);
    }

    #[test]
    fn a_refused_contribution_leaves_the_journal_unchanged() {
        let (_directory, mut writer) = writer();
        writer.append(&observation(1)).expect("first");

        let error = writer.append(&observation(1)).expect_err("duplicate");
        assert!(matches!(
            error,
            WriteError::Refused(Rejection::DuplicateMessageId)
        ));

        let count: i64 = writer
            .connection
            .query_row("SELECT COUNT(*) FROM contribution", [], |row| row.get(0))
            .expect("count");
        assert_eq!(count, 1);
    }

    #[test]
    fn a_derived_contribution_records_its_evidence_in_order() {
        let (_directory, mut writer) = writer();
        writer.append(&observation(1)).expect("first");
        writer.append(&observation(2)).expect("second");

        let mut learning = observation(3);
        learning.kind = Kind::Learning as u16;
        learning.causation_id = uuid(1);
        learning.evidence = vec![uuid(2)];
        writer.append(&learning).expect("learning");

        let mut statement = writer
            .connection
            .prepare(
                "SELECT evidence_id, ordinal FROM contribution_evidence \
                 WHERE contribution_id = ?1 ORDER BY ordinal",
            )
            .expect("prepare");
        let rows: Vec<(String, i64)> = statement
            .query_map([uuid(3).hyphenated().to_string()], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("query")
            .map(|row| row.expect("row"))
            .collect();
        assert_eq!(rows, vec![(uuid(2).hyphenated().to_string(), 0)]);
    }

    #[test]
    fn a_contribution_citing_something_absent_is_refused() {
        let (_directory, mut writer) = writer();
        let mut learning = observation(3);
        learning.kind = Kind::Learning as u16;
        learning.causation_id = uuid(99);
        assert!(matches!(
            writer.append(&learning).expect_err("missing cause"),
            WriteError::Refused(Rejection::MissingCausation)
        ));
    }

    #[test]
    fn a_sealed_contribution_is_refused_rather_than_stored_in_the_clear() {
        let (_directory, mut writer) = writer();
        let mut sealed = observation(1);
        sealed.schema_version = 3;
        sealed.sealed = true;
        sealed.key_domain_id = uuid(7);
        assert!(matches!(
            writer.append(&sealed).expect_err("sealed"),
            WriteError::SealedWithoutKeyStore
        ));
    }

    #[test]
    fn a_newer_schema_is_refused_rather_than_written_to() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("journal.db");
        JournalWriter::open(&path).expect("create");
        {
            let connection = rusqlite::Connection::open(&path).expect("reopen");
            connection
                .execute_batch("PRAGMA user_version = 3")
                .expect("bump");
        }
        assert!(matches!(
            JournalWriter::open(&path).expect_err("newer"),
            WriteError::UnsupportedSchema { received: 3, .. }
        ));
    }

    #[test]
    fn a_declared_schema_with_no_tables_is_refused_rather_than_repaired() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("journal.db");
        {
            let connection = rusqlite::Connection::open(&path).expect("create");
            connection
                .execute_batch("PRAGMA user_version = 2")
                .expect("declare");
        }
        assert!(matches!(
            JournalWriter::open(&path).expect_err("inconsistent"),
            WriteError::InconsistentSchema(2)
        ));
    }

    #[test]
    fn instants_carry_exactly_three_subsecond_digits() {
        assert_eq!(
            qt_instant(1_760_000_000_123).as_deref(),
            Some("2025-10-09T08:53:20.123Z")
        );
        assert_eq!(
            qt_instant(1_760_000_000_000).as_deref(),
            Some("2025-10-09T08:53:20.000Z")
        );
    }

    #[test]
    fn a_second_writer_is_refused_rather_than_admitted() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("journal.db");
        let holder = JournalWriter::open(&path).expect("holder");
        let mut contender = JournalWriter::open(&path).expect("contender");

        holder
            .connection
            .execute_batch("BEGIN IMMEDIATE")
            .expect("hold");

        let error = contender.append(&observation(1)).expect_err("contended");
        assert!(
            matches!(error, WriteError::Concurrent),
            "a busy database must not look like a defective statement: {error:?}"
        );

        holder
            .connection
            .execute_batch("ROLLBACK")
            .expect("release");
        assert!(contender.append(&observation(1)).is_ok());
    }

    #[test]
    fn a_refusal_leaves_the_database_byte_identical() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("journal.db");
        {
            let mut writer = JournalWriter::open(&path).expect("open");
            writer.append(&observation(1)).expect("first");
        }
        let before = std::fs::read(&path).expect("read before");

        {
            let mut writer = JournalWriter::open(&path).expect("reopen");
            let mut dangling = observation(3);
            dangling.kind = Kind::Learning as u16;
            dangling.causation_id = uuid(99);
            writer.append(&dangling).expect_err("dangling");
        }

        assert_eq!(
            std::fs::read(&path).expect("read after"),
            before,
            "a refused append must not change a single byte"
        );
    }

    fn legacy_journal(path: &std::path::Path) {
        let connection = rusqlite::Connection::open(path).expect("create");
        connection
            .execute_batch(
                "PRAGMA journal_mode=WAL;
                 CREATE TABLE contribution (
                     seq            INTEGER PRIMARY KEY AUTOINCREMENT,
                     message_id     TEXT    NOT NULL UNIQUE,
                     correlation_id TEXT    NOT NULL,
                     causation_id   TEXT,
                     origin_organ   TEXT    NOT NULL,
                     origin_node    TEXT    NOT NULL DEFAULT '',
                     kind           INTEGER NOT NULL,
                     wall_time      TEXT    NOT NULL,
                     monotonic_time INTEGER NOT NULL,
                     logical_clock  INTEGER NOT NULL,
                     confidence     REAL    NOT NULL,
                     evidence       TEXT,
                     payload        BLOB,
                     privacy        INTEGER NOT NULL,
                     capability     TEXT,
                     prev_hash      BLOB,
                     hash           BLOB    NOT NULL
                 );
                 CREATE TABLE journal_meta (
                     id             INTEGER PRIMARY KEY CHECK (id = 1),
                     erasure_epoch  INTEGER NOT NULL DEFAULT 0,
                     rotated_epoch  INTEGER NOT NULL DEFAULT 0
                 );
                 INSERT OR IGNORE INTO journal_meta (id) VALUES (1);
                 PRAGMA user_version = 1;",
            )
            .expect("legacy schema");
    }

    fn legacy_row(
        connection: &rusqlite::Connection,
        sequence: i64,
        envelope: &CanonicalEnvelope,
        evidence_column: Option<&str>,
    ) {
        let previous: Vec<u8> = connection
            .query_row(
                "SELECT hash FROM contribution ORDER BY seq DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or_default();
        let wall_time = qt_instant(envelope.wall_time_ms).expect("instant");

        let mut input = previous.clone();
        input.extend_from_slice(sequence.to_string().as_bytes());
        for id in [
            envelope.message_id,
            envelope.correlation_id,
            envelope.causation_id,
        ] {
            input.extend_from_slice(format!("{{{id}}}").as_bytes());
        }
        input.extend_from_slice(envelope.origin_organ.as_bytes());
        input.extend_from_slice(u16::to_string(&envelope.kind).as_bytes());
        input.extend_from_slice(wall_time.as_bytes());
        input.extend_from_slice(envelope.logical_clock.to_string().as_bytes());
        input.extend_from_slice(&envelope.payload);
        let hash = cybou_protocol::canonical::sha256(&input);

        connection
            .execute(
                "INSERT INTO contribution (seq, message_id, correlation_id, causation_id, \
                 origin_organ, origin_node, kind, wall_time, monotonic_time, logical_clock, \
                 confidence, evidence, payload, privacy, capability, prev_hash, hash) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
                rusqlite::params![
                    sequence,
                    envelope.message_id.hyphenated().to_string(),
                    envelope.correlation_id.hyphenated().to_string(),
                    None::<String>,
                    envelope.origin_organ,
                    envelope.origin_node,
                    i64::from(envelope.kind),
                    wall_time,
                    0_i64,
                    i64::try_from(envelope.logical_clock).expect("clock"),
                    envelope.confidence,
                    evidence_column,
                    envelope.payload,
                    i64::from(envelope.privacy),
                    envelope.capability_scope,
                    previous,
                    hash.as_slice(),
                ],
            )
            .expect("legacy row");
    }

    #[test]
    fn a_legacy_journal_migrates_with_its_evidence_and_a_backup() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("journal.db");
        legacy_journal(&path);
        {
            let connection = rusqlite::Connection::open(&path).expect("open");
            legacy_row(&connection, 1, &observation(1), None);
            legacy_row(&connection, 2, &observation(2), None);
            let mut learning = observation(3);
            learning.kind = Kind::Learning as u16;
            legacy_row(
                &connection,
                3,
                &learning,
                Some(&format!(
                    "{},{}",
                    uuid(1).hyphenated(),
                    uuid(2).hyphenated()
                )),
            );
        }

        super::migrate_v1_to_v2(&path).expect("migrate");

        let inspection = crate::inspect_journal(&path).expect("inspect");
        assert_eq!(inspection.schema_version, 2);
        assert_eq!(inspection.contribution_count, 3);

        let connection = rusqlite::Connection::open(&path).expect("reopen");
        let mut statement = connection
            .prepare(
                "SELECT evidence_id, ordinal FROM contribution_evidence \
                 WHERE contribution_id = ?1 ORDER BY ordinal",
            )
            .expect("prepare");
        let links: Vec<(String, i64)> = statement
            .query_map([uuid(3).hyphenated().to_string()], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("query")
            .map(|row| row.expect("row"))
            .collect();
        assert_eq!(
            links,
            vec![
                (uuid(1).hyphenated().to_string(), 0),
                (uuid(2).hyphenated().to_string(), 1),
            ],
            "legacy evidence order is preserved as a column, not as a joined string"
        );

        let mut backup = path.clone().into_os_string();
        backup.push(".v1.bak");
        assert!(
            std::path::PathBuf::from(backup).exists(),
            "the pre-migration copy must survive the migration"
        );
    }

    #[test]
    fn a_migrated_legacy_chain_still_verifies_at_hash_v1() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("journal.db");
        legacy_journal(&path);
        {
            let connection = rusqlite::Connection::open(&path).expect("open");
            legacy_row(&connection, 1, &observation(1), None);
            legacy_row(&connection, 2, &observation(2), None);
        }

        super::migrate_v1_to_v2(&path).expect("migrate");

        let verification = crate::verify_journal_from(&path, None).expect("verify");
        assert_eq!(verification.verified_through, 2);
    }

    #[test]
    fn a_broken_legacy_chain_is_refused_and_nothing_is_migrated() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("journal.db");
        legacy_journal(&path);
        {
            let connection = rusqlite::Connection::open(&path).expect("open");
            legacy_row(&connection, 1, &observation(1), None);
            legacy_row(&connection, 2, &observation(2), None);
            connection
                .execute(
                    "UPDATE contribution SET payload = ?1 WHERE seq = 2",
                    rusqlite::params![vec![0xff_u8, 0xff]],
                )
                .expect("tamper");
        }

        let error = super::migrate_v1_to_v2(&path).expect_err("broken chain");
        assert!(matches!(error, WriteError::Migration(_)), "{error:?}");

        let connection = rusqlite::Connection::open(&path).expect("reopen");
        let version: i64 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("user_version");
        assert_eq!(version, 1, "a refused migration leaves the journal at v1");
    }

    #[test]
    fn legacy_evidence_naming_something_absent_is_refused() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("journal.db");
        legacy_journal(&path);
        {
            let connection = rusqlite::Connection::open(&path).expect("open");
            let mut learning = observation(3);
            learning.kind = Kind::Learning as u16;
            legacy_row(
                &connection,
                1,
                &learning,
                Some(&uuid(99).hyphenated().to_string()),
            );
        }

        let error = super::migrate_v1_to_v2(&path).expect_err("dangling evidence");
        assert!(
            matches!(&error, WriteError::Migration(reason) if reason.contains("missing")),
            "{error:?}"
        );
    }

    #[test]
    fn a_partially_versioned_schema_is_refused_rather_than_repaired() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("journal.db");
        legacy_journal(&path);
        {
            let connection = rusqlite::Connection::open(&path).expect("open");
            connection
                .execute_batch(
                    "ALTER TABLE contribution ADD COLUMN schema_version INTEGER NOT NULL DEFAULT 1",
                )
                .expect("half-migrate");
        }

        let error = super::migrate_v1_to_v2(&path).expect_err("partial");
        assert!(
            matches!(&error, WriteError::Migration(reason) if reason.contains("partially")),
            "{error:?}"
        );
    }

    #[test]
    fn a_v2_journal_is_not_migrated_again() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("journal.db");
        JournalWriter::open(&path).expect("create v2");

        let error = super::migrate_v1_to_v2(&path).expect_err("already v2");
        assert!(matches!(error, WriteError::Migration(_)), "{error:?}");
    }
}
