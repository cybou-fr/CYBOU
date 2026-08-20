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
    /// Epistemic validity status as it stood when the belief was last written.
    ///
    /// Read through [`EpistemicCore::query`] or [`EpistemicCore::projection`], which decide
    /// staleness against the clock rather than against whenever the last observation arrived.
    pub status: EpistemicStatus,
    /// When the observation behind this belief stops vouching for it, if it said.
    ///
    /// An observation names its own freshness horizon. Past it, nothing is asserting the belief
    /// any more: it is what was last seen, not what is the case.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub fresh_until: Option<OffsetDateTime>,
}

/// Persistent snapshot of the epistemic projection state.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EpistemicState {
    /// Which derivation rule produced these beliefs.
    ///
    /// A projection is derived, not observed: it is only as good as the rule that produced it.
    /// When that rule changes, beliefs carried over from the old one assert things the current
    /// rule would never have concluded, so they are discarded and rebuilt from the Journal rather
    /// than trusted because they happen to be on disk. Absent in state written before this field
    /// existed, which is exactly the state that must not be trusted.
    #[serde(default)]
    pub rule_version: u32,
    /// Journal sequence cursor mark.
    pub cursor: u64,
    /// Map of active beliefs.
    pub beliefs: HashMap<String, EpistemicBelief>,
}

/// The rule this build derives beliefs with.
///
/// Raise it whenever `ingest_envelope` changes what it concludes from the same contribution.
pub const BELIEF_RULE_VERSION: u32 = 2;

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
            if state.rule_version == BELIEF_RULE_VERSION {
                (state.cursor, state.beliefs)
            } else {
                // Rebuild from sequence zero rather than keep conclusions a rule that no longer
                // exists once drew. Nothing is lost: the Journal is the record, and the
                // projection is reconstructible from it by design.
                (0, HashMap::new())
            }
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
                rule_version: BELIEF_RULE_VERSION,
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
        self.ingest_at(subject, value, confidence, evidence_id, now, None);
    }

    /// Ingest an observation that came from a known position in the Journal.
    ///
    /// The sequence travels with the beliefs so both are written in one step.
    pub fn ingest_at(
        &self,
        subject: impl Into<String>,
        value: impl Into<String>,
        confidence: f64,
        evidence_id: Option<Uuid>,
        now: OffsetDateTime,
        at_sequence: Option<u64>,
    ) {
        self.ingest_at_until(
            subject,
            value,
            confidence,
            evidence_id,
            now,
            at_sequence,
            None,
        );
    }

    /// Ingest an observation that names how long it vouches for what it reports.
    #[allow(
        clippy::too_many_arguments,
        reason = "every part of an observation is needed to decide what it does to a belief"
    )]
    pub fn ingest_at_until(
        &self,
        subject: impl Into<String>,
        value: impl Into<String>,
        confidence: f64,
        evidence_id: Option<Uuid>,
        now: OffsetDateTime,
        at_sequence: Option<u64>,
        fresh_until: Option<OffsetDateTime>,
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
                fresh_until,
            });

        // Whether the belief being replaced still had anything vouching for it is the difference
        // between two sources contradicting each other and one report simply outliving another.
        // Calling both a dispute made every ordinary change of a clock, a battery reading or a
        // hostname look like the system arguing with itself.
        let was_still_vouched_for = entry.fresh_until.is_none_or(|until| now < until);

        if entry.value == value_str {
            entry.confidence = (entry.confidence * 0.7 + confidence * 0.3).clamp(0.0, 1.0);
            entry.last_corroborated_at = now;
            entry.status = EpistemicStatus::Observed;
        } else if was_still_vouched_for {
            entry.status = EpistemicStatus::Disputed;
            entry.confidence = (entry.confidence * 0.5).clamp(0.0, 1.0);
        } else {
            // Nothing was standing behind the old value any more, so this is not a contradiction
            // to hold open: it is the newer report taking the place of the older one.
            entry.value.clone_from(&value_str);
            entry.confidence = confidence.clamp(0.0, 1.0);
            entry.last_corroborated_at = now;
            entry.status = EpistemicStatus::Superseded;
            entry.evidence.clear();
        }
        entry.fresh_until = fresh_until;

        if let Some(id) = evidence_id
            && !entry.evidence.contains(&id)
        {
            entry.evidence.push(id);
        }

        self.commit(candidate, at_sequence);
    }

    /// Persist beliefs and the cursor that produced them as one value, then publish both.
    ///
    /// They have to move together. Persisting beliefs against the old cursor left the disk saying
    /// "these beliefs, through event 41" when they were formed through 42, so a restart re-ingested
    /// 42 — and the blending and dispute rules are not idempotent, so replaying one contribution
    /// twice can change what the system believes.
    fn commit(&self, beliefs: HashMap<String, EpistemicBelief>, at_sequence: Option<u64>) {
        let cursor = at_sequence.unwrap_or_else(|| self.cursor());
        if self.persist_candidate(cursor, &beliefs).is_err() {
            return;
        }
        if let Ok(mut lock) = self.beliefs.write() {
            *lock = beliefs;
        }
        if let Some(sequence) = at_sequence
            && let Ok(mut cur) = self.cursor.write()
            && sequence > *cur
        {
            *cur = sequence;
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
            let Some((subject, value, fresh_until)) = observed_claim(&envelope.payload) else {
                // A payload that does not decode is not a belief about anything. Storing its
                // bytes as the asserted value would put an unreadable string where a claim
                // belongs — and publish whatever the payload happened to contain.
                return;
            };

            let now = OffsetDateTime::from_unix_timestamp_nanos(
                i128::from(envelope.wall_time_ms) * 1_000_000,
            )
            .unwrap_or_else(|_| OffsetDateTime::now_utc());

            self.ingest_at_until(
                subject,
                value,
                envelope.confidence,
                Some(envelope.message_id),
                now,
                Some(sequence),
                fresh_until,
            );
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
        let belief = self.beliefs.read().ok()?.get(subject).cloned()?;
        Some(as_of(belief, OffsetDateTime::now_utc()))
    }

    /// Full epistemic projection sorted by subject.
    #[must_use]
    pub fn projection(&self) -> Vec<EpistemicBelief> {
        let map = match self.beliefs.read() {
            Ok(g) => g.clone(),
            Err(_) => return vec![],
        };
        let now = OffsetDateTime::now_utc();
        let mut list: Vec<_> = map.into_values().map(|belief| as_of(belief, now)).collect();
        list.sort_by(|a, b| a.subject.cmp(&b.subject));
        list
    }
}

