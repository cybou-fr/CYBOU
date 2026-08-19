// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Epistemic projection and belief validity engine (ADR-0027: observation != knowledge).
//!
//! Evaluates incoming observations and journal replay against historical evidence,
//! maintaining reconstructible epistemic propositions with dispute and staleness tracking.

use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::RwLock,
};

use cybou_protocol::{Kind, canonical::CanonicalEnvelope, observation::ObservationV1};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

#[cfg(target_os = "linux")]
pub mod service;

/// Epistemic validity status of a proposition.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EpistemicStatus {
    /// Actively corroborated by recent observations.
    Observed,
    /// Previously observed but beyond freshness horizon without corroboration.
    Stale,
    /// Contradicted by competing observations with conflicting values.
    Disputed,
    /// Explicitly superseded by a newer belief revision.
    Superseded,
    /// Not yet observed or unresolvable.
    Unknown,
}

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
    /// Epistemic validity status.
    pub status: EpistemicStatus,
}

/// Persistent snapshot of the epistemic projection state.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpistemicState {
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

/// Core domain logic of the epistemic organ.
pub struct EpistemicCore {
    state_path: Option<PathBuf>,
    cursor: RwLock<u64>,
    beliefs: RwLock<HashMap<String, EpistemicBelief>>,
}

impl Default for EpistemicCore {
    fn default() -> Self {
        Self::new()
    }
}

impl EpistemicCore {
    /// Create a new transient `EpistemicCore` engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state_path: None,
            cursor: RwLock::new(0),
            beliefs: RwLock::new(HashMap::new()),
        }
    }

    /// Open `EpistemicCore` with persistent JSON storage.
    ///
    /// # Errors
    ///
    /// Returns [`EpistemicError`] on I/O failure or corrupt state file.
    pub fn open(path: &Path) -> Result<Self, EpistemicError> {
        let (cursor, beliefs) = if path.exists() {
            let mut file = File::open(path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            let state: EpistemicState = serde_json::from_str(&content)
                .map_err(|e| EpistemicError::CorruptState(e.to_string()))?;
            (state.cursor, state.beliefs)
        } else {
            (0, HashMap::new())
        };

        Ok(Self {
            state_path: Some(path.to_path_buf()),
            cursor: RwLock::new(cursor),
            beliefs: RwLock::new(beliefs),
        })
    }

    fn persist_candidate(
        &self,
        cursor: u64,
        beliefs: &HashMap<String, EpistemicBelief>,
    ) -> Result<(), EpistemicError> {
        if let Some(path) = &self.state_path {
            let state = EpistemicState {
                cursor,
                beliefs: beliefs.clone(),
            };
            let serialized = serde_json::to_string_pretty(&state)
                .map_err(|e| EpistemicError::CorruptState(e.to_string()))?;

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let temp_path = path.with_extension("tmp");
            {
                let mut temp_file = File::create(&temp_path)?;
                temp_file.write_all(serialized.as_bytes())?;
                temp_file.sync_all()?;
            }
            fs::rename(&temp_path, path)?;
        }
        Ok(())
    }

    /// Ingest a single observation into the epistemic belief network.
    pub fn ingest(
        &self,
        subject: impl Into<String>,
        value: impl Into<String>,
        confidence: f64,
        evidence_id: Option<Uuid>,
        now: OffsetDateTime,
    ) {
        let subject_str = subject.into();
        let value_str = value.into();

        let mut candidate = self.beliefs.read().map(|g| g.clone()).unwrap_or_default();
        let entry = candidate
            .entry(subject_str.clone())
            .or_insert_with(|| EpistemicBelief {
                subject: subject_str,
                value: value_str.clone(),
                confidence,
                evidence: Vec::new(),
                last_corroborated_at: now,
                status: EpistemicStatus::Observed,
            });

        if entry.value == value_str {
            entry.confidence = (entry.confidence * 0.7 + confidence * 0.3).clamp(0.0, 1.0);
            entry.last_corroborated_at = now;
            entry.status = EpistemicStatus::Observed;
        } else {
            entry.status = EpistemicStatus::Disputed;
            entry.confidence = (entry.confidence * 0.5).clamp(0.0, 1.0);
        }

        if let Some(id) = evidence_id
            && !entry.evidence.contains(&id)
        {
            entry.evidence.push(id);
        }

        let cur = self.cursor();
        if self.persist_candidate(cur, &candidate).is_ok()
            && let Ok(mut lock) = self.beliefs.write()
        {
            *lock = candidate;
        }
    }

    /// Ingest an envelope during Journal replay.
    pub fn ingest_envelope(&self, envelope: &CanonicalEnvelope, sequence: u64) {
        let Some(kind) = Kind::from_u16(envelope.kind) else {
            return;
        };

        if matches!(
            kind,
            Kind::Observation | Kind::BeliefRevision | Kind::Hypothesis
        ) {
            // The organ that spoke is not the subject of what it said. An observation carries its
            // own subject and value, and using them is what makes two organs able to corroborate
            // or contradict each other about the same thing. Keying beliefs by origin organ
            // instead collapsed everything one organ ever observed into a single belief that
            // disputed itself on every new observation.
            let Some((subject, value)) = observed_claim(&envelope.payload) else {
                // A payload that does not decode is not a belief about anything. Storing its
                // bytes as the asserted value would put an unreadable string where a claim
                // belongs — and publish whatever the payload happened to contain.
                return;
            };

            let now = OffsetDateTime::from_unix_timestamp_nanos(
                i128::from(envelope.wall_time_ms) * 1_000_000,
            )
            .unwrap_or_else(|_| OffsetDateTime::now_utc());

            self.ingest(
                subject,
                value,
                envelope.confidence,
                Some(envelope.message_id),
                now,
            );

            if let Ok(mut cur) = self.cursor.write()
                && sequence > *cur
            {
                *cur = sequence;
            }
        }
    }

    /// Replay a batch of canonical envelopes to reconstruct epistemic state.
    pub fn replay_batch(&self, envelopes: &[(u64, CanonicalEnvelope)]) {
        for (seq, env) in envelopes {
            self.ingest_envelope(env, *seq);
        }
    }

    /// Current journal sequence cursor mark.
    #[must_use]
    pub fn cursor(&self) -> u64 {
        self.cursor.read().map_or(0, |g| *g)
    }

    /// Query a single belief by subject.
    #[must_use]
    pub fn query(&self, subject: &str) -> Option<EpistemicBelief> {
        self.beliefs.read().ok()?.get(subject).cloned()
    }

    /// Full epistemic projection sorted by subject.
    #[must_use]
    pub fn projection(&self) -> Vec<EpistemicBelief> {
        let map = match self.beliefs.read() {
            Ok(g) => g.clone(),
            Err(_) => return vec![],
        };
        let mut list: Vec<_> = map.into_values().collect();
        list.sort_by(|a, b| a.subject.cmp(&b.subject));
        list
    }
}

