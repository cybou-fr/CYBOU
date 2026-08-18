// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Frozen canonical envelope and Journal-row representations.

use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Protocol fields needed by canonical hashing, expressed without storage concerns.
#[derive(Clone, Debug)]
pub struct CanonicalEnvelope {
    /// Envelope schema selecting the canonical field set.
    pub schema_version: u16,
    /// Stable contribution identity.
    pub message_id: Uuid,
    /// Stable correlation identity.
    pub correlation_id: Uuid,
    /// Causal predecessor or nil for a root contribution.
    pub causation_id: Uuid,
    /// Process owner name.
    pub origin_organ: String,
    /// Node name.
    pub origin_node: String,
    /// Frozen numeric contribution kind.
    pub kind: u16,
    /// UTC epoch milliseconds.
    pub wall_time_ms: i64,
    /// Process-local monotonic reading.
    pub monotonic_time: u64,
    /// Logical clock.
    pub logical_clock: u64,
    /// Bounded confidence.
    pub confidence: f64,
    /// Evidence identities; canonical order is lexical UUID bytes.
    pub evidence: Vec<Uuid>,
    /// Stored payload bytes.
    pub payload: Vec<u8>,
    /// Frozen numeric privacy class.
    pub privacy: u8,
    /// Optional capability scope represented as an empty string when absent.
    pub capability_scope: String,
    /// Whether schema 3/4 payload protection is active.
    pub sealed: bool,
    /// Opaque protection key domain or nil.
    pub key_domain_id: Uuid,
    /// Protection key epoch.
    pub key_epoch: u32,
    /// Frozen numeric retention class.
    pub retention_class: u8,
    /// Retention policy version.
    pub retention_policy_version: u16,
    /// UTC expiry epoch milliseconds, or zero when unbounded.
    pub retain_until_ms: u64,
    /// Frozen numeric sensitivity class for schema 4.
    pub sensitivity: u8,
}

/// Predecessor-compatible canonical envelope v2 bytes.
#[must_use]
pub fn canonical_envelope_v2(envelope: &CanonicalEnvelope) -> Vec<u8> {
    let mut out = b"CYBOU-ENVELOPE-V2".to_vec();
    common_prefix(&mut out, envelope);
    append_bytes(&mut out, &envelope.payload);
    out.push(envelope.privacy);
    append_string(&mut out, &envelope.capability_scope);
    out
}

/// Canonical non-erasable metadata bytes used by Journal hash v3.
#[must_use]
pub fn canonical_nonerasable_v3(envelope: &CanonicalEnvelope) -> Vec<u8> {
    let mut out = b"CYBOU-ENVELOPE-NONERASABLE-V3".to_vec();
    common_prefix(&mut out, envelope);
    out.push(envelope.privacy);
    append_string(&mut out, &envelope.capability_scope);
    if matches!(envelope.schema_version, 3 | 4) {
        out.push(u8::from(envelope.sealed));
        out.extend_from_slice(envelope.key_domain_id.as_bytes());
        out.extend_from_slice(&envelope.key_epoch.to_be_bytes());
        out.push(envelope.retention_class);
        out.extend_from_slice(&envelope.retention_policy_version.to_be_bytes());
        out.extend_from_slice(&envelope.retain_until_ms.to_be_bytes());
    }
    if envelope.schema_version == 4 {
        out.push(envelope.sensitivity);
    }
    out
}

/// Stable Journal row v2 representation.
#[must_use]
pub fn canonical_journal_row_v2(
    sequence: u64,
    previous_hash: &[u8],
    envelope: &CanonicalEnvelope,
) -> Vec<u8> {
    let mut out = b"CYBOU-JOURNAL-ROW-V2".to_vec();
    out.extend_from_slice(&2_u16.to_be_bytes());
    out.extend_from_slice(&sequence.to_be_bytes());
    append_bytes(&mut out, previous_hash);
    append_bytes(&mut out, &canonical_envelope_v2(envelope));
    out
}

/// SHA-256 digest used by the predecessor Journal.
#[must_use]
pub fn sha256(value: &[u8]) -> [u8; 32] {
    Sha256::digest(value).into()
}

