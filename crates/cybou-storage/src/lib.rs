// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Fail-closed, read-only inspection of predecessor Journal databases.

use std::path::Path;

pub mod writer;

use cybou_protocol::canonical::{
    CanonicalEnvelope, canonical_journal_row_v2, canonical_journal_row_v3,
    canonical_nonerasable_v3, sha256,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use thiserror::Error;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

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

/// Trusted chain position from a previous successful verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JournalCheckpoint {
    /// Last verified sequence, or zero before the first row.
    pub sequence: u64,
    /// Stored hash at `sequence`, empty only when `sequence` is zero.
    pub hash: Vec<u8>,
}

/// Bounded verification facts for the suffix after a checkpoint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JournalVerification {
    /// Trusted checkpoint sequence supplied by the caller.
    pub verified_from: u64,
    /// Last sequence cryptographically replayed by this call.
    pub verified_through: u64,
    /// V3 payloads whose bytes were still present and matched their commitment.
    pub content_verified: u64,
    /// Erased v3 payloads skipped while their metadata remained verified.
    pub content_skipped: u64,
    /// Whether rows remain after this bounded page.
    pub has_more: bool,
    /// Checkpoint suitable for the next incremental verification.
    pub checkpoint: JournalCheckpoint,
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
    /// A supplied checkpoint no longer names the same stored row.
    #[error("Journal checkpoint does not match sequence {sequence}")]
    CheckpointMismatch {
        /// Sequence whose stored hash differs or no longer exists.
        sequence: u64,
    },
    /// A paged replay must always make forward progress.
    #[error("Journal verification page size must be greater than zero")]
    InvalidPageSize,
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
    inspect_chain(&connection, None, None)?;
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

fn inspect_chain(
    connection: &Connection,
    checkpoint: Option<&JournalCheckpoint>,
    max_rows: Option<u64>,
) -> Result<JournalVerification, StorageError> {
    let (start_after, mut previous_hash) = anchor_state(connection, checkpoint)?;
    let mut statement = connection
        .prepare(
            "SELECT seq, hash_version, schema_version, message_id, correlation_id, causation_id, \
             origin_organ, origin_node, kind, wall_time, monotonic_time, logical_clock, confidence, \
             payload, privacy, capability, sealed, key_domain, key_epoch, retention_class, \
             retention_policy, retain_until, sensitivity, prev_hash, hash, commitment, \
             payload_commitment, erased_at \
             FROM contribution WHERE seq > ?1 ORDER BY seq ASC LIMIT ?2",
        )
        .map_err(StorageError::Query)?;
    let row_limit = max_rows.map_or(i64::MAX, |value| i64::try_from(value).unwrap_or(i64::MAX));
    let mut rows = statement
        .query(rusqlite::params![
            i64::try_from(start_after).map_err(|_| StorageError::InvalidCounter)?,
            row_limit
        ])
        .map_err(StorageError::Query)?;
    let mut expected_sequence = start_after.saturating_add(1);
    let mut content_verified = 0_u64;
    let mut content_skipped = 0_u64;
    while let Some(row) = rows.next().map_err(StorageError::Query)? {
        let raw_sequence: i64 = row.get(0).map_err(StorageError::Query)?;
        let sequence = u64::try_from(raw_sequence).map_err(|_| StorageError::InvalidCounter)?;
        let hash_version: i64 = row.get(1).map_err(StorageError::Query)?;
        let stored_previous: Option<Vec<u8>> = row.get(23).map_err(StorageError::Query)?;
        let hash: Vec<u8> = row.get(24).map_err(StorageError::Query)?;
        let commitment: Option<Vec<u8>> = row.get(25).map_err(StorageError::Query)?;
        let payload_commitment: Option<Vec<u8>> = row.get(26).map_err(StorageError::Query)?;
        let erased_at: Option<String> = row.get(27).map_err(StorageError::Query)?;
        let invalid = |reason| StorageError::InvalidChain { sequence, reason };
        if sequence != expected_sequence {
            return Err(invalid("sequence is not contiguous"));
        }
        if stored_previous.as_deref().unwrap_or_default() != previous_hash {
            return Err(invalid("previous hash does not match the preceding row"));
        }
        if hash.len() != 32 {
            return Err(invalid("row hash is not SHA-256 sized"));
        }
        if !(1..=3).contains(&hash_version) {
            return Err(invalid("hash version is unsupported"));
        }
        let envelope = decode_envelope(connection, row, sequence)?;
        let previous = stored_previous.unwrap_or_default();
        let expected = match hash_version {
            1 => legacy_row_hash(
                sequence,
                &previous,
                &envelope,
                &row.get::<_, String>(9).map_err(StorageError::Query)?,
            ),
            2 if envelope.schema_version == 2 => {
                sha256(&canonical_journal_row_v2(sequence, &previous, &envelope))
            }
            3 if (2..=4).contains(&envelope.schema_version) => {
                let commitment = commitment
                    .as_deref()
                    .filter(|value| value.len() == 32)
                    .ok_or_else(|| invalid("v3 commitment is missing or malformed"))?;
                let payload_commitment = payload_commitment
                    .as_deref()
                    .filter(|value| value.len() == 32)
                    .ok_or_else(|| invalid("v3 payload commitment is missing or malformed"))?;
                let metadata = sha256(&canonical_nonerasable_v3(&envelope));
                let mut joined = [0_u8; 64];
                joined[..32].copy_from_slice(&metadata);
                joined[32..].copy_from_slice(payload_commitment);
                if sha256(&joined).as_slice() != commitment {
                    return Err(invalid("v3 metadata commitment does not match"));
                }
                if erased_at.is_none() && sha256(&envelope.payload).as_slice() != payload_commitment
                {
                    return Err(invalid("v3 payload commitment does not match"));
                }
                if erased_at.is_some() {
                    content_skipped = content_skipped.saturating_add(1);
                } else {
                    content_verified = content_verified.saturating_add(1);
                }
                sha256(&canonical_journal_row_v3(sequence, &previous, commitment))
            }
            _ => return Err(invalid("hash and envelope versions are incompatible")),
        };
        if expected.as_slice() != hash {
            return Err(invalid("stored hash does not match canonical row"));
        }
        previous_hash = hash;
        expected_sequence = expected_sequence.saturating_add(1);
    }
    let verified_through = expected_sequence.saturating_sub(1);
    finish_verification(
        connection,
        start_after,
        verified_through,
        previous_hash,
        content_verified,
        content_skipped,
    )
}

