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

use cybou_protocol::{canonical::CanonicalEnvelope, Kind};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[cfg(target_os = "linux")]
pub mod service;

/// Structured forecast output returned by `predict`.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Forecast {
    /// Generated contribution ID.
    pub id: Uuid,
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
    /// Create a transient in-memory PredictorCore engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state_path: None,
            cursor: RwLock::new(0),
            by_subject: RwLock::new(HashMap::new()),
        }
    }

    /// Open PredictorCore with persistent JSON storage.
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
        let subject = subject.trim().to_string();
        if subject.is_empty() {
            return;
        }

        let mut candidate = self.by_subject.read().map(|g| g.clone()).unwrap_or_default();
        let state = candidate.entry(subject).or_default();
        state.samples.push(Sample {
            contribution_id,
            value,
        });

        let cur = self.cursor();
        if self.persist_candidate(cur, &candidate).is_ok() {
            if let Ok(mut lock) = self.by_subject.write() {
                *lock = candidate;
            }
        }
    }

    /// Generate a forecast for a subject based on past observed samples.
    ///
    /// # Errors
    ///
    /// Returns [`PredictorError`] if subject is empty or has no history.
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
            id: Uuid::new_v4(),
            subject: subject.to_string(),
            estimate,
            margin,
            confidence,
            samples: count as u32,
        })
    }

    /// Settle a forecast with actual measured outcome (durable before visible).
    pub fn settle(&self, subject: &str, forecast_estimate: f64, actual: f64) {
        let subject = subject.trim().to_string();
        if subject.is_empty() {
            return;
        }

        let mut candidate = self.by_subject.read().map(|g| g.clone()).unwrap_or_default();
        let state = candidate.entry(subject).or_default();
        let error = actual - forecast_estimate;
        state.absolute_error += error.abs();
        state.signed_error += error;
        state.settled += 1;

        let cur = self.cursor();
        if self.persist_candidate(cur, &candidate).is_ok() {
            if let Ok(mut lock) = self.by_subject.write() {
                *lock = candidate;
            }
        }
    }

    /// Replay an envelope from Journal.
    pub fn ingest_envelope(&self, envelope: &CanonicalEnvelope, sequence: u64) {
        let Some(kind) = Kind::from_u16(envelope.kind) else {
            return;
        };

        if kind == Kind::Observation {
            // Check for numeric observation payload
            if let Ok(val) = ciborium::from_reader::<f64, _>(envelope.payload.as_slice()) {
                self.observe(&envelope.origin_organ, val, envelope.message_id);
            }
        }

        if let Ok(mut cur) = self.cursor.write() {
            if sequence > *cur {
                *cur = sequence;
            }
        }
    }

    /// Current journal replay cursor.
    #[must_use]
    pub fn cursor(&self) -> u64 {
        self.cursor.read().map(|g| *g).unwrap_or(0)
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
            mean_error: state.absolute_error / state.settled as f64,
            bias: state.signed_error / state.settled as f64,
        })
    }

    /// Return all calibrated subjects.
    #[must_use]
    pub fn all_calibrations(&self) -> Vec<Calibration> {
        let map = match self.by_subject.read() {
            Ok(g) => g,
            Err(_) => return vec![],
        };
        let mut list = Vec::new();
        for (subject, state) in map.iter() {
            if state.settled > 0 {
                list.push(Calibration {
                    subject: subject.clone(),
                    settled: state.settled,
                    mean_error: state.absolute_error / state.settled as f64,
                    bias: state.signed_error / state.settled as f64,
                });
            }
        }
        list
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

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
