// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Read-only inspection of `SQLite` Journal v2 schema and hash chain validation.

use std::path::Path;

use cybou_protocol::canonical::{
    CanonicalEnvelope, canonical_journal_row_v2, canonical_journal_row_v3,
    canonical_nonerasable_v3, sha256,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::types::{
    JOURNAL_SCHEMA_V2, JournalCheckpoint, JournalInspection, JournalVerification,
    REQUIRED_CONTRIBUTION_COLUMNS, REQUIRED_TABLES, StorageError,
};

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

/// Inspect `SQLite` Journal hash chain after a checkpoint.
///
/// # Errors
///
/// Returns [`StorageError`] if query execution, hash chain verification, or commitments fail.
pub fn inspect_chain(
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

/// Decode a canonical envelope from an `SQLite` contribution row and its evidence relations.
///
/// # Errors
///
/// Returns [`StorageError`] if row fields, types, or UUIDs are malformed.
pub fn decode_envelope(
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

/// Parse RFC 3339 timestamp string into milliseconds since UNIX epoch.
#[must_use]
pub fn parse_millis(value: &str) -> Option<i64> {
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