fn finish_verification(
    connection: &Connection,
    verified_from: u64,
    verified_through: u64,
    hash: Vec<u8>,
    content_verified: u64,
    content_skipped: u64,
) -> Result<JournalVerification, StorageError> {
    let has_more: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM contribution WHERE seq > ?1)",
            [i64::try_from(verified_through).map_err(|_| StorageError::InvalidCounter)?],
            |row| row.get(0),
        )
        .map_err(StorageError::Query)?;
    Ok(JournalVerification {
        verified_from,
        verified_through,
        content_verified,
        content_skipped,
        has_more,
        checkpoint: JournalCheckpoint {
            sequence: verified_through,
            hash,
        },
    })
}

fn anchor_state(
    connection: &Connection,
    checkpoint: Option<&JournalCheckpoint>,
) -> Result<(u64, Vec<u8>), StorageError> {
    let Some(anchor) = checkpoint else {
        return Ok((0, Vec::new()));
    };
    if anchor.sequence == 0 {
        return if anchor.hash.is_empty() {
            Ok((0, Vec::new()))
        } else {
            Err(StorageError::CheckpointMismatch { sequence: 0 })
        };
    }
    let stored: Option<Vec<u8>> = connection
        .query_row(
            "SELECT hash FROM contribution WHERE seq=?1",
            [i64::try_from(anchor.sequence).map_err(|_| StorageError::InvalidCounter)?],
            |row| row.get(0),
        )
        .optional()
        .map_err(StorageError::Query)?;
    if stored.as_deref() != Some(anchor.hash.as_slice()) {
        return Err(StorageError::CheckpointMismatch {
            sequence: anchor.sequence,
        });
    }
    Ok((anchor.sequence, anchor.hash.clone()))
}

