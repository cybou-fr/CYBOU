// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Write errors and `SQLite` error classification.

use cybou_protocol::admission::Rejection;
use crate::StorageError;

/// Journal hash version written by this build.
pub const JOURNAL_HASH_V3: i64 = 3;

/// `SQLite` `synchronous` level at or above which a returned `COMMIT` has reached storage.
pub const REQUIRED_SYNCHRONOUS_LEVEL: i64 = 2;

/// Whether a failure is another writer holding the database rather than a defective statement.
#[must_use]
pub fn is_busy(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(inner, _)
            if inner.code == rusqlite::ErrorCode::DatabaseBusy
                || inner.code == rusqlite::ErrorCode::DatabaseLocked
    )
}

/// Classify rusqlite error into [`WriteError`].
#[must_use]
pub fn write_error(error: rusqlite::Error) -> WriteError {
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
