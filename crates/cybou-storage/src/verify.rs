// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Incremental and bounded Journal verification routines.

use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use crate::inspect::inspect_chain;
use crate::types::{JOURNAL_SCHEMA_V2, JournalCheckpoint, JournalVerification, StorageError};

/// Verify only the Journal suffix after a previously trusted checkpoint.
///
/// # Errors
///
/// Fails closed if the database cannot be opened, the checkpoint does not match the stored row, or
/// any subsequent link, canonical hash, metadata commitment, or live payload commitment differs.
pub fn verify_journal_from(
    path: &Path,
    checkpoint: Option<&JournalCheckpoint>,
) -> Result<JournalVerification, StorageError> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(StorageError::Open)?;
    let schema_version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(StorageError::Query)?;
    if schema_version != JOURNAL_SCHEMA_V2 {
        return Err(StorageError::UnsupportedSchema {
            received: schema_version,
        });
    }
    inspect_chain(&connection, checkpoint, None)
}

/// Verify at most `max_rows` after a trusted checkpoint.
///
/// # Errors
///
/// Fails on a zero page size or under the same fail-closed conditions as [`verify_journal_from`].
pub fn verify_journal_page(
    path: &Path,
    checkpoint: Option<&JournalCheckpoint>,
    max_rows: u64,
) -> Result<JournalVerification, StorageError> {
    if max_rows == 0 {
        return Err(StorageError::InvalidPageSize);
    }
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(StorageError::Open)?;
    let schema_version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(StorageError::Query)?;
    if schema_version != JOURNAL_SCHEMA_V2 {
        return Err(StorageError::UnsupportedSchema {
            received: schema_version,
        });
    }
    inspect_chain(&connection, checkpoint, Some(max_rows))
}
