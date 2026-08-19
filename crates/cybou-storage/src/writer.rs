// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The Rust Journal writer, in the two slices that do not need a key store.
//!
//! W2 opens or creates the predecessor's schema v2 and refuses to proceed when the durability the
//! Journal claims did not actually take. W3 appends one contribution: it resolves the references
//! the envelope names, applies [`cybou_protocol::admission`], chains hash v3 over its split
//! commitment, and inserts the row with the predecessor's exact column spellings.
//!
//! This is not an owner. The C++ `eventd` remains the single canonical writer until the
//! differential, interruption, recovery, scale, and rollback gates pass, and dual-running two
//! canonical owners against the same writable state stays forbidden. What this type provides is a
//! writer that can be compared against the predecessor, not one that may replace it.
//!
//! Sealing is deliberately absent. A sealed contribution is refused outright rather than stored in
//! the clear: a payload written unsealed because no key store was reachable would be a payload
//! nobody could later erase.

use std::fmt::Write as _;
use std::path::Path;

use cybou_protocol::admission::{
    self, Kind, Privacy, ReferenceFacts, Rejection, Resolved, Sensitivity,
};
use cybou_protocol::canonical::{
    CanonicalEnvelope, canonical_journal_row_v3, commitment_v3, sha256,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{JOURNAL_SCHEMA_V2, StorageError};

/// Journal hash version written by this build.
pub const JOURNAL_HASH_V3: i64 = 3;

/// `SQLite` `synchronous` level at or above which a returned `COMMIT` has reached storage.
///
/// Below this, acceptance would be published for a commit that a power loss can still discard.
const REQUIRED_SYNCHRONOUS_LEVEL: i64 = 2;

/// The evidence join table, shared by schema creation and the v1 migration.
///
/// One definition rather than two copies: a migration that produced a subtly different table from
/// the one creation produces would leave two shapes of Journal in the world, and only one of them
/// would be the shape the tests cover.
const EVIDENCE_TABLE_DDL: &str = "CREATE TABLE contribution_evidence (
     contribution_id TEXT    NOT NULL,
     evidence_id     TEXT    NOT NULL,
     ordinal         INTEGER NOT NULL,
     PRIMARY KEY (contribution_id, evidence_id),
     UNIQUE (contribution_id, ordinal),
     FOREIGN KEY (contribution_id) REFERENCES contribution(message_id) ON DELETE RESTRICT,
     FOREIGN KEY (evidence_id) REFERENCES contribution(message_id) ON DELETE RESTRICT
 );";

/// The v2 indexes, including the partial unique index that makes one terminal `Outcome` per cause
/// a constraint of the storage rather than only a rule the writer applies.
fn v2_indexes_ddl() -> String {
    format!(
        "CREATE INDEX IF NOT EXISTS idx_correlation ON contribution(correlation_id);
         CREATE INDEX IF NOT EXISTS idx_causation ON contribution(causation_id);
         CREATE INDEX IF NOT EXISTS idx_kind ON contribution(kind);
         CREATE INDEX IF NOT EXISTS idx_evidence_target ON contribution_evidence(evidence_id);
         CREATE UNIQUE INDEX IF NOT EXISTS idx_one_outcome_per_cause
             ON contribution(causation_id)
             WHERE kind = {outcome} AND causation_id IS NOT NULL;",
        outcome = Kind::Outcome as u16,
    )
}

/// Whether a failure is another writer holding the database rather than a defective statement.
///
/// A caller must be able to tell "someone else is writing" from "this statement is wrong": the
/// first is a retry, the second is a bug, and collapsing them into one error would have every
/// concurrency stall look like corruption.
fn is_busy(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(inner, _)
            if inner.code == rusqlite::ErrorCode::DatabaseBusy
                || inner.code == rusqlite::ErrorCode::DatabaseLocked
    )
}

fn write_error(error: rusqlite::Error) -> WriteError {
    if is_busy(&error) {
        WriteError::Concurrent
    } else {
        WriteError::Query(error)
    }
}

