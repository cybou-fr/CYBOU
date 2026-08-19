// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Predictor1` service implementation on zbus.

use std::sync::Arc;

use uuid::Uuid;
use zbus::interface;

use crate::PredictorCore;

/// D-Bus Service exporting `org.cybou.Mind.Predictor1`.
pub struct Predictor1Service {
    core: Arc<PredictorCore>,
}

impl Predictor1Service {
    /// Create a new Predictor1 D-Bus service handler around `PredictorCore`.
    #[must_use]
    pub fn new(core: Arc<PredictorCore>) -> Self {
        Self { core }
    }
}

#[interface(name = "org.cybou.Mind.Predictor1")]
impl Predictor1Service {
    /// Service readiness.
    async fn ready(&self) -> bool {
        true
    }

    /// Record an empirical observation on a subject with its authentic contribution identity.
    async fn observe(&self, subject: String, value: f64, contribution_id: String) -> bool {
        let Ok(id) = Uuid::parse_str(&contribution_id) else {
            return false;
        };
        self.core.observe(&subject, value, id);
        true
    }

    /// Produce a forecast for a subject and return Forecast CBOR.
    async fn predict(&self, subject: String) -> Vec<u8> {
        let mut buf = Vec::new();
        if let Ok(forecast) = self.core.predict(&subject) {
            let _ = ciborium::into_writer(&forecast, &mut buf);
        }
        buf
    }

    /// Settle a forecast with actual measured outcome.
    async fn settle(&self, subject: String, forecast_estimate: f64, actual: f64) -> bool {
        self.core.settle(&subject, forecast_estimate, actual);
        true
    }

    /// Return Calibration CBOR for a subject.
    async fn calibration(&self, subject: String) -> Vec<u8> {
        let mut buf = Vec::new();
        if let Some(cal) = self.core.calibration(&subject) {
            let _ = ciborium::into_writer(&cal, &mut buf);
        }
        buf
    }

    /// Return all Calibrations CBOR.
    async fn all_calibrations(&self) -> Vec<u8> {
        let list = self.core.all_calibrations();
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&list, &mut buf);
        buf
    }
}