/// The subject and asserted value carried by an observation payload.
///
/// Returns `None` when the payload is not an observation this version can read. Refusing is the
/// point: a belief whose value is undecodable bytes asserts nothing, and rendering it anywhere
/// would show a reader the payload rather than the claim.
fn observed_claim(payload: &[u8]) -> Option<(String, String, Option<OffsetDateTime>)> {
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
    // An unreadable horizon is not an unlimited one, but neither is it grounds to throw the
    // observation away: the belief simply carries no horizon and is never called stale on the
    // strength of a field nobody could read.
    let fresh_until = OffsetDateTime::parse(
        &observation.freshness_until,
        &time::format_description::well_known::Rfc3339,
    )
    .ok();
    Some((observation.subject, value, fresh_until))
}

/// A belief as it stands at `now`.
///
/// Staleness is a fact about the clock, not about the last time an observation happened to arrive.
/// Deciding it at write time would have left a belief reading `observed` for as long as nothing
/// else was written about the subject, which is precisely the case where it is least true.
fn as_of(mut belief: EpistemicBelief, now: OffsetDateTime) -> EpistemicBelief {
    if belief.status != EpistemicStatus::Disputed
        && belief.fresh_until.is_some_and(|until| now >= until)
    {
        belief.status = EpistemicStatus::Stale;
    }
    belief
}