fn common_prefix(out: &mut Vec<u8>, envelope: &CanonicalEnvelope) {
    out.extend_from_slice(&envelope.schema_version.to_be_bytes());
    out.extend_from_slice(envelope.message_id.as_bytes());
    out.extend_from_slice(envelope.correlation_id.as_bytes());
    out.extend_from_slice(envelope.causation_id.as_bytes());
    append_string(out, &envelope.origin_organ);
    append_string(out, &envelope.origin_node);
    out.extend_from_slice(&envelope.kind.to_be_bytes());
    out.extend_from_slice(&envelope.wall_time_ms.to_be_bytes());
    out.extend_from_slice(&envelope.monotonic_time.to_be_bytes());
    out.extend_from_slice(&envelope.logical_clock.to_be_bytes());
    let confidence = if envelope.confidence == 0.0 {
        0.0
    } else {
        envelope.confidence
    };
    out.extend_from_slice(&confidence.to_bits().to_be_bytes());
    let mut evidence = envelope.evidence.clone();
    evidence.sort_unstable_by_key(|id| *id.as_bytes());
    out.extend_from_slice(&(u32::try_from(evidence.len()).unwrap_or(u32::MAX)).to_be_bytes());
    for id in evidence {
        out.extend_from_slice(id.as_bytes());
    }
}

fn append_string(out: &mut Vec<u8>, value: &str) {
    append_bytes(out, value.as_bytes());
}

fn append_bytes(out: &mut Vec<u8>, value: &[u8]) {
    out.extend_from_slice(&(u32::try_from(value.len()).unwrap_or(u32::MAX)).to_be_bytes());
    out.extend_from_slice(value);
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::{
        CanonicalEnvelope, canonical_envelope_v2, canonical_journal_row_v2,
        canonical_nonerasable_v3, sha256,
    };

    fn envelope() -> CanonicalEnvelope {
        CanonicalEnvelope {
            schema_version: 4,
            message_id: Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap(),
            correlation_id: Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap(),
            causation_id: Uuid::parse_str("33333333-3333-4333-8333-333333333333").unwrap(),
            origin_organ: "predictord".into(),
            origin_node: "local".into(),
            kind: 12,
            wall_time_ms: 1_787_127_330_125,
            monotonic_time: 123,
            logical_clock: 456,
            confidence: 0.75,
            evidence: vec![
                Uuid::parse_str("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb").unwrap(),
                Uuid::parse_str("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa").unwrap(),
            ],
            payload: vec![0xa1, 0x61, 0x78, 0x01],
            privacy: 1,
            capability_scope: "mind.prediction.read".into(),
            sealed: true,
            key_domain_id: Uuid::parse_str("44444444-4444-4444-8444-444444444444").unwrap(),
            key_epoch: 7,
            retention_class: 3,
            retention_policy_version: 2,
            retain_until_ms: 1_789_805_730_125,
            sensitivity: 3,
        }
    }

    fn bytes(name: &str) -> Vec<u8> {
        let raw = match name {
            "v2" => include_str!("../../../fixtures/protocol/envelope-v2.hex"),
            "v3" => include_str!("../../../fixtures/protocol/nonerasable-v3.hex"),
            "row" => include_str!("../../../fixtures/protocol/journal-row-v2.hex"),
            "v2-hash" => include_str!("../../../fixtures/protocol/envelope-v2-sha256.hex"),
            "v3-hash" => include_str!("../../../fixtures/protocol/nonerasable-v3-sha256.hex"),
            _ => unreachable!(),
        };
        raw.trim()
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    #[test]
    fn canonical_bytes_and_digests_are_identical_to_qt() {
        let envelope = envelope();
        let v2 = canonical_envelope_v2(&envelope);
        let v3 = canonical_nonerasable_v3(&envelope);
        assert_eq!(v2, bytes("v2"));
        assert_eq!(v3, bytes("v3"));
        assert_eq!(
            canonical_journal_row_v2(9, &[0x5a; 32], &envelope),
            bytes("row")
        );
        assert_eq!(sha256(&v2).as_slice(), bytes("v2-hash"));
        assert_eq!(sha256(&v3).as_slice(), bytes("v3-hash"));
    }

    #[test]
    fn evidence_order_and_negative_zero_are_canonicalized() {
        let mut left = envelope();
        let mut right = left.clone();
        right.evidence.reverse();
        left.confidence = -0.0;
        right.confidence = 0.0;
        assert_eq!(canonical_envelope_v2(&left), canonical_envelope_v2(&right));
    }
}
