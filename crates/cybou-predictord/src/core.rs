// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `PredictorCore` statistical forecasting and settlement engine.

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
use uuid::Uuid;

use crate::types::{
    Calibration, Forecast, PredictorError, PredictorState, Sample, SubjectState,
    observed_measurement,
};

/// Core domain logic of the predictor organ.
pub struct PredictorCore {
    caught_up: AtomicBool,
    state_path: Option<PathBuf>,
    cursor: RwLock<u64>,
    by_subject: RwLock<HashMap<String, SubjectState>>,
}

impl Default for PredictorCore {
    fn default() -> Self {
        Self::new()
    }
}

impl PredictorCore {
    /// Record that every contribution the Journal already held has now been delivered.
    pub fn mark_caught_up(&self) {
        self.caught_up.store(true, Ordering::Release);
    }

    /// Whether this projection has seen the whole Journal at least once.
    #[must_use]
    pub fn is_caught_up(&self) -> bool {
        self.caught_up.load(Ordering::Acquire)
    }

    /// Create a transient in-memory `PredictorCore` engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            caught_up: AtomicBool::new(false),
            state_path: None,
            cursor: RwLock::new(0),
            by_subject: RwLock::new(HashMap::new()),
        }
    }

    /// Open `PredictorCore` with persistent JSON storage.
    ///
    /// # Errors
    ///
    /// Returns [`PredictorError`] on I/O failure or corrupt state file.
    pub fn open(path: &Path) -> Result<Self, PredictorError> {
        let (cursor, subjects) = if path.exists() {
            let mut file = File::open(path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            let state: PredictorState = serde_json::from_str(&content)
                .map_err(|e| PredictorError::CorruptState(e.to_string()))?;
            (state.cursor, state.subjects)
        } else {
            (0, HashMap::new())
        };

        Ok(Self {
            caught_up: AtomicBool::new(false),
            state_path: Some(path.to_path_buf()),
            cursor: RwLock::new(cursor),
            by_subject: RwLock::new(subjects),
        })
    }

    fn persist_candidate(
        &self,
        cursor: u64,
        subjects: &HashMap<String, SubjectState>,
    ) -> Result<(), PredictorError> {
        if let Some(path) = &self.state_path {
            let state = PredictorState {
                cursor,
                subjects: subjects.clone(),
            };
            let serialized = serde_json::to_string_pretty(&state)
                .map_err(|e| PredictorError::CorruptState(e.to_string()))?;

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

    /// Record an observation sample for a subject (durable before visible).
    pub fn observe(&self, subject: &str, value: f64, contribution_id: Uuid) {
        self.observe_at(subject, value, contribution_id, self.cursor());
    }

    fn observe_at(&self, subject: &str, value: f64, contribution_id: Uuid, sequence: u64) {
        let subject = subject.trim().to_string();
        if subject.is_empty() {
            return;
        }

        let mut candidate = self
            .by_subject
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        let state = candidate.entry(subject).or_default();
        state.samples.push(Sample {
            contribution_id,
            value,
        });

        if self.persist_candidate(sequence, &candidate).is_ok()
            && let Ok(mut lock) = self.by_subject.write()
        {
            *lock = candidate;
            self.set_cursor(sequence);
        }
    }

    fn set_cursor(&self, sequence: u64) {
        if let Ok(mut cursor) = self.cursor.write()
            && sequence > *cursor
        {
            *cursor = sequence;
        }
    }

    /// Generate a forecast for a subject based on past observed samples.
    ///
    /// # Errors
    ///
    /// Returns [`PredictorError`] if subject is empty or has no history.
    #[allow(
        clippy::cast_precision_loss,
        reason = "sample counts are empirical statistics; f64 rounding cannot change a forecast"
    )]
    pub fn predict(&self, subject: &str) -> Result<Forecast, PredictorError> {
        let subject = subject.trim();
        if subject.is_empty() {
            return Err(PredictorError::EmptySubject);
        }

        let map = self
            .by_subject
            .read()
            .map_err(|_| PredictorError::LockPoisoned)?;
        let state = map
            .get(subject)
            .ok_or_else(|| PredictorError::NoHistory(subject.to_string()))?;

        if state.samples.is_empty() {
            return Err(PredictorError::NoHistory(subject.to_string()));
        }

        let count = state.samples.len();
        let sum: f64 = state.samples.iter().map(|s| s.value).sum();
        let estimate = sum / count as f64;

        let spread: f64 = state
            .samples
            .iter()
            .map(|s| (s.value - estimate).abs())
            .sum();
        let margin = spread / count as f64;
        let confidence = count as f64 / (count as f64 + 3.0);

        Ok(Forecast {
            subject: subject.to_string(),
            estimate,
            margin,
            confidence,
            samples: u32::try_from(count).unwrap_or(u32::MAX),
        })
    }

    /// Settle a forecast with actual measured outcome (durable before visible).
    pub fn settle(&self, subject: &str, forecast_estimate: f64, actual: f64) {
        let subject = subject.trim().to_string();
        if subject.is_empty() {
            return;
        }

        let mut candidate = self
            .by_subject
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();
        let state = candidate.entry(subject).or_default();
        let error = actual - forecast_estimate;
        state.absolute_error += error.abs();
        state.signed_error += error;
        state.settled += 1;

        let cur = self.cursor();
        if self.persist_candidate(cur, &candidate).is_ok()
            && let Ok(mut lock) = self.by_subject.write()
        {
            *lock = candidate;
        }
    }

    /// Replay an envelope from Journal.
    pub fn ingest_envelope(&self, envelope: &CanonicalEnvelope, sequence: u64) {
        if Kind::from_u16(envelope.kind) == Some(Kind::Observation)
            && let Some((subject, value)) = observed_measurement(&envelope.payload)
        {
            self.observe_at(&subject, value, envelope.message_id, sequence);
            return;
        }

        let candidate = self
            .by_subject
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        if self.persist_candidate(sequence, &candidate).is_ok() {
            self.set_cursor(sequence);
        }
    }

    /// Current journal replay cursor.
    #[must_use]
    pub fn cursor(&self) -> u64 {
        self.cursor.read().map_or(0, |g| *g)
    }

    /// Return calibration for a subject.
    #[must_use]
    pub fn calibration(&self, subject: &str) -> Option<Calibration> {
        let map = self.by_subject.read().ok()?;
        let state = map.get(subject)?;
        if state.settled == 0 {
            return None;
        }
        Some(Calibration {
            subject: subject.to_string(),
            settled: state.settled,
            mean_error: state.absolute_error / f64::from(state.settled),
            bias: state.signed_error / f64::from(state.settled),
        })
    }

    /// Return all calibrated subjects.
    #[must_use]
    pub fn all_calibrations(&self) -> Vec<Calibration> {
        let Ok(map) = self.by_subject.read() else {
            return vec![];
        };
        let mut list = Vec::new();
        for (subject, state) in map.iter() {
            if state.settled > 0 {
                list.push(Calibration {
                    subject: subject.clone(),
                    settled: state.settled,
                    mean_error: state.absolute_error / f64::from(state.settled),
                    bias: state.signed_error / f64::from(state.settled),
                });
            }
        }
        list
    }
}