fn decode_envelope(
    connection: &Connection,
    row: &rusqlite::Row<'_>,
    sequence: u64,
) -> Result<CanonicalEnvelope, StorageError> {
    let invalid = |reason| StorageError::InvalidChain { sequence, reason };
    let integer =
        |index| -> Result<i64, StorageError> { row.get(index).map_err(StorageError::Query) };
    let text =
        |index| -> Result<String, StorageError> { row.get(index).map_err(StorageError::Query) };
    let optional_text = |index| -> Result<Option<String>, StorageError> {
        row.get(index).map_err(StorageError::Query)
    };
    let uuid = |index| -> Result<Uuid, StorageError> {
        optional_text(index)?
            .filter(|value| !value.is_empty())
            .map_or(Ok(Uuid::nil()), |value| {
                Uuid::parse_str(&value).map_err(|_| invalid("UUID is malformed"))
            })
    };
    let non_negative = |index| -> Result<u64, StorageError> {
        u64::try_from(integer(index)?).map_err(|_| invalid("numeric field is negative"))
    };
    let wall_time = text(9)?;
    let wall_time_ms = parse_millis(&wall_time).ok_or_else(|| invalid("wall time is malformed"))?;
    let retain_until = optional_text(21)?;
    let message_id = uuid(3)?;
    let mut evidence_statement = connection
        .prepare(
            "SELECT evidence_id FROM contribution_evidence \
             WHERE contribution_id=?1 ORDER BY ordinal",
        )
        .map_err(StorageError::Query)?;
    let evidence = evidence_statement
        .query_map([message_id.hyphenated().to_string()], |evidence_row| {
            evidence_row.get::<_, String>(0)
        })
        .map_err(StorageError::Query)?
        .map(|value| {
            value.map_err(StorageError::Query).and_then(|value| {
                Uuid::parse_str(&value).map_err(|_| invalid("evidence UUID is malformed"))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CanonicalEnvelope {
        schema_version: u16::try_from(integer(2)?)
            .map_err(|_| invalid("schema version is invalid"))?,
        message_id,
        correlation_id: uuid(4)?,
        causation_id: uuid(5)?,
        origin_organ: text(6)?,
        origin_node: text(7)?,
        kind: u16::try_from(integer(8)?).map_err(|_| invalid("kind is invalid"))?,
        wall_time_ms,
        monotonic_time: non_negative(10)?,
        logical_clock: non_negative(11)?,
        confidence: row.get(12).map_err(StorageError::Query)?,
        evidence,
        payload: row
            .get::<_, Option<Vec<u8>>>(13)
            .map_err(StorageError::Query)?
            .unwrap_or_default(),
        privacy: u8::try_from(integer(14)?).map_err(|_| invalid("privacy is invalid"))?,
        capability_scope: optional_text(15)?.unwrap_or_default(),
        sealed: integer(16)? != 0,
        key_domain_id: uuid(17)?,
        key_epoch: u32::try_from(integer(18)?).map_err(|_| invalid("key epoch is invalid"))?,
        retention_class: u8::try_from(integer(19)?)
            .map_err(|_| invalid("retention class is invalid"))?,
        retention_policy_version: u16::try_from(integer(20)?)
            .map_err(|_| invalid("retention policy is invalid"))?,
        retain_until_ms: retain_until
            .filter(|value| !value.is_empty())
            .map_or(Ok(0), |value| {
                parse_millis(&value)
                    .and_then(|value| u64::try_from(value).ok())
                    .ok_or_else(|| invalid("retention time is malformed"))
            })?,
        sensitivity: u8::try_from(integer(22)?).map_err(|_| invalid("sensitivity is invalid"))?,
    })
}

fn parse_millis(value: &str) -> Option<i64> {
    let instant = OffsetDateTime::parse(value, &Rfc3339).ok()?;
    i64::try_from(instant.unix_timestamp_nanos() / 1_000_000).ok()
}

fn legacy_row_hash(
    sequence: u64,
    previous: &[u8],
    envelope: &CanonicalEnvelope,
    wall_time: &str,
) -> [u8; 32] {
    let mut input = previous.to_vec();
    input.extend_from_slice(sequence.to_string().as_bytes());
    for id in [
        envelope.message_id,
        envelope.correlation_id,
        envelope.causation_id,
    ] {
        input.extend_from_slice(format!("{{{id}}}").as_bytes());
    }
    input.extend_from_slice(envelope.origin_organ.as_bytes());
    input.extend_from_slice(envelope.kind.to_string().as_bytes());
    input.extend_from_slice(wall_time.as_bytes());
    input.extend_from_slice(envelope.logical_clock.to_string().as_bytes());
    input.extend_from_slice(&envelope.payload);
    sha256(&input)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::Connection;
    use tempfile::tempdir;

    use super::{
        JOURNAL_SCHEMA_V2, JournalCheckpoint, StorageError, inspect_journal, verify_journal_from,
        verify_journal_page,
    };

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

    fn populate_valid_chain(connection: &Connection) {
        populate_chain(connection, 2);
    }

    fn populate_chain(connection: &Connection, row_count: u64) {
        use cybou_protocol::canonical::{
            CanonicalEnvelope, canonical_journal_row_v3, commitment_v3, sha256,
        };
        use rusqlite::params;
        use uuid::Uuid;

        let mut previous = Vec::new();
        for sequence in 1_u64..=row_count {
            let id = Uuid::from_u128(sequence.into());
            let wall_time = "2026-08-19T00:00:00.000Z".to_owned();
            let envelope = CanonicalEnvelope {
                schema_version: 4,
                message_id: id,
                correlation_id: id,
                causation_id: Uuid::nil(),
                origin_organ: "fixture".into(),
                origin_node: String::new(),
                kind: 1,
                wall_time_ms: super::parse_millis(&wall_time).expect("fixture time"),
                monotonic_time: 0,
                logical_clock: sequence,
                confidence: 1.0,
                evidence: Vec::new(),
                payload: sequence.to_be_bytes().to_vec(),
                privacy: 0,
                capability_scope: String::new(),
                sealed: false,
                key_domain_id: Uuid::nil(),
                key_epoch: 0,
                retention_class: 2,
                retention_policy_version: 0,
                retain_until_ms: 0,
                sensitivity: 1,
            };
            let (_, payload, commitment) = commitment_v3(&envelope);
            let hash = sha256(&canonical_journal_row_v3(sequence, &previous, &commitment));
            let stored_sequence = i64::try_from(sequence).expect("fixture sequence");
            connection
                .execute(
                    "INSERT INTO contribution (
                       seq, message_id, correlation_id, origin_organ, origin_node, kind, wall_time,
                       monotonic_time, logical_clock, confidence, payload, privacy, capability,
                       schema_version, hash_version, prev_hash, hash, commitment,
                       payload_commitment, sealed, key_epoch, retention_class, retention_policy,
                       sensitivity
                     ) VALUES (
                       ?1, ?2, ?2, 'fixture', '', 1, ?3, 0, ?1, 1.0, ?8, 0, '', 4, 3,
                       ?4, ?5, ?6, ?7, 0, 0, 2, 0, 1
                     )",
                    params![
                        stored_sequence,
                        id.hyphenated().to_string(),
                        wall_time,
                        previous,
                        hash,
                        commitment,
                        payload,
                        envelope.payload
                    ],
                )
                .expect("valid contribution");
            previous = hash.to_vec();
        }
    }

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
        populate_valid_chain(&connection);
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
        populate_valid_chain(&connection);
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

    #[test]
    fn canonical_hash_and_live_payload_tampering_fail_closed() {
        let root = tempdir().expect("temporary root");
        let hash_path = root.path().join("hash.db");
        let connection = Connection::open(&hash_path).expect("fixture database");
        connection.execute_batch(SCHEMA).expect("fixture schema");
        populate_valid_chain(&connection);
        connection
            .execute("UPDATE contribution SET hash=zeroblob(32) WHERE seq=1", [])
            .expect("break canonical hash");
        drop(connection);
        assert!(matches!(
            inspect_journal(&hash_path),
            Err(StorageError::InvalidChain { sequence: 1, .. })
        ));

        let payload_path = root.path().join("payload.db");
        let connection = Connection::open(&payload_path).expect("fixture database");
        connection.execute_batch(SCHEMA).expect("fixture schema");
        populate_valid_chain(&connection);
        connection
            .execute("UPDATE contribution SET payload=X'ff' WHERE seq=2", [])
            .expect("break payload commitment");
        drop(connection);
        assert!(matches!(
            inspect_journal(&payload_path),
            Err(StorageError::InvalidChain { sequence: 2, .. })
        ));
    }

    #[test]
    fn erased_payload_is_skipped_but_surviving_metadata_is_verified() {
        let root = tempdir().expect("temporary root");
        let path = root.path().join("erased.db");
        let connection = Connection::open(&path).expect("fixture database");
        connection.execute_batch(SCHEMA).expect("fixture schema");
        populate_valid_chain(&connection);
        connection
            .execute(
                "UPDATE contribution SET payload=X'', erased_at='2026-08-20T00:00:00.000Z' \
                 WHERE seq=2",
                [],
            )
            .expect("erase payload bytes");
        drop(connection);
        assert_eq!(
            inspect_journal(&path)
                .expect("verifiable erasure")
                .contribution_count,
            2
        );
    }

    #[test]
    fn checkpoint_verifies_only_the_suffix_and_refuses_a_stale_anchor() {
        let root = tempdir().expect("temporary root");
        let path = root.path().join("checkpoint.db");
        let connection = Connection::open(&path).expect("fixture database");
        connection.execute_batch(SCHEMA).expect("fixture schema");
        populate_valid_chain(&connection);
        let first_hash: Vec<u8> = connection
            .query_row("SELECT hash FROM contribution WHERE seq=1", [], |row| {
                row.get(0)
            })
            .expect("first hash");
        drop(connection);

        let full = verify_journal_from(&path, None).expect("full verification");
        assert_eq!(full.verified_from, 0);
        assert_eq!(full.verified_through, 2);
        assert_eq!(full.content_verified, 2);
        assert!(!full.has_more);

        let first_page = verify_journal_page(&path, None, 1).expect("first page");
        assert_eq!(first_page.verified_through, 1);
        assert_eq!(first_page.content_verified, 1);
        assert!(first_page.has_more);
        let second_page =
            verify_journal_page(&path, Some(&first_page.checkpoint), 1).expect("second page");
        assert_eq!(second_page.verified_from, 1);
        assert_eq!(second_page.verified_through, 2);
        assert_eq!(second_page.content_verified, 1);
        assert!(!second_page.has_more);
        assert!(matches!(
            verify_journal_page(&path, None, 0),
            Err(StorageError::InvalidPageSize)
        ));

        let suffix = verify_journal_from(
            &path,
            Some(&JournalCheckpoint {
                sequence: 1,
                hash: first_hash,
            }),
        )
        .expect("suffix verification");
        assert_eq!(suffix.verified_from, 1);
        assert_eq!(suffix.verified_through, 2);
        assert_eq!(suffix.content_verified, 1);

        let at_head = verify_journal_from(&path, Some(&full.checkpoint)).expect("head checkpoint");
        assert_eq!(at_head.verified_from, 2);
        assert_eq!(at_head.verified_through, 2);
        assert_eq!(at_head.content_verified, 0);

        let stale = JournalCheckpoint {
            sequence: 2,
            hash: vec![0; 32],
        };
        assert!(matches!(
            verify_journal_from(&path, Some(&stale)),
            Err(StorageError::CheckpointMismatch { sequence: 2 })
        ));
    }

    #[test]
    fn paged_replay_respects_its_row_budget_across_a_larger_chain() {
        const ROWS: u64 = 513;
        const PAGE_SIZE: u64 = 64;

        let root = tempdir().expect("temporary root");
        let path = root.path().join("scale.db");
        let connection = Connection::open(&path).expect("fixture database");
        connection.execute_batch(SCHEMA).expect("fixture schema");
        populate_chain(&connection, ROWS);
        drop(connection);

        let full = verify_journal_from(&path, None).expect("full verification");
        let mut checkpoint = None;
        let mut verified = 0_u64;
        let mut pages = 0_u64;
        loop {
            let page = verify_journal_page(&path, checkpoint.as_ref(), PAGE_SIZE)
                .expect("bounded verification page");
            let page_rows = page.verified_through - page.verified_from;
            assert!(page_rows <= PAGE_SIZE);
            assert_eq!(page.content_verified, page_rows);
            verified += page_rows;
            pages += 1;
            checkpoint = Some(page.checkpoint);
            if !page.has_more {
                break;
            }
        }
        assert_eq!(verified, ROWS);
        assert_eq!(pages, ROWS.div_ceil(PAGE_SIZE));
        assert_eq!(checkpoint, Some(full.checkpoint));
    }
}