/// The subject and asserted value carried by an observation payload.
///
/// Returns `None` when the payload is not an observation this version can read. Refusing is the
/// point: a belief whose value is undecodable bytes asserts nothing, and rendering it anywhere
/// would show a reader the payload rather than the claim.
fn observed_claim(payload: &[u8]) -> Option<(String, String)> {
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
    Some((observation.subject, value))
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_belief_is_about_what_was_observed_not_about_who_observed_it() {
        let observation = cybou_protocol::observation::ObservationV1 {
            source_id: "linux.system".into(),
            subject: "operating-system".into(),
            value: ciborium::Value::Text("Debian GNU/Linux 13 (trixie)".into()),
            acquired_at: "2026-08-19T17:54:15.103Z".into(),
            freshness_until: "2026-08-19T17:59:15.103Z".into(),
            provenance: "os-release from /etc/os-release".into(),
        };
        let mut payload = Vec::new();
        ciborium::into_writer(&observation, &mut payload).expect("encode observation");

        assert_eq!(
            super::observed_claim(&payload),
            Some((
                "operating-system".to_owned(),
                "Debian GNU/Linux 13 (trixie)".to_owned()
            ))
        );

        // Anything that is not a readable observation asserts nothing and must not become one.
        assert_eq!(super::observed_claim(b"not cbor at all"), None);
        assert_eq!(super::observed_claim(&[]), None);
    }

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn epistemic_reconstruction_and_persistence() {
        let dir = tempdir().expect("tempdir");
        let state_path = dir.path().join("epistemic.json");

        let core = EpistemicCore::open(&state_path).expect("open");
        let now = OffsetDateTime::now_utc();
        let ev1 = Uuid::new_v4();

        core.ingest("system.os", "Debian 13", 1.0, Some(ev1), now);
        assert_eq!(
            core.query("system.os").unwrap().status,
            EpistemicStatus::Observed
        );

        // Competing value creates dispute
        core.ingest("system.os", "Fedora 40", 0.9, None, now);
        assert_eq!(
            core.query("system.os").unwrap().status,
            EpistemicStatus::Disputed
        );

        // Reopen from disk: must survive restart
        let reopened = EpistemicCore::open(&state_path).expect("reopen");
        let b = reopened.query("system.os").expect("reopened belief");
        assert_eq!(b.status, EpistemicStatus::Disputed);
    }
}
