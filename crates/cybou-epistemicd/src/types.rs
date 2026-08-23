// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Epistemic belief types, statuses, state snapshots, and decoding helpers.

use std::collections::HashMap;

use cybou_protocol::observation::ObservationV1;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

/// The rule this build derives beliefs with.
pub const BELIEF_RULE_VERSION: u32 = 3;

pub use cybou_protocol::epistemic::EpistemicStatus;

/// A validated epistemic proposition.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpistemicBelief {
    /// Subject of the belief (e.g. "os.version", "system.power").
    pub subject: String,
    /// Asserted value or state.
    pub value: String,
    /// Epistemic confidence in [0.0, 1.0].
    pub confidence: f64,
    /// Contributing evidence / causal message IDs.
    pub evidence: Vec<Uuid>,
    /// When this belief was last corroborated.
    #[serde(with = "time::serde::rfc3339")]
    pub last_corroborated_at: OffsetDateTime,
    /// Epistemic validity status as it stood when the belief was last written.
    pub status: EpistemicStatus,
    /// When the observation behind this belief stops vouching for it, if it said.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub fresh_until: Option<OffsetDateTime>,
    /// The highest sensitivity among the observations that produced this belief.
    #[serde(default)]
    pub sensitivity: u8,
}

/// Persistent snapshot of the epistemic projection state.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpistemicState {
    /// Which derivation rule produced these beliefs.
    #[serde(default)]
    pub rule_version: u32,
    /// Journal sequence cursor mark.
    pub cursor: u64,
    /// Map of active beliefs.
    pub beliefs: HashMap<String, EpistemicBelief>,
}

/// Errors occurring in the epistemic engine.
#[derive(Debug, Error)]
pub enum EpistemicError {
    /// I/O error reading or writing state.
    #[error("epistemic state i/o error: {0}")]
    Io(#[from] std::io::Error),
    /// Corrupt state file.
    #[error("epistemic state file corrupted: {0}")]
    CorruptState(String),
    /// Internal lock poisoned.
    #[error("epistemic lock poisoned")]
    LockPoisoned,
}

/// The subject and asserted value carried by an observation payload.
#[must_use]
pub fn observed_claim(payload: &[u8]) -> Option<(String, String, Option<OffsetDateTime>)> {
    let observation: ObservationV1 = ciborium::from_reader(payload).ok()?;
    let value = match &observation.value {
        ciborium::Value::Text(text) => text.clone(),
        ciborium::Value::Integer(number) => i128::from(*number).to_string(),
        ciborium::Value::Float(number) => number.to_string(),
        ciborium::Value::Bool(flag) => flag.to_string(),
        _ => return None,
    };
    if observation.subject.is_empty() || value.is_empty() {
        return None;
    }
    let fresh_until = OffsetDateTime::parse(
        &observation.freshness_until,
        &time::format_description::well_known::Rfc3339,
    )
    .ok();
    Some((observation.subject, value, fresh_until))
}

/// A belief as it stands at `now`.
#[must_use]
pub fn as_of(mut belief: EpistemicBelief, now: OffsetDateTime) -> EpistemicBelief {
    if belief.status != EpistemicStatus::Disputed
        && belief.fresh_until.is_some_and(|until| now >= until)
    {
        belief.status = EpistemicStatus::Stale;
    }
    belief
}
