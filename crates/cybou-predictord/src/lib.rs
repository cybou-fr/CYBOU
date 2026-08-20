// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Empirical forecasting, prediction calibration, and outcome settlement.
//!
//! Maintains reconstructible statistical models per subject, mapping predictions
//! against empirical outcomes and preserving calibration history across restarts.

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
use uuid::Uuid;

#[cfg(target_os = "linux")]
pub mod service;

/// Structured forecast output returned by `predict`.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Forecast {
    /// Prediction subject.
    pub subject: String,
    /// Point estimate value.
    pub estimate: f64,
    /// Mean absolute deviation.
    pub margin: f64,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f64,
    /// Number of samples used.
    pub samples: u32,
}

/// Empirical calibration metrics for a subject.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Calibration {
    /// Prediction subject.
    pub subject: String,
    /// Count of settled predictions.
    pub settled: u32,
    /// Mean absolute error.
    pub mean_error: f64,
    /// Signed empirical bias (-1.0 to 1.0).
    pub bias: f64,
}

/// A single numerical observation sample.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    /// Contribution ID of the recorded sample.
    pub contribution_id: Uuid,
    /// Numerical sample value.
    pub value: f64,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SubjectState {
    samples: Vec<Sample>,
    settled: u32,
    absolute_error: f64,
    signed_error: f64,
}

/// Persistent predictor state schema.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct PredictorState {
    cursor: u64,
    subjects: HashMap<String, SubjectState>,
}

/// Errors occurring in the prediction engine.
#[derive(Debug, Error)]
pub enum PredictorError {
    /// Empty subject string.
    #[error("subject must not be empty")]
    EmptySubject,
    /// No historical samples for subject.
    #[error("no history for '{0}' yet")]
    NoHistory(String),
    /// Forecast ID not found.
    #[error("no such forecast in the journal")]
    ForecastNotFound,
    /// Forecast already settled.
    #[error("the forecast is already settled")]
    AlreadySettled,
    /// I/O error.
    #[error("predictor state i/o failed: {0}")]
    Io(#[from] std::io::Error),
    /// Corrupt state.
    #[error("predictor state corrupted: {0}")]
    CorruptState(String),
    /// Internal lock poisoning.
    #[error("internal lock poisoned")]
    LockPoisoned,
}

/// Core domain logic of the predictor organ.
pub struct PredictorCore {
    /// Whether every contribution the Journal already held has been delivered here.
    ///
    /// A forecast made from part of the history is a forecast about a different series than the
    /// one it names.
    caught_up: std::sync::atomic::AtomicBool,
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
        self.caught_up
            .store(true, std::sync::atomic::Ordering::Release);
    }

