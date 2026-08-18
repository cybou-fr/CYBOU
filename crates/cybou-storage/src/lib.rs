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
        assert_eq!(inspection.contribution_count, 0);
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
}
