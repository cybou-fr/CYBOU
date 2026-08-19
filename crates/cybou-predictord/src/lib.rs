// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Empirical forecasting, prediction calibration, and outcome settlement.

use std::{
    collections::HashMap,
    sync::RwLock,
};

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
#[derive(Clone, Debug)]
pub struct Sample {
    /// Contribution ID of the recorded sample.
    pub contribution_id: Uuid,
    /// Numerical sample value.
    pub value: f64,
}

#[derive(Clone, Debug, Default)]
struct SubjectState {
    samples: Vec<Sample>,
    settled: u32,
    absolute_error: f64,
    signed_error: f64,
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
    /// Internal lock poisoning.
    #[error("internal lock poisoned")]
    LockPoisoned,
}

/// Core domain logic of the predictor organ.
pub struct PredictorCore {
    by_subject: RwLock<HashMap<String, SubjectState>>,
}

impl Default for PredictorCore {
    fn default() -> Self {
        Self::new()
    }
}

impl PredictorCore {
    /// Create a new PredictorCore engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_subject: RwLock::new(HashMap::new()),
        }
    }

    /// Record an observation sample for a subject.
    pub fn observe(&self, subject: &str, value: f64, contribution_id: Uuid) {
        let subject = subject.trim().to_string();
        if subject.is_empty() {
            return;
        }
        if let Ok(mut map) = self.by_subject.write() {
            let state = map.entry(subject).or_default();
            state.samples.push(Sample {
                contribution_id,
                value,
            });
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

    /// Settle a forecast with actual measured outcome.
    pub fn settle(&self, subject: &str, forecast_estimate: f64, actual: f64) {
        let subject = subject.trim();
        if let Ok(mut map) = self.by_subject.write() {
            let state = map.entry(subject.to_string()).or_default();
            let error = actual - forecast_estimate;
            state.absolute_error += error.abs();
            state.signed_error += error;
            state.settled += 1;
        }
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
    use super::*;

    #[test]
    fn forecast_and_calibration_lifecycle() {
        let core = PredictorCore::new();
        assert!(core.predict("build-time").is_err());

        // Observe samples
        core.observe("build-time", 10.0, Uuid::new_v4());
        core.observe("build-time", 12.0, Uuid::new_v4());
        core.observe("build-time", 14.0, Uuid::new_v4());

        let forecast = core.predict("build-time").expect("predict success");
        assert_eq!(forecast.samples, 3);
        assert!((forecast.estimate - 12.0).abs() < 1e-6);
        assert!((forecast.confidence - (3.0 / 6.0)).abs() < 1e-6);

        // Settle with actual = 13.0
        core.settle("build-time", forecast.estimate, 13.0);

        let cal = core.calibration("build-time").expect("cal exists");
        assert_eq!(cal.settled, 1);
        assert!((cal.mean_error - 1.0).abs() < 1e-6);
        assert!((cal.bias - 1.0).abs() < 1e-6);
    }
}