    /// Whether this projection has seen the whole Journal at least once.
    #[must_use]
    pub fn is_caught_up(&self) -> bool {
        self.caught_up.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Create a transient in-memory `PredictorCore` engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            caught_up: std::sync::atomic::AtomicBool::new(false),
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
            caught_up: std::sync::atomic::AtomicBool::new(false),
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

    /// Record an observation sample together with the Journal position it came from.
    ///
    /// The samples and the cursor are one write. Advancing the cursor separately would let a
    /// restart find a position that claims contributions the samples do not contain, or contain
    /// samples for contributions it will replay again.
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
            // A forecast is about a subject — free disk, battery charge — not about which process
            // happened to write the row down. Keying by origin organ averaged every number one
            // organ ever reported into one meaningless series, and it read the payload as a bare
            // float, which is not what an observation is on the wire, so nothing ever matched.
            && let Some((subject, value)) = observed_measurement(&envelope.payload)
        {
            self.observe_at(&subject, value, envelope.message_id, sequence);
            return;
        }

        // Nothing here to learn from, but the position was still read: persisting it means a
        // restart resumes after it rather than replaying the whole Journal to reach the same
        // samples. It is only recorded once it is on disk with the samples that stand at it.
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

/// The subject and number an observation reports, or `None` when it reports neither.
///
/// A forecast can only be made about something measurable. An observation whose value is text or
/// a flag is a fact about the world, not a series, and is left to the organs that reason about
/// facts rather than being coerced into a number that would forecast nothing.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "the round trip below is exactly the check that neither happened"
)]
fn observed_measurement(payload: &[u8]) -> Option<(String, f64)> {
    let observation: ObservationV1 = ciborium::from_reader(payload).ok()?;
    if observation.subject.is_empty() {
        return None;
    }
    let value = match &observation.value {
        ciborium::Value::Integer(number) => {
            let number = i128::from(*number);
            let approximate = number as f64;
            // A magnitude past what a float represents exactly would be recorded as a different
            // number than the one observed, and every forecast built on it would be about that
            // different number instead.
            if approximate as i128 != number {
                return None;
            }
            approximate
        }
        ciborium::Value::Float(number) if number.is_finite() => *number,
        _ => return None,
    };
    Some((observation.subject, value))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    fn observation_row(subject: &str, value: ciborium::Value, sequence: u64) -> CanonicalEnvelope {
        let observation = ObservationV1 {
            source_id: "test".into(),
            subject: subject.into(),
            value,
            acquired_at: "2026-08-20T00:00:00.000Z".into(),
            freshness_until: "2026-08-21T00:00:00.000Z".into(),
            provenance: "a fixture".into(),
        };
        let mut payload = Vec::new();
        ciborium::into_writer(&observation, &mut payload).expect("encode observation");
        CanonicalEnvelope {
            schema_version: 4,
            message_id: Uuid::from_u128(u128::from(sequence)),
            correlation_id: Uuid::nil(),
            causation_id: Uuid::nil(),
            origin_organ: "cybou-perceptiond".into(),
            origin_node: "node".into(),
            kind: Kind::Observation as u16,
            wall_time_ms: 0,
            monotonic_time: 0,
            logical_clock: sequence,
            confidence: 1.0,
            evidence: Vec::new(),
            payload,
            privacy: 0,
            capability_scope: String::new(),
            sealed: false,
            key_domain_id: Uuid::nil(),
            key_epoch: 0,
            retention_class: 0,
            retention_policy_version: 1,
            retain_until_ms: 0,
            sensitivity: 0,
        }
    }

    #[test]
    fn a_forecast_is_about_what_was_observed_and_survives_where_it_stood() {
        let dir = tempdir().expect("tempdir");
        let state_path = dir.path().join("predictor.json");
        let core = PredictorCore::open(&state_path).expect("open");

        core.ingest_envelope(&observation_row("disk-free", 100.into(), 1), 1);
        core.ingest_envelope(&observation_row("disk-free", 200.into(), 2), 2);
        // Not a measurement: a forecast cannot be made from it, and it must not become one.
        core.ingest_envelope(
            &observation_row("hostname", ciborium::Value::Text("node".into()), 3),
            3,
        );

        // Keyed by what was observed, not by the process that wrote it down.
        let forecast = core.predict("disk-free").expect("a subject with history");
        assert_eq!(forecast.samples, 2);
        assert!((forecast.estimate - 150.0).abs() < 1e-6);
        assert!(core.predict("cybou-perceptiond").is_err());
        assert!(core.predict("hostname").is_err());

        // The position and the samples standing at it are one write: a restart resumes after the
        // row it last learned from, rather than replaying it or skipping past what it never read.
        assert_eq!(core.cursor(), 3);
        let reopened = PredictorCore::open(&state_path).expect("reopen");
        assert_eq!(reopened.cursor(), 3);
        assert_eq!(
            reopened
                .predict("disk-free")
                .expect("history survived")
                .samples,
            2
        );
    }

    #[test]
    fn forecast_and_calibration_persistence() {
        let dir = tempdir().expect("tempdir");
        let state_path = dir.path().join("predictor.json");

        let core = PredictorCore::open(&state_path).expect("open");
        assert!(core.predict("build-time").is_err());

        // Observe samples
        core.observe("build-time", 10.0, Uuid::new_v4());
        core.observe("build-time", 12.0, Uuid::new_v4());
        core.observe("build-time", 14.0, Uuid::new_v4());

        let forecast = core.predict("build-time").expect("predict success");
        assert_eq!(forecast.samples, 3);
        assert!((forecast.estimate - 12.0).abs() < 1e-6);

        // Settle with actual = 13.0
        core.settle("build-time", forecast.estimate, 13.0);

        let cal = core.calibration("build-time").expect("cal exists");
        assert_eq!(cal.settled, 1);
        assert!((cal.mean_error - 1.0).abs() < 1e-6);

        // Reopen from disk: survives restart
        let reopened = PredictorCore::open(&state_path).expect("reopen");
        let reopened_cal = reopened.calibration("build-time").expect("reopened cal");
        assert_eq!(reopened_cal.settled, 1);
    }
}