#[cfg(test)]
mod tests {
    #[test]
    fn beliefs_and_the_cursor_that_produced_them_are_written_together() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("epistemic-state.json");
        let core = super::EpistemicCore::open(&path).expect("open state");

        core.ingest_at(
            "operating-system",
            "Debian GNU/Linux 13 (trixie)",
            1.0,
            None,
            time::OffsetDateTime::now_utc(),
            Some(42),
        );

        // Persisting beliefs against the previous cursor left the disk claiming the beliefs of
        // event 42 had been formed through 41, so a restart ingested 42 a second time — and the
        // blending and dispute rules are not idempotent.
        let restarted = super::EpistemicCore::open(&path).expect("reopen state");
        assert_eq!(restarted.cursor(), 42);
        assert_eq!(restarted.projection().len(), 1);
    }

    #[test]
    fn beliefs_derived_by_an_older_rule_are_rebuilt_rather_than_trusted() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("epistemic-state.json");

        // State as an earlier build wrote it: no rule version, and a belief that build concluded.
        std::fs::write(
            &path,
            r#"{"cursor":42,"beliefs":{"organ.perceptiond":{"subject":"organ.perceptiond","value":"garbage","confidence":1.0,"evidence":[],"lastCorroboratedAt":"2026-08-19T12:00:00Z","status":"disputed"}}}"#,
        )
        .expect("write legacy state");

        let core = super::EpistemicCore::open(&path).expect("open over legacy state");
        assert_eq!(core.cursor(), 0, "replay must restart from the Journal");
        assert!(
            core.projection().is_empty(),
            "conclusions of a rule that no longer exists must not survive"
        );
    }

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

        let (subject, value, fresh_until) =
            super::observed_claim(&payload).expect("a readable observation");
        assert_eq!(subject, "operating-system");
        assert_eq!(value, "Debian GNU/Linux 13 (trixie)");
        // The horizon the observation named is carried, not discarded: it is what makes the belief
        // become stale on its own rather than reading `observed` until something else arrives.
        assert_eq!(
            fresh_until.expect("the horizon was read").unix_timestamp(),
            time::OffsetDateTime::parse(
                "2026-08-19T17:59:15.103Z",
                &time::format_description::well_known::Rfc3339
            )
            .expect("fixture parses")
            .unix_timestamp()
        );

        // Anything that is not a readable observation asserts nothing and must not become one.
        assert_eq!(super::observed_claim(b"not cbor at all"), None);
        assert_eq!(super::observed_claim(&[]), None);
    }

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn a_report_that_outlived_its_horizon_is_replaced_rather_than_disputed() {
        let core = EpistemicCore::new();
        let rfc = &time::format_description::well_known::Rfc3339;
        let at = |text: &str| OffsetDateTime::parse(text, rfc).expect("fixture parses");

        let observed = at("2026-08-20T12:00:00Z");
        let horizon = at("2026-08-20T12:05:00Z");
        core.ingest_at_until("battery", "80", 1.0, None, observed, None, Some(horizon));
        assert_eq!(
            core.query("battery").expect("a belief").status,
            EpistemicStatus::Stale,
            "the horizon is long past, so nothing is vouching for it now"
        );

        // Within the horizon two different values are two sources contradicting each other.
        core.ingest_at_until(
            "battery",
            "60",
            1.0,
            None,
            at("2026-08-20T12:01:00Z"),
            None,
            Some(horizon),
        );
        assert_eq!(
            core.query("battery").expect("a belief").status,
            EpistemicStatus::Disputed
        );

        // Past it, a new reading is not an argument: it takes the place of the old one. The new
        // horizon is ahead of the reader's clock, because a belief whose horizon has also passed
        // is stale to a reader no matter how it came to hold the value it holds.
        let now = OffsetDateTime::now_utc();
        core.ingest_at_until(
            "battery",
            "42",
            1.0,
            None,
            now,
            None,
            Some(now + time::Duration::hours(1)),
        );
        let replaced = core.query("battery").expect("a belief");
        assert_eq!(replaced.value, "42");
        assert_eq!(replaced.status, EpistemicStatus::Superseded);
    }

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
