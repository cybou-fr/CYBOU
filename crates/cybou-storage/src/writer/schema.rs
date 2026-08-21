// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Schema management, DDL definitions, connection setup, and v1-to-v2 migration.

use std::fmt::Write as _;
use std::path::Path;

use cybou_protocol::admission::Kind;
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use uuid::Uuid;

use crate::JOURNAL_SCHEMA_V2;
use crate::writer::error::{REQUIRED_SYNCHRONOUS_LEVEL, WriteError, write_error};

/// The evidence join table, shared by schema creation and the v1 migration.
pub const EVIDENCE_TABLE_DDL: &str = "CREATE TABLE contribution_evidence (
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
#[must_use]
pub fn v2_indexes_ddl() -> String {
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

/// Open an `SQLite` database file with required flags and pragmas.
///
/// # Errors
///
/// Returns [`WriteError::Open`] or [`WriteError::Query`] if opening or applying pragmas fails.
pub fn open_for_write(path: &Path) -> Result<Connection, WriteError> {
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
        connection
            .query_row(pragma, [], |_| Ok(()))
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(()),
                other => Err(write_error(other)),
            })?;
    }
    Ok(connection)
}

/// Verify that the connection is running in WAL mode with sufficient synchronisation level.
///
/// # Errors
///
/// Returns [`WriteError::Durability`] if synchronous mode or WAL is insufficient.
pub fn ensure_durability(connection: &Connection) -> Result<(), WriteError> {
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

/// Check if a given table exists in the database.
///
/// # Errors
///
/// Returns [`WriteError`] on database query failure.
pub fn table_exists(connection: &Connection, table: &str) -> Result<bool, WriteError> {
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

/// Check if a given column exists in a table.
///
/// # Errors
///
/// Returns [`WriteError`] on database query failure.
pub fn column_exists(connection: &Connection, table: &str, column: &str) -> Result<bool, WriteError> {
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

/// Migrate a v1 Journal to schema v2, or refuse and leave it untouched.
///
/// # Errors
///
/// Returns [`WriteError::Migration`] if migration verification fails or schema is incompatible.
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
            let evidence_id = parsed.hyphenated().to_string();
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