/// Why the writer refused.
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    /// The database could not be opened or created.
    #[error("cannot open Journal for writing: {0}")]
    Open(#[source] rusqlite::Error),
    /// A statement failed.
    #[error("Journal write failed: {0}")]
    Query(#[source] rusqlite::Error),
    /// The commit mode or synchronisation level is weaker than the durability claim requires.
    #[error("Journal durability is weaker than required: {0}")]
    Durability(String),
    /// The existing schema is not one this slice may write to.
    #[error("Journal schema {received} cannot be written by this build; expected {expected}")]
    UnsupportedSchema {
        /// Version read from `PRAGMA user_version`.
        received: i64,
        /// Version this build writes.
        expected: i64,
    },
    /// A database declaring a schema but missing its tables must never be repaired implicitly.
    #[error("Journal declares schema {0} but has no contribution table")]
    InconsistentSchema(i64),
    /// The contribution is not admissible.
    #[error("contribution refused: {0}")]
    Refused(#[from] Rejection),
    /// A sealed contribution reached a writer with no key store.
    #[error("refusing a sealed contribution: this journal has no key store")]
    SealedWithoutKeyStore,
    /// Cryptographic failure.
    #[error("cryptographic failure: {0}")]
    Crypto(#[from] cybou_crypto::CryptoError),
    /// Key store failure.
    #[error("key store failure: {0}")]
    KeyStore(#[from] cybou_crypto::KeyStoreError),
    /// A stored value could not be read back as the type its column promises.
    #[error("Journal contains a malformed stored value: {0}")]
    Malformed(&'static str),
    /// Another writer holds the database.
    #[error("another writer holds this Journal")]
    Concurrent,
    /// The v1 migration refused, leaving the database exactly as it was.
    #[error("Journal migration refused: {0}")]
    Migration(String),
}

impl From<StorageError> for WriteError {
    fn from(value: StorageError) -> Self {
        match value {
            StorageError::Open(error) => Self::Open(error),
            StorageError::Query(error) => Self::Query(error),
            other => Self::Durability(other.to_string()),
        }
    }
}

/// One appended contribution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Appended {
    /// Sequence assigned by the chain, starting at one.
    pub sequence: u64,
    /// Row hash written at [`JOURNAL_HASH_V3`].
    pub hash: [u8; 32],
}

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
    /// Durability is verified rather than requested. `SQLite` silently keeps the previous mode when
    /// it cannot apply one — a filesystem without shared-memory support falls back from WAL, for
    /// instance — and a silent fallback would leave "durable before visible" stated more strongly
    /// than the storage supports. Both pragmas are read back, and the writer refuses to open rather
    /// than weakening the guarantee unannounced.
    ///
    /// A v1 database is refused, not migrated. Migration has its own backup, interruption, and
    /// rollback evidence, and performing it as a side effect of opening a connection would run it
    /// where none of that evidence is being collected.
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

    /// Attach an active KeyStore, key encryption key (KEK), and key domain for sealing sensitive contributions.
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
    /// Everything happens inside one `BEGIN IMMEDIATE` transaction: the reference reads that
    /// admission depends on, the tail read that assigns the sequence, and the insert. Reading the
    /// references outside the transaction would let a concurrent writer erase or expire one between
    /// the check and the row that rests on it.
    ///
    /// The returned sequence is not acceptance. Acceptance is published by the Event owner after
    /// this call returns, which is what "durable before visible" means and why nothing here emits a
    /// signal.
    ///
    /// # Errors
    ///
    /// Returns [`WriteError::Refused`] when a rule declines the contribution, leaving the Journal
    /// exactly as it was, or another [`WriteError`] when the database itself failed.
    pub fn append(&mut self, envelope: &CanonicalEnvelope) -> Result<Appended, WriteError> {
        let envelope_to_write;
        let target_envelope = if envelope.sealed {
            let (Some(store), Some(kek), Some(domain)) = (&self.key_store, &self.kek, &self.key_domain) else {
                return Err(WriteError::SealedWithoutKeyStore);
            };
            let data_key = store.create_key_for(&envelope.message_id, kek)?;
            let sealed = cybou_crypto::Seal::seal(&envelope.payload, &data_key)?;
            let mut stored = envelope.clone();
            let mut payload_bytes = Vec::with_capacity(sealed.nonce.len() + sealed.ciphertext.len());
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

        // Immediate rather than deferred: the write lock is taken before the reference reads, so
        // a concurrent writer is refused here, at the start, rather than at the commit after the
        // admission rules have already been decided against state that moved.
        let transaction = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(write_error)?;
        let appended = append_within_transaction(&transaction, target_envelope)?;
        transaction.commit().map_err(write_error)?;
        Ok(appended)
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
                 retention_class, retention_policy, retain_until \
                 FROM contribution ORDER BY seq DESC LIMIT 1",
            )
            .map_err(WriteError::Query)?;
        let mut rows = stmt.query([]).map_err(WriteError::Query)?;
        if let Some(row) = rows.next().map_err(WriteError::Query)? {
            let seq: i64 = row.get(0).map_err(WriteError::Query)?;
            let envelope = crate::decode_envelope(
                &self.connection,
                row,
                u64::try_from(seq).unwrap_or(0),
            )
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
                 retention_class, retention_policy, retain_until \
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
             retention_class, retention_policy, retain_until \
             FROM contribution {limit_clause}"
        );
        let mut stmt = self.connection.prepare(&query).map_err(WriteError::Query)?;
        let mut rows = stmt.query([]).map_err(WriteError::Query)?;
        let mut result = Vec::new();
        while let Some(row) = rows.next().map_err(WriteError::Query)? {
            let seq: i64 = row.get(0).map_err(WriteError::Query)?;
            let envelope = crate::decode_envelope(
                &self.connection,
                row,
                u64::try_from(seq).unwrap_or(0),
            )
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
        let after_i64 =
            i64::try_from(after_sequence).map_err(|_| WriteError::Malformed("sequence overflow"))?;
        let limit_clause = if limit > 0 {
            format!("LIMIT {limit}")
        } else {
            String::new()
        };
        let query = format!(
            "SELECT seq, prev_hash, schema_version, message_id, correlation_id, causation_id, \
             origin_organ, origin_node, kind, wall_time, monotonic_time, logical_clock, \
             confidence, payload, privacy, capability, sealed, key_domain, key_epoch, \
             retention_class, retention_policy, retain_until \
             FROM contribution WHERE seq > ?1 ORDER BY seq ASC {limit_clause}"
        );
        let mut stmt = self.connection.prepare(&query).map_err(WriteError::Query)?;
        let mut rows = stmt.query([after_i64]).map_err(WriteError::Query)?;
        let mut result = Vec::new();
        while let Some(row) = rows.next().map_err(WriteError::Query)? {
            let seq: i64 = row.get(0).map_err(WriteError::Query)?;
            let envelope = crate::decode_envelope(
                &self.connection,
                row,
                u64::try_from(seq).unwrap_or(0),
            )
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
                 retention_class, retention_policy, retain_until \
                 FROM contribution WHERE message_id=?1 LIMIT 1",
            )
            .map_err(WriteError::Query)?;
        let mut rows = stmt
            .query([message_id.hyphenated().to_string()])
            .map_err(WriteError::Query)?;
        if let Some(row) = rows.next().map_err(WriteError::Query)? {
            let seq: i64 = row.get(0).map_err(WriteError::Query)?;
            let envelope = crate::decode_envelope(
                &self.connection,
                row,
                u64::try_from(seq).unwrap_or(0),
            )
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
                 retention_class, retention_policy, retain_until \
                 FROM contribution WHERE correlation_id=?1 ORDER BY seq ASC",
            )
            .map_err(WriteError::Query)?;
        let mut rows = stmt
            .query([correlation_id.hyphenated().to_string()])
            .map_err(WriteError::Query)?;
        let mut result = Vec::new();
        while let Some(row) = rows.next().map_err(WriteError::Query)? {
            let seq: i64 = row.get(0).map_err(WriteError::Query)?;
            let envelope = crate::decode_envelope(
                &self.connection,
                row,
                u64::try_from(seq).unwrap_or(0),
            )
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

    /// Current erasure epoch from journal_meta.
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

    /// Append many contributions under one transaction, returning the last accepted position.
    ///
    /// Every contribution is validated, hashed and chained exactly as [`Self::append`] does; only
    /// the commit — and therefore the fsync — is shared. This exists so a large Journal can be
    /// built for measurement without spending one fsync per row, which at a million rows is the
    /// difference between minutes and hours.
    ///
    /// It must never be reachable from Event1. Acceptance there is per contribution and has to
    /// stay that way: batching it would publish acceptance for contributions whose commit had not
    /// yet returned, which is exactly the durability ordering this writer exists to preserve. The
    /// batch is atomic, so a refusal anywhere leaves the Journal exactly as it was.
    ///
    /// A contribution may cite one earlier in the same batch. It is already inserted by then, so
    /// the reference resolves against the open transaction like any other.
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

/// Validate, hash, chain and insert one contribution inside a transaction the caller opened.
///
/// Never commits and never rolls back. Both are the caller's, because the difference between one
/// append and a batch is exactly where the commit goes.
fn append_within_transaction(
    transaction: &Connection,
    envelope: &CanonicalEnvelope,
) -> Result<Appended, WriteError> {
    {
        let resolved = resolve(transaction, envelope)?;
        admission::check_admission(envelope, &resolved)?;

        let (sequence, previous_hash) = tail(transaction)?;
        let (_, payload_commitment, commitment) = commitment_v3(envelope);
        let hash = sha256(&canonical_journal_row_v3(
            sequence,
            &previous_hash,
            &commitment,
        ));

        // SQLite has no unsigned integer type, and the predecessor binds these as signed 64-bit
        // values. A count past `i64::MAX` cannot be stored without changing what was stored, so it
        // is refused rather than wrapped.
        let signed = |value: u64, what: &'static str| {
            i64::try_from(value).map_err(|_| WriteError::Malformed(what))
        };

        transaction
            .execute(
                "INSERT INTO contribution (seq, message_id, correlation_id, causation_id, \
                 origin_organ, origin_node, kind, wall_time, monotonic_time, logical_clock, \
                 confidence, evidence, payload, privacy, capability, schema_version, \
                 hash_version, prev_hash, hash, commitment, payload_commitment, sealed, \
                 key_domain, key_epoch, retention_class, retention_policy, retain_until, \
                 sensitivity) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,\
                 ?21,?22,?23,?24,?25,?26,?27,?28)",
                params![
                    signed(sequence, "sequence is out of range")?,
                    hyphenated(envelope.message_id),
                    hyphenated(envelope.correlation_id),
                    optional_uuid(envelope.causation_id),
                    envelope.origin_organ,
                    envelope.origin_node,
                    i64::from(envelope.kind),
                    qt_instant(envelope.wall_time_ms)
                        .ok_or(WriteError::Malformed("wall time is out of range"))?,
                    signed(envelope.monotonic_time, "monotonic time is out of range")?,
                    signed(envelope.logical_clock, "logical clock is out of range")?,
                    envelope.confidence,
                    // The evidence column is inherited and always null: evidence lives in
                    // `contribution_evidence`, where its order is a column rather than a
                    // convention about how a string was joined.
                    None::<String>,
                    envelope.payload,
                    i64::from(envelope.privacy),
                    // Absent, not empty. The predecessor binds a null `QString` here and the
                    // SQLite driver turns that into NULL; it converts `originNode` to an empty
                    // string with an explicit ternary and does not do the same for this column,
                    // so the difference is a decision rather than an accident. The canonical
                    // envelope represents an absent capability scope as an empty string, so that
                    // is the value that has to become NULL.
                    absent_if_empty(&envelope.capability_scope),
                    i64::from(envelope.schema_version),
                    JOURNAL_HASH_V3,
                    // The first row chains onto nothing, and the predecessor stores that as NULL
                    // rather than a zero-length blob. Verification treats the two alike, so this
                    // would never have failed a check — it would only have made every Journal
                    // written by Rust distinguishable from every Journal written by Qt.
                    previous_hash_column(&previous_hash),
                    hash.as_slice(),
                    commitment.as_slice(),
                    payload_commitment.as_slice(),
                    0_i64,
                    optional_uuid(envelope.key_domain_id),
                    envelope.key_epoch,
                    i64::from(envelope.retention_class),
                    i64::from(envelope.retention_policy_version),
                    optional_instant(envelope.retain_until_ms)
                        .transpose()
                        .map_err(|()| WriteError::Malformed("retention time is out of range"))?,
                    i64::from(envelope.sensitivity),
                ],
            )
            .map_err(WriteError::Query)?;

        for (ordinal, evidence_id) in envelope.evidence.iter().enumerate() {
            transaction
                .execute(
                    "INSERT INTO contribution_evidence (contribution_id, evidence_id, ordinal) \
                     VALUES (?1, ?2, ?3)",
                    params![
                        hyphenated(envelope.message_id),
                        hyphenated(*evidence_id),
                        i64::try_from(ordinal).map_err(|_| WriteError::Malformed(
                            "evidence ordinal is out of range"
                        ))?
                    ],
                )
                .map_err(WriteError::Query)?;
        }

        Ok(Appended { sequence, hash })
    }
}

/// Migrate a v1 Journal to schema v2, or refuse and leave it untouched.
///
/// Explicit rather than automatic. The predecessor performs this while opening a connection, and
/// that is the one part of its behavior not reproduced here: a migration carries a backup, a
/// verification of the entire legacy chain, and a rollback, and running it as a side effect of
/// opening a database means running it wherever a connection happens to be made — including in
/// processes that are only reading, and at moments when nothing is watching for it to fail.
///
/// The order is the predecessor's and matters: the backup is taken before the transaction opens,
/// because a `VACUUM INTO` cannot run inside one, and because a backup taken after the first
/// irreversible step is not a backup. Everything after it is one transaction, so an interruption
/// leaves either a v1 database with a spare backup beside it or a complete v2 — never a
/// half-migrated one.
///
/// The legacy chain is verified *before* the commit, not after. A migration that committed a broken
/// chain and then reported it would have already made the corruption the new baseline.
///
/// # Errors
///
/// Returns [`WriteError::Migration`] when the database is not a migratable v1, when legacy evidence
/// is malformed or dangling, when a cause carries more than one terminal `Outcome`, or when the
/// legacy hash chain does not verify. In every case the database is left unchanged.
pub fn migrate_v1_to_v2(path: &Path) -> Result<(), WriteError> {
    let connection = open_for_write(path)?;
    ensure_durability(&connection)?;

    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(write_error)?;
    if version != 0 && version != 1 {
        return Err(WriteError::Migration(format!(
            "schema {version} is not a migratable v1 journal"
        )));
    }
    if !table_exists(&connection, "contribution")? {
        return Err(WriteError::Migration(
            "journal has no contribution table to migrate".into(),
        ));
    }
    // A journal already carrying half the v2 columns was interrupted by something this migration
    // did not do, and repairing it blind would decide what the missing half should have been.
    for column in ["schema_version", "hash_version"] {
        if column_exists(&connection, "contribution", column)? {
            return Err(WriteError::Migration(
                "journal has a partially versioned schema; refusing repair".into(),
            ));
        }
    }

    create_migration_backup(&connection, path)?;

    let mut statements = String::from("BEGIN IMMEDIATE;\n");
    for column in [
        "schema_version INTEGER NOT NULL DEFAULT 1",
        "hash_version INTEGER NOT NULL DEFAULT 1",
        "commitment BLOB",
        "payload_commitment BLOB",
        "erased_at TEXT",
        "sealed INTEGER NOT NULL DEFAULT 0",
        "key_domain TEXT",
        "key_epoch INTEGER NOT NULL DEFAULT 0",
        "retention_class INTEGER NOT NULL DEFAULT 2",
        "retention_policy INTEGER NOT NULL DEFAULT 0",
        "retain_until TEXT",
        "sensitivity INTEGER NOT NULL DEFAULT 1",
    ] {
        writeln!(statements, "ALTER TABLE contribution ADD COLUMN {column};")
            .map_err(|_| WriteError::Malformed("cannot build the migration statement"))?;
    }
    statements.push_str(EVIDENCE_TABLE_DDL);
    connection.execute_batch(&statements).map_err(write_error)?;

    match migrate_within_transaction(&connection) {
        Ok(()) => connection.execute_batch("COMMIT;").map_err(write_error),
        Err(error) => {
            // Best effort: the transaction is already doomed, and a failing rollback must not
            // replace the reason the migration refused.
            drop(connection.execute_batch("ROLLBACK;"));
            Err(error)
        }
    }
}

fn migrate_within_transaction(connection: &Connection) -> Result<(), WriteError> {
    move_legacy_evidence(connection)?;

    let duplicate: Option<String> = connection
        .query_row(
            "SELECT causation_id FROM contribution \
             WHERE kind = ?1 AND causation_id IS NOT NULL \
             GROUP BY causation_id HAVING COUNT(*) > 1 LIMIT 1",
            params![Kind::Outcome as u16],
            |row| row.get(0),
        )
        .optional()
        .map_err(write_error)?;
    if duplicate.is_some() {
        return Err(WriteError::Migration(
            "legacy journal contains multiple terminal Outcomes for one cause".into(),
        ));
    }

    connection
        .execute_batch(&format!(
            "UPDATE contribution SET schema_version = 1, hash_version = 1;
             {indexes}
             PRAGMA user_version = {version};",
            indexes = v2_indexes_ddl(),
            version = JOURNAL_SCHEMA_V2,
        ))
        .map_err(write_error)?;

    crate::inspect_chain(connection, None, None)
        .map_err(|error| WriteError::Migration(format!("legacy hash chain is broken: {error}")))?;

    Ok(())
}

/// Turn the legacy comma-joined `evidence` column into ordered join-table rows.
///
/// Every identity is parsed, deduplicated, and required to exist. A dangling reference is refused
/// rather than dropped: an evidence link that silently disappeared during a migration would leave a
/// conclusion in the Journal with nothing recorded under it.
fn move_legacy_evidence(connection: &Connection) -> Result<(), WriteError> {
    let mut legacy = connection
        .prepare("SELECT message_id, evidence FROM contribution ORDER BY seq")
        .map_err(write_error)?;
    let rows: Vec<(String, Option<String>)> = legacy
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(write_error)?
        .collect::<Result<_, _>>()
        .map_err(write_error)?;

    let malformed = || WriteError::Migration("legacy evidence contains an invalid UUID".into());

    for (contribution_id, evidence) in rows {
        let Some(evidence) = evidence else { continue };
        let mut seen: Vec<String> = Vec::new();
        for (ordinal, raw) in evidence
            .split(',')
            .filter(|part| !part.is_empty())
            .enumerate()
        {
            let parsed = Uuid::parse_str(raw.trim()).map_err(|_| malformed())?;
            if parsed.is_nil() {
                return Err(malformed());
            }
            let evidence_id = hyphenated(parsed);
            if seen.contains(&evidence_id) {
                return Err(WriteError::Migration(
                    "legacy evidence contains a duplicate UUID".into(),
                ));
            }
            seen.push(evidence_id.clone());

            if !message_id_present(connection, &evidence_id)? {
                return Err(WriteError::Migration(
                    "legacy evidence references a missing contribution".into(),
                ));
            }

            let ordinal = i64::try_from(ordinal)
                .map_err(|_| WriteError::Malformed("evidence ordinal is out of range"))?;
            connection
                .execute(
                    "INSERT INTO contribution_evidence (contribution_id, evidence_id, ordinal) \
                     VALUES (?1, ?2, ?3)",
                    params![contribution_id, evidence_id, ordinal],
                )
                .map_err(write_error)?;
        }
    }

    Ok(())
}

fn message_id_present(connection: &Connection, message_id: &str) -> Result<bool, WriteError> {
    connection
        .query_row(
            "SELECT 1 FROM contribution WHERE message_id = ?1 LIMIT 1",
            params![message_id],
            |_| Ok(()),
        )
        .optional()
        .map(|found| found.is_some())
        .map_err(write_error)
}

/// A copy of the database as it was, before the first irreversible step.
///
/// `VACUUM INTO` rather than a file copy: it runs through `SQLite` and therefore accounts for the
/// write-ahead log, which copying the main file alone would silently leave behind.
fn create_migration_backup(connection: &Connection, path: &Path) -> Result<(), WriteError> {
    connection
        .query_row("PRAGMA wal_checkpoint(FULL)", [], |_| Ok(()))
        .optional()
        .map_err(write_error)?;

    let mut backup = path.as_os_str().to_os_string();
    backup.push(".v1.bak");
    let backup = std::path::PathBuf::from(backup);
    if backup.exists() {
        std::fs::remove_file(&backup).map_err(|error| {
            WriteError::Migration(format!("cannot replace migration backup: {error}"))
        })?;
    }
    let literal = backup
        .to_str()
        .ok_or(WriteError::Malformed("backup path is not valid UTF-8"))?
        .replace('\'', "''");
    connection
        .execute_batch(&format!("VACUUM INTO '{literal}'"))
        .map_err(write_error)
}

fn open_for_write(path: &Path) -> Result<Connection, WriteError> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(WriteError::Open)?;
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .map_err(write_error)?;
    for pragma in [
        "PRAGMA foreign_keys=ON",
        "PRAGMA journal_mode=WAL",
        "PRAGMA synchronous=FULL",
    ] {
        // `journal_mode` answers with a row, so it cannot go through `execute_batch`.
        connection
            .query_row(pragma, [], |_| Ok(()))
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(()),
                other => Err(write_error(other)),
            })?;
    }
    Ok(connection)
}

fn ensure_durability(connection: &Connection) -> Result<(), WriteError> {
    let mode: String = connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .map_err(write_error)?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(WriteError::Durability(format!(
            "commit mode is {mode}, not the required write-ahead log"
        )));
    }

    let synchronous: i64 = connection
        .query_row("PRAGMA synchronous", [], |row| row.get(0))
        .map_err(write_error)?;
    if synchronous < REQUIRED_SYNCHRONOUS_LEVEL {
        return Err(WriteError::Durability(format!(
            "synchronisation level {synchronous} does not survive power loss; \
             acceptance cannot be published as durable"
        )));
    }

    Ok(())
}

fn table_exists(connection: &Connection, table: &str) -> Result<bool, WriteError> {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1 LIMIT 1",
            params![table],
            |_| Ok(()),
        )
        .optional()
        .map(|found| found.is_some())
        .map_err(write_error)
}

fn column_exists(connection: &Connection, table: &str, column: &str) -> Result<bool, WriteError> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(write_error)?;
    let mut rows = statement.query([]).map_err(write_error)?;
    while let Some(row) = rows.next().map_err(write_error)? {
        let name: String = row.get(1).map_err(write_error)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Sequence to assign and the hash it chains onto.
fn tail(connection: &Connection) -> Result<(u64, Vec<u8>), WriteError> {
    let row: Option<(i64, Vec<u8>)> = connection
        .query_row(
            "SELECT seq, hash FROM contribution ORDER BY seq DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(WriteError::Query)?;

    match row {
        None => Ok((1, Vec::new())),
        Some((sequence, hash)) => {
            let sequence = u64::try_from(sequence)
                .map_err(|_| WriteError::Malformed("sequence is negative"))?;
            Ok((sequence + 1, hash))
        }
    }
}

/// Read back exactly what admission needs about everything this contribution names.
fn resolve(connection: &Connection, envelope: &CanonicalEnvelope) -> Result<Resolved, WriteError> {
    let message_id_exists = reference_facts(connection, envelope.message_id)?.is_some();

    let causation = if envelope.causation_id.is_nil() {
        None
    } else {
        Some(reference_facts(connection, envelope.causation_id)?)
    };

    let mut evidence = Vec::with_capacity(envelope.evidence.len());
    for id in &envelope.evidence {
        evidence.push(reference_facts(connection, *id)?);
    }

    // Asked only when it can matter. A cause that already concluded is a fact about the cause, and
    // a contribution that declares none cannot collide with one.
    let causation_has_outcome = if envelope.causation_id.is_nil() {
        false
    } else {
        connection
            .query_row(
                "SELECT 1 FROM contribution WHERE causation_id = ?1 AND kind = ?2 LIMIT 1",
                params![hyphenated(envelope.causation_id), Kind::Outcome as u16],
                |_| Ok(()),
            )
            .optional()
            .map_err(WriteError::Query)?
            .is_some()
    };

    Ok(Resolved {
        causation,
        evidence,
        causation_has_outcome,
        message_id_exists,
    })
}

fn reference_facts(
    connection: &Connection,
    id: Uuid,
) -> Result<Option<ReferenceFacts>, WriteError> {
    let row: Option<(i64, Option<String>, i64)> = connection
        .query_row(
            "SELECT privacy, retain_until, sensitivity FROM contribution WHERE message_id = ?1",
            params![hyphenated(id)],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(WriteError::Query)?;

    let Some((privacy, retain_until, sensitivity)) = row else {
        return Ok(None);
    };

    let privacy = u8::try_from(privacy)
        .ok()
        .and_then(Privacy::from_u8)
        .ok_or(WriteError::Malformed("stored privacy class is unknown"))?;
    let sensitivity = u8::try_from(sensitivity)
        .ok()
        .and_then(Sensitivity::from_u8)
        .ok_or(WriteError::Malformed("stored sensitivity class is unknown"))?;
    let retain_until_ms = match retain_until.filter(|value| !value.is_empty()) {
        None => 0,
        Some(value) => parse_instant(&value)
            .ok_or(WriteError::Malformed("stored retention time is malformed"))?,
    };

    Ok(Some(ReferenceFacts {
        privacy,
        retain_until_ms,
        sensitivity,
    }))
}

/// An absent text column, spelled the way the predecessor's driver spells one.
fn absent_if_empty(value: &str) -> Option<&str> {
    if value.is_empty() { None } else { Some(value) }
}

/// The `prev_hash` column: NULL at the head of the chain, the stored hash after it.
fn previous_hash_column(previous: &[u8]) -> Option<&[u8]> {
    if previous.is_empty() {
        None
    } else {
        Some(previous)
    }
}

fn hyphenated(id: Uuid) -> String {
    id.hyphenated().to_string()
}

fn optional_uuid(id: Uuid) -> Option<String> {
    if id.is_nil() {
        None
    } else {
        Some(hyphenated(id))
    }
}

fn optional_instant(millis: u64) -> Option<Result<String, ()>> {
    if millis == 0 {
        return None;
    }
    Some(i64::try_from(millis).ok().and_then(qt_instant).ok_or(()))
}

/// The predecessor's `Qt::ISODateWithMs` spelling of a UTC instant.
///
/// Written by hand rather than through a general RFC3339 formatter because the predecessor always
/// emits exactly three subsecond digits, and a formatter that trims trailing zeros would produce a
/// different string for the same instant. That string is stored, and for a legacy v1 row it is
/// hashed, so its exact shape is part of the format rather than a presentation choice.
fn qt_instant(millis: i64) -> Option<String> {
    let instant = OffsetDateTime::from_unix_timestamp_nanos(i128::from(millis) * 1_000_000).ok()?;
    let year = instant.year();
    if !(0..=9999).contains(&year) {
        return None;
    }
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milli:03}Z",
        month = u8::from(instant.month()),
        day = instant.day(),
        hour = instant.hour(),
        minute = instant.minute(),
        second = instant.second(),
        milli = instant.millisecond(),
    ))
}

fn parse_instant(value: &str) -> Option<u64> {
    let instant =
        OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339).ok()?;
    u64::try_from(instant.unix_timestamp_nanos() / 1_000_000).ok()
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

        // The head of the chain links onto nothing, and the predecessor spells that as NULL rather
        // than a zero-length blob. Verification treats the two alike, so only a differential
        // comparison would ever have noticed the difference.
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
        // A whole second still spells its milliseconds; a formatter that trimmed them would
        // produce a different stored string for the same instant.
        assert_eq!(
            qt_instant(1_760_000_000_000).as_deref(),
            Some("2025-10-09T08:53:20.000Z")
        );
    }

    // --- W4: concurrency and rollback -------------------------------------------------------

    #[test]
    fn a_second_writer_is_refused_rather_than_admitted() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("journal.db");
        let holder = JournalWriter::open(&path).expect("holder");
        let mut contender = JournalWriter::open(&path).expect("contender");

        // Hold the write lock exactly as `append` does, then let the other writer try.
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

    // --- W5: the v1 migration ----------------------------------------------------------------

    /// A legacy v1 journal: no versioning columns, evidence as a comma-joined string, hash v1.
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

    /// Append a legacy row, chaining hash v1 the way the predecessor did.
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
            // Hash v1 spells identities with braces, which is the whole reason it is frozen
            // rather than reconstructed from the current spelling.
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
