// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Fail-closed, read-only inspection of predecessor Journal databases.

use std::path::Path;

use rusqlite::{Connection, OpenFlags};
use thiserror::Error;

/// Current predecessor `SQLite` schema version.
pub const JOURNAL_SCHEMA_V2: i64 = 2;

const REQUIRED_TABLES: &[&str] = &["contribution", "contribution_evidence", "journal_meta"];
const REQUIRED_CONTRIBUTION_COLUMNS: &[&str] = &[
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
}

/// Open an existing database strictly read-only and verify its v2 structural boundary.
///
/// # Errors
///
/// Fails without creating or migrating anything when the file, version, tables, columns, metadata,
/// or counters do not match the accepted predecessor contract.
pub fn inspect_journal(path: &Path) -> Result<JournalInspection, StorageError> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(StorageError::Open)?;
    let schema_version = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(StorageError::Query)?;
    if schema_version != JOURNAL_SCHEMA_V2 {
        return Err(StorageError::UnsupportedSchema {
            received: schema_version,
        });
    }
    for table in REQUIRED_TABLES {
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                [table],
                |row| row.get(0),
            )
            .map_err(StorageError::Query)?;
        if !exists {
            return Err(StorageError::MissingSchema((*table).into()));
        }
    }
    let mut columns = connection
        .prepare("PRAGMA table_info(contribution)")
        .map_err(StorageError::Query)?;
    let present = columns
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(StorageError::Query)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(StorageError::Query)?;
    for column in REQUIRED_CONTRIBUTION_COLUMNS {
        if !present.iter().any(|candidate| candidate == column) {
            return Err(StorageError::MissingSchema(format!(
                "contribution.{column}"
            )));
        }
    }
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM contribution", [], |row| row.get(0))
        .map_err(StorageError::Query)?;
    inspect_chain_shape(&connection)?;
    let (erasure_epoch, rotated_epoch): (i64, i64) = connection
        .query_row(
            "SELECT erasure_epoch, rotated_epoch FROM journal_meta WHERE id=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(StorageError::Query)?;
    Ok(JournalInspection {
        schema_version,
        contribution_count: u64::try_from(count).map_err(|_| StorageError::InvalidCounter)?,
        erasure_epoch: u64::try_from(erasure_epoch).map_err(|_| StorageError::InvalidCounter)?,
        rotated_epoch: u64::try_from(rotated_epoch).map_err(|_| StorageError::InvalidCounter)?,
    })
}

