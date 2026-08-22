// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Statistical forecast, calibration, sample structures, and measurement decoding.

use std::collections::HashMap;

use cybou_protocol::observation::ObservationV1;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

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
pub(crate) struct SubjectState {
    pub(crate) samples: Vec<Sample>,
    pub(crate) settled: u32,
    pub(crate) absolute_error: f64,
    pub(crate) signed_error: f64,
}

/// Persistent predictor state schema.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PredictorState {
    pub(crate) cursor: u64,
    pub(crate) subjects: HashMap<String, SubjectState>,
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

/// The subject and number an observation reports, or `None` when it reports neither.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "the round trip below is exactly the check that neither happened"
)]
#[must_use]
pub fn observed_measurement(payload: &[u8]) -> Option<(String, f64)> {
    let observation: ObservationV1 = ciborium::from_reader(payload).ok()?;
    if observation.subject.is_empty() {
        return None;
    }
    let value = match &observation.value {
        ciborium::Value::Integer(number) => {
            let number = i128::from(*number);
            let approximate = number as f64;
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
