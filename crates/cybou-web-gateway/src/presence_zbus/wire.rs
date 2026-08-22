// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The owners' own row shapes, and the two questions asked of every CBOR value.
//!
//! Nothing here decides anything. These are the shapes the organs actually write, decoded only so
//! they can be re-projected into the web contract, plus the helpers that read a CBOR map. They sit
//! apart from the readers because they describe somebody else's bytes: they change when an owner
//! changes, not when this gateway does.

use ciborium::Value;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

pub(super) fn field<'a>(map: &'a [(Value, Value)], name: &str) -> Option<&'a Value> {
    map.iter()
        .find_map(|(key, value)| (key.as_text() == Some(name)).then_some(value))
}

pub(super) fn text(value: &Value) -> Option<&str> {
    match value {
        Value::Text(value) => Some(value),
        Value::Tag(_, inner) => text(inner),
        _ => None,
    }
}
/// How many Journal rows the panel asks for. Enough to show that a system is living, few enough
/// that one read stays inside the gateway's budget.
pub(super) const RECENT_CONTRIBUTIONS: i32 = 12;

/// The frozen kind in its own spelling, or an explicit unknown.
///
/// A kind this contract version cannot name is reported as unknown rather than guessed at, for the
/// same reason `Kind::from_u16` refuses to default it.
pub(super) fn kind_name(kind: u16) -> String {
    cybou_protocol::Kind::from_u16(kind).map_or_else(
        || format!("unknown kind {kind}"),
        |kind| format!("{kind:?}").to_lowercase(),
    )
}

pub(super) fn millis_to_rfc3339(millis: i64) -> String {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(millis) * 1_000_000)
        .ok()
        .and_then(|instant| instant.format(&Rfc3339).ok())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".into())
}

/// Intention1's own row shape, decoded only to be re-projected into the web contract.
///
/// The identity is a `Uuid`, not a string: serde encodes a UUID as raw bytes in CBOR and as text
/// only in human-readable formats, so decoding it as `String` fails against the owner's real
/// bytes while looking correct in a JSON fixture.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OwnerIntention {
    pub(super) id: uuid::Uuid,
    pub(super) description: String,
    pub(super) trigger: String,
    pub(super) formed: String,
}

/// The fields of Self1's report the panel uses; the rest of the report stays with its owner.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OwnerSelfReport {
    pub(super) age_in_days: i64,
    pub(super) sessions: u64,
    pub(super) open_intentions: u32,
    pub(super) settled_predictions: u32,
}

/// Workspace1's momentary state.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OwnerMomentState {
    pub(super) focus: Option<uuid::Uuid>,
    pub(super) salience: f64,
    pub(super) organs: Vec<String>,
}

/// Epistemic1's belief row.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OwnerBelief {
    pub(super) subject: String,
    pub(super) value: String,
    pub(super) confidence: f64,
    pub(super) status: String,
    pub(super) last_corroborated_at: String,
    /// The contributions this belief was formed from.
    ///
    /// Absent in projections written before beliefs carried it, which is treated as provenance that
    /// cannot be accounted for rather than as a belief that came from nothing.
    #[serde(default)]
    pub(super) evidence: Vec<Uuid>,
    /// What the belief was derived from, on the frozen sensitivity scale.
    ///
    /// Absent in projections written before beliefs carried it. Absent is not ordinary: a belief
    /// this gateway cannot classify is one it must not decide is safe to publish, so the default
    /// is the most exposing value rather than the least.
    #[serde(default = "unclassified")]
    pub(super) sensitivity: u8,
}

/// The frozen sensitivity class of something that is about the person.
///
/// `Personal` on the scale in `cybou_protocol::admission`. Named here because two projections are
/// filtered against it without carrying a class of their own: what they hold is about the person by
/// construction rather than by classification.
pub(super) const PERSONAL: u8 = 1;

/// What an owner projection written before it carried a class is treated as.
///
/// The top of the frozen scale. An older projection is not evidence that its contents are
/// ordinary, and defaulting to zero would publish exactly the rows nobody had classified yet.
pub(super) const fn unclassified() -> u8 {
    u8::MAX
}

/// Perception1's last acquisition.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OwnerPerceptionState {
    pub(super) status: String,
    pub(super) acquired_at: String,
    pub(super) source_id: String,
}

/// Context1's concept node.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OwnerConcept {
    pub(super) label: String,
    pub(super) salience: f64,
    pub(super) activation_reason: String,
    pub(super) last_activated_at: String,
    /// What activated the concept, on the frozen sensitivity scale.
    #[serde(default = "unclassified")]
    pub(super) sensitivity: u8,
}

/// Event1's verification state.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OwnerVerification {
    pub(super) verified_through: u64,
    pub(super) head: u64,
    pub(super) broken_at: Option<u64>,
}

/// Lifecycle1's own state shape, of which the panel uses two fields.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OwnerLifecycle {
    pub(super) mode: String,
    pub(super) last_user_activity_at: String,
}
