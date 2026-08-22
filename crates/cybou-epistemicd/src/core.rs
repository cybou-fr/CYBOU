// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `EpistemicCore` belief network engine and journal replay evaluation.

use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

use cybou_protocol::{Kind, canonical::CanonicalEnvelope};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::types::{
    BELIEF_RULE_VERSION, EpistemicBelief, EpistemicError, EpistemicState, EpistemicStatus, as_of,
    observed_claim,
};

/// Core domain logic of the epistemic organ.
pub struct EpistemicCore {
    caught_up: AtomicBool,
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
    /// Record that every contribution the Journal already held has now been delivered.
    pub fn mark_caught_up(&self) {
        self.caught_up.store(true, Ordering::Release);
    }

    /// Whether this projection has seen the whole Journal at least once.
    #[must_use]
    pub fn is_caught_up(&self) -> bool {
        self.caught_up.load(Ordering::Acquire)
    }

    /// Create a new transient `EpistemicCore` engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            caught_up: AtomicBool::new(false),
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
                (0, HashMap::new())
            }
        } else {
            (0, HashMap::new())
        };

        Ok(Self {
            caught_up: AtomicBool::new(false),
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
        self.ingest_classified(
            subject,
            value,
            confidence,
            evidence_id,
            now,
            at_sequence,
            fresh_until,
            0,
        );
    }

    /// Ingest an observation together with how sensitive the contribution carrying it was.
    #[allow(
        clippy::too_many_arguments,
        reason = "every part of an observation is needed to decide what it does to a belief"
    )]
    pub fn ingest_classified(
        &self,
        subject: impl Into<String>,
        value: impl Into<String>,
        confidence: f64,
        evidence_id: Option<Uuid>,
        now: OffsetDateTime,
        at_sequence: Option<u64>,
        fresh_until: Option<OffsetDateTime>,
        sensitivity: u8,
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
                sensitivity,
            });

        let was_still_vouched_for = entry.fresh_until.is_none_or(|until| now < until);

        if entry.value == value_str {
            entry.confidence = (entry.confidence * 0.7 + confidence * 0.3).clamp(0.0, 1.0);
            entry.last_corroborated_at = now;
            entry.status = EpistemicStatus::Observed;
        } else if was_still_vouched_for {
            entry.status = EpistemicStatus::Disputed;
            entry.confidence = (entry.confidence * 0.5).clamp(0.0, 1.0);
        } else {
            entry.value.clone_from(&value_str);
            entry.confidence = confidence.clamp(0.0, 1.0);
            entry.last_corroborated_at = now;
            entry.status = EpistemicStatus::Superseded;
            entry.evidence.clear();
        }
        entry.fresh_until = fresh_until;
        entry.sensitivity = entry.sensitivity.max(sensitivity);

        if let Some(id) = evidence_id
            && !entry.evidence.contains(&id)
        {
            entry.evidence.push(id);
        }

        self.commit(candidate, at_sequence);
    }

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
            let Some((subject, value, fresh_until)) = observed_claim(&envelope.payload) else {
                return;
            };

            let now = OffsetDateTime::from_unix_timestamp_nanos(
                i128::from(envelope.wall_time_ms) * 1_000_000,
            )
            .unwrap_or_else(|_| OffsetDateTime::now_utc());

            self.ingest_classified(
                subject,
                value,
                envelope.confidence,
                Some(envelope.message_id),
                now,
                Some(sequence),
                fresh_until,
                envelope.sensitivity,
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
