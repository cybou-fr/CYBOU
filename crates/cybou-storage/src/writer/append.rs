// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Contribution appending, hashing, admission resolution, and database insertion pipeline.

use cybou_protocol::admission::{
    self, Kind, Privacy, ReferenceFacts, Resolved, Sensitivity,
};
use cybou_protocol::canonical::{
    CanonicalEnvelope, canonical_journal_row_v3, commitment_v3, sha256,
};
use rusqlite::{Connection, OptionalExtension, params};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::writer::error::{JOURNAL_HASH_V3, WriteError};

/// One appended contribution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Appended {
    /// Sequence assigned by the chain, starting at one.
    pub sequence: u64,
    /// Row hash written at [`JOURNAL_HASH_V3`].
    pub hash: [u8; 32],
}

/// What one erasure actually reached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Erased {
    /// Everything the erasure applied to, including the target itself.
    pub closure: Vec<Uuid>,
    /// Those whose payload was actually redacted by this call.
    pub redacted: Vec<Uuid>,
    /// The erasure epoch the Journal now stands at.
    pub epoch: u64,
}

/// Validate, hash, chain and insert one contribution inside a transaction the caller opened.
///
/// # Errors
///
/// Returns [`WriteError`] if admission check fails or database insertion encounters an error.
pub fn append_within_transaction(
    transaction: &Connection,
    envelope: &CanonicalEnvelope,
) -> Result<Appended, WriteError> {
    let resolved = resolve(transaction, envelope)?;
    admission::check_admission(envelope, &resolved)?;

    let (sequence, previous_hash) = tail(transaction)?;
    let (_, payload_commitment, commitment) = commitment_v3(envelope);
    let hash = sha256(&canonical_journal_row_v3(
        sequence,
        &previous_hash,
        &commitment,
    ));

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
                None::<String>,
                envelope.payload,
                i64::from(envelope.privacy),
                absent_if_empty(&envelope.capability_scope),
                i64::from(envelope.schema_version),
                JOURNAL_HASH_V3,
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

/// Read back exactly what admission needs about everything this contribution names.
///
/// # Errors
///
/// Returns [`WriteError`] on database query failure.
pub fn resolve(connection: &Connection, envelope: &CanonicalEnvelope) -> Result<Resolved, WriteError> {
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

/// Sequence to assign and the hash it chains onto.
///
/// # Errors
///
/// Returns [`WriteError`] on database query failure.
pub fn tail(connection: &Connection) -> Result<(u64, Vec<u8>), WriteError> {
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

/// Look up reference facts (privacy, retention, sensitivity) for a named message.
///
/// # Errors
///
/// Returns [`WriteError`] on database query failure or malformed fields.
pub fn reference_facts(
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

/// Parse an RFC3339 timestamp string into epoch milliseconds.
#[must_use]
pub fn parse_instant(value: &str) -> Option<u64> {
    let instant =
        OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339).ok()?;
    u64::try_from(instant.unix_timestamp_nanos() / 1_000_000).ok()
}

/// An absent text column, spelled the way the predecessor's driver spells one.
#[must_use]
pub fn absent_if_empty(value: &str) -> Option<&str> {
    if value.is_empty() { None } else { Some(value) }
}

/// The `prev_hash` column: NULL at the head of the chain, the stored hash after it.
#[must_use]
pub fn previous_hash_column(previous: &[u8]) -> Option<&[u8]> {
    if previous.is_empty() {
        None
    } else {
        Some(previous)
    }
}

/// Hyphenated string representation of a UUID.
#[must_use]
pub fn hyphenated(id: Uuid) -> String {
    id.hyphenated().to_string()
}

/// Format an optional non-nil UUID as hyphenated string.
#[must_use]
pub fn optional_uuid(id: Uuid) -> Option<String> {
    if id.is_nil() {
        None
    } else {
        Some(hyphenated(id))
    }
}

/// Format an optional epoch millisecond instant into Qt ISO string.
pub fn optional_instant(millis: u64) -> Option<Result<String, ()>> {
    if millis == 0 {
        return None;
    }
    Some(i64::try_from(millis).ok().and_then(qt_instant).ok_or(()))
}

/// The predecessor's `Qt::ISODateWithMs` spelling of a UTC instant.
#[must_use]
pub fn qt_instant(millis: i64) -> Option<String> {
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
