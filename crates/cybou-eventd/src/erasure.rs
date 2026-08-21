// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Decoding and validation helpers for erasure records.

use cybou_protocol::{admission::ErasureReason, canonical::CanonicalEnvelope};
use uuid::Uuid;

/// The target and reason an erasure record carries.
#[must_use]
pub fn decode_erasure_record(envelope: &CanonicalEnvelope) -> Option<(Uuid, ErasureReason)> {
    let value: ciborium::Value = ciborium::from_reader(envelope.payload.as_slice()).ok()?;
    let map = value.as_map()?;
    let field = |name: &str| {
        map.iter()
            .find(|(key, _)| key.as_text() == Some(name))
            .and_then(|(_, value)| value.as_text())
    };
    let target = Uuid::parse_str(field("target")?).ok()?;
    let reason = ErasureReason::from_name(field("reason")?)?;
    Some((target, reason))
}