fn inspect_chain_shape(connection: &Connection) -> Result<(), StorageError> {
    let mut statement = connection
        .prepare(
            "SELECT seq, hash_version, prev_hash, hash, commitment, payload_commitment \
             FROM contribution ORDER BY seq ASC",
        )
        .map_err(StorageError::Query)?;
    let mut rows = statement.query([]).map_err(StorageError::Query)?;
    let mut expected_sequence = 1_u64;
    let mut previous_hash = Vec::new();
    while let Some(row) = rows.next().map_err(StorageError::Query)? {
        let raw_sequence: i64 = row.get(0).map_err(StorageError::Query)?;
        let sequence = u64::try_from(raw_sequence).map_err(|_| StorageError::InvalidCounter)?;
        let hash_version: i64 = row.get(1).map_err(StorageError::Query)?;
        let stored_previous: Option<Vec<u8>> = row.get(2).map_err(StorageError::Query)?;
        let hash: Vec<u8> = row.get(3).map_err(StorageError::Query)?;
        let commitment: Option<Vec<u8>> = row.get(4).map_err(StorageError::Query)?;
        let payload_commitment: Option<Vec<u8>> = row.get(5).map_err(StorageError::Query)?;
        let invalid = |reason| StorageError::InvalidChain { sequence, reason };
        if sequence != expected_sequence {
            return Err(invalid("sequence is not contiguous"));
        }
        if stored_previous.unwrap_or_default() != previous_hash {
            return Err(invalid("previous hash does not match the preceding row"));
        }
        if hash.len() != 32 {
            return Err(invalid("row hash is not SHA-256 sized"));
        }
        if !(1..=3).contains(&hash_version) {
            return Err(invalid("hash version is unsupported"));
        }
        if hash_version == 3
            && (commitment.as_deref().is_none_or(|value| value.len() != 32)
                || payload_commitment
                    .as_deref()
                    .is_none_or(|value| value.len() != 32))
        {
            return Err(invalid("v3 commitment material is missing or malformed"));
        }
        previous_hash = hash;
        expected_sequence = expected_sequence.saturating_add(1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::{JOURNAL_SCHEMA_V2, StorageError, inspect_journal};

    const SCHEMA: &str = "
        CREATE TABLE contribution (
          seq INTEGER PRIMARY KEY, message_id TEXT, correlation_id TEXT, causation_id TEXT,
          origin_organ TEXT, origin_node TEXT, kind INTEGER, wall_time TEXT,
          monotonic_time INTEGER, logical_clock INTEGER, confidence REAL, evidence TEXT, payload BLOB,
          privacy INTEGER, capability TEXT, schema_version INTEGER, hash_version INTEGER,
          prev_hash BLOB, hash BLOB, commitment BLOB, payload_commitment BLOB,
          erased_at TEXT, sealed INTEGER, key_domain TEXT, key_epoch INTEGER,
          retention_class INTEGER, retention_policy INTEGER, retain_until TEXT, sensitivity INTEGER
        );
        CREATE TABLE contribution_evidence (contribution_id TEXT, evidence_id TEXT, ordinal INTEGER);
        CREATE TABLE journal_meta (id INTEGER PRIMARY KEY, erasure_epoch INTEGER, rotated_epoch INTEGER);
        INSERT INTO journal_meta VALUES (1, 4, 3);
        INSERT INTO contribution
          (seq, message_id, correlation_id, origin_organ, origin_node, kind, wall_time,
           monotonic_time, logical_clock, confidence, privacy, schema_version, hash_version,
           prev_hash, hash, commitment, payload_commitment, sealed, key_epoch,
           retention_class, retention_policy, sensitivity)
        VALUES
          (1, '00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000001',
           'fixture', '', 1, '2026-08-19T00:00:00.000Z', 0, 1, 1.0, 0, 4, 3,
           X'', zeroblob(32), zeroblob(32), zeroblob(32), 0, 0, 2, 0, 1),
          (2, '00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000002',
           'fixture', '', 1, '2026-08-19T00:00:01.000Z', 0, 2, 1.0, 0, 4, 3,
           zeroblob(32), X'0101010101010101010101010101010101010101010101010101010101010101',
           zeroblob(32), zeroblob(32), 0, 0, 2, 0, 1);
        PRAGMA user_version=2;
    ";

    #[test]
    fn missing_database_is_not_created() {
        let root = tempdir().expect("temporary root");
        let path = root.path().join("missing.db");
        assert!(matches!(inspect_journal(&path), Err(StorageError::Open(_))));
        assert!(!path.exists());
    }

    #[test]
    fn v2_database_is_inspected_without_writing_it() {
        let root = tempdir().expect("temporary root");
        let path = root.path().join("journal.db");
        let connection = Connection::open(&path).expect("fixture database");
        connection.execute_batch(SCHEMA).expect("fixture schema");
        drop(connection);
        let before = fs::read(&path).expect("database bytes");

        let inspection = inspect_journal(&path).expect("compatible journal");

        assert_eq!(inspection.schema_version, JOURNAL_SCHEMA_V2);
        assert_eq!(inspection.contribution_count, 2);
        assert_eq!(inspection.erasure_epoch, 4);
        assert_eq!(inspection.rotated_epoch, 3);
        assert_eq!(fs::read(path).expect("database bytes"), before);
    }

    #[test]
    fn future_or_partial_schema_fails_closed() {
        let root = tempdir().expect("temporary root");
        let future = root.path().join("future.db");
        let connection = Connection::open(&future).expect("future database");
        connection
            .execute_batch("CREATE TABLE contribution(seq INTEGER); PRAGMA user_version=3;")
            .expect("future schema");
        drop(connection);
        assert!(matches!(
            inspect_journal(&future),
            Err(StorageError::UnsupportedSchema { received: 3 })
        ));

        let partial = root.path().join("partial.db");
        let connection = Connection::open(&partial).expect("partial database");
        connection
            .execute_batch("CREATE TABLE contribution(seq INTEGER); PRAGMA user_version=2;")
            .expect("partial schema");
        drop(connection);
        assert!(matches!(
            inspect_journal(&partial),
            Err(StorageError::MissingSchema(_))
        ));
    }

    #[test]
    fn broken_previous_hash_fails_closed() {
        let root = tempdir().expect("temporary root");
        let path = root.path().join("broken.db");
        let connection = Connection::open(&path).expect("fixture database");
        connection.execute_batch(SCHEMA).expect("fixture schema");
        connection
            .execute(
                "UPDATE contribution SET prev_hash=zeroblob(31) WHERE seq=2",
                [],
            )
            .expect("break chain link");
        drop(connection);
        assert!(matches!(
            inspect_journal(&path),
            Err(StorageError::InvalidChain { sequence: 2, .. })
        ));
    }
}
