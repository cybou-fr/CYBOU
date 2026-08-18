// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Byte-compatible Observation v1 payload and deterministic acquisition identity.

use ciborium::Value;
use thiserror::Error;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

/// Only accepted Observation payload schema.
pub const OBSERVATION_SCHEMA_V1: u16 = 1;
/// Discriminator separating epistemic observations from other Observation-kind contributions.
pub const OBSERVATION_PAYLOAD_TYPE: &str = "cybou.observation.v1";
const OBSERVATION_NAMESPACE: Uuid = Uuid::from_u128(0x9f2c1d84_6b3a_5e07_bc41_0d2a7f9e5c13);

/// Structurally valid Observation v1 payload.
#[derive(Clone, Debug, PartialEq)]
pub struct ObservationV1 {
    /// Stable source identity, independent of the adapter process.
    pub source_id: String,
    /// Comparable subject key.
    pub subject: String,
    /// Typed observed value; null is forbidden.
    pub value: Value,
    /// UTC ISO-8601 acquisition time with milliseconds.
    pub acquired_at: String,
    /// Exclusive UTC freshness horizon with milliseconds.
    pub freshness_until: String,
    /// Re-derivable acquisition description.
    pub provenance: String,
}

/// Observation payload encoding failure.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ObservationError {
    /// A required value is empty or null.
    #[error("observation failed structural validation")]
    Invalid,
    /// CBOR serialization failed.
    #[error("observation CBOR encoding failed")]
    Encoding,
}

impl ObservationV1 {
    /// Whether the fields meet the language-independent structural subset.
    #[must_use]
    pub fn is_structurally_valid(&self) -> bool {
        !self.source_id.trim().is_empty()
            && !self.subject.trim().is_empty()
            && !self.provenance.trim().is_empty()
            && !matches!(self.value, Value::Null)
            && self
                .parsed_times()
                .is_some_and(|(acquired, fresh_until)| fresh_until > acquired)
    }

    /// Encode using the exact predecessor key order and bare-CBOR representation.
    ///
    /// # Errors
    ///
    /// Returns a typed error for an invalid observation or a serialization failure.
    pub fn encode(&self) -> Result<Vec<u8>, ObservationError> {
        if !self.is_structurally_valid() {
            return Err(ObservationError::Invalid);
        }
        let map = Value::Map(vec![
            text_pair("@type", OBSERVATION_PAYLOAD_TYPE),
            (
                text("schemaVersion"),
                Value::Integer(OBSERVATION_SCHEMA_V1.into()),
            ),
            text_pair("sourceId", &self.source_id),
            text_pair("subject", &self.subject),
            (text("value"), self.value.clone()),
            text_pair("acquiredAt", &self.acquired_at),
            text_pair("freshnessUntil", &self.freshness_until),
            text_pair("provenance", &self.provenance),
        ]);
        let mut encoded = Vec::new();
        ciborium::into_writer(&map, &mut encoded).map_err(|_| ObservationError::Encoding)?;
        Ok(encoded)
    }

    /// Deterministic UUID v5 over the predecessor's canonical CBOR identity tuple.
    ///
    /// # Errors
    ///
    /// Returns an encoding error only if CBOR serialization fails.
    pub fn message_id(&self) -> Result<Uuid, ObservationError> {
        let (acquired_at, _) = self.parsed_times().ok_or(ObservationError::Invalid)?;
        let acquired_at_unix_ms = i64::try_from(acquired_at.unix_timestamp_nanos() / 1_000_000)
            .map_err(|_| ObservationError::Invalid)?;
        let key = Value::Array(vec![
            text(&self.source_id),
            text(&self.subject),
            Value::Integer(acquired_at_unix_ms.into()),
            self.value.clone(),
        ]);
        let mut encoded = Vec::new();
        ciborium::into_writer(&key, &mut encoded).map_err(|_| ObservationError::Encoding)?;
        Ok(Uuid::new_v5(&OBSERVATION_NAMESPACE, &encoded))
    }

    fn parsed_times(&self) -> Option<(OffsetDateTime, OffsetDateTime)> {
        Some((
            OffsetDateTime::parse(&self.acquired_at, &Rfc3339).ok()?,
            OffsetDateTime::parse(&self.freshness_until, &Rfc3339).ok()?,
        ))
    }
}

fn text(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn text_pair(key: &str, value: &str) -> (Value, Value) {
    (text(key), text(value))
}

#[cfg(test)]
mod tests {
    use ciborium::Value;
    use uuid::Uuid;

    use super::{ObservationError, ObservationV1};

    const QT_PAYLOAD_HEX: &str = include_str!("../../../fixtures/protocol/observation-v1.hex");
    const QT_MESSAGE_ID: &str =
        include_str!("../../../fixtures/protocol/observation-v1-message-id.txt");

    fn fixture() -> ObservationV1 {
        ObservationV1 {
            source_id: "nixos.system.generation".into(),
            subject: "current-generation".into(),
            value: Value::Integer(142.into()),
            acquired_at: "2026-08-11T09:00:00.000Z".into(),
            freshness_until: "2026-08-11T09:05:00.000Z".into(),
            provenance: "readlink /run/current-system".into(),
        }
    }

    fn hex_bytes(value: &str) -> Vec<u8> {
        value
            .trim()
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                u8::from_str_radix(std::str::from_utf8(pair).expect("ASCII hex"), 16)
                    .expect("valid hex byte")
            })
            .collect()
    }

    #[test]
    fn payload_is_byte_identical_to_qt_observation_v1() {
        assert_eq!(
            fixture().encode().expect("encode"),
            hex_bytes(QT_PAYLOAD_HEX)
        );
    }

    #[test]
    fn message_identity_is_identical_to_qt_uuid_v5() {
        let expected = Uuid::parse_str(QT_MESSAGE_ID.trim()).expect("Qt UUID fixture");
        assert_eq!(fixture().message_id().expect("message identity"), expected);
    }

    #[test]
    fn null_and_empty_evidence_fail_closed() {
        let mut observation = fixture();
        observation.value = Value::Null;
        assert_eq!(observation.encode(), Err(ObservationError::Invalid));
        observation = fixture();
        observation.provenance.clear();
        assert_eq!(observation.encode(), Err(ObservationError::Invalid));
    }

    #[test]
    fn value_type_and_field_boundaries_participate_in_identity() {
        let base = fixture();
        let integer = base.message_id().expect("integer id");
        let mut string = base.clone();
        string.value = Value::Text("142".into());
        assert_ne!(integer, string.message_id().expect("string id"));

        let mut left = base.clone();
        left.source_id = "a".into();
        left.subject = "b\u{1f}c".into();
        let mut right = base;
        right.source_id = "a\u{1f}b".into();
        right.subject = "c".into();
        assert_ne!(
            left.message_id().expect("left id"),
            right.message_id().expect("right id")
        );
    }

    #[test]
    fn malformed_or_non_forward_time_fails_closed() {
        let mut observation = fixture();
        observation.acquired_at = "not-a-time".into();
        assert_eq!(observation.encode(), Err(ObservationError::Invalid));
        assert_eq!(observation.message_id(), Err(ObservationError::Invalid));

        observation = fixture();
        observation.freshness_until = observation.acquired_at.clone();
        assert_eq!(observation.encode(), Err(ObservationError::Invalid));
    }
}
