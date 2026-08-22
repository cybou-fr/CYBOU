// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Empirical forecasting, prediction calibration, and outcome settlement.
//!
//! Maintains reconstructible statistical models per subject, mapping predictions
//! against empirical outcomes and preserving calibration history across restarts.

pub mod core;
pub mod types;

#[cfg(target_os = "linux")]
pub mod service;

pub use core::PredictorCore;
pub use types::{Calibration, Forecast, PredictorError, Sample, observed_measurement};

#[cfg(test)]
mod tests {
    use cybou_protocol::{Kind, canonical::CanonicalEnvelope, observation::ObservationV1};
    use tempfile::tempdir;
    use uuid::Uuid;

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
        core.ingest_envelope(
            &observation_row("hostname", ciborium::Value::Text("node".into()), 3),
            3,
        );

        let forecast = core.predict("disk-free").expect("a subject with history");
        assert_eq!(forecast.samples, 2);
        assert!((forecast.estimate - 150.0).abs() < 1e-6);
        assert!(core.predict("cybou-perceptiond").is_err());
        assert!(core.predict("hostname").is_err());

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

        core.observe("build-time", 10.0, Uuid::new_v4());
        core.observe("build-time", 12.0, Uuid::new_v4());
        core.observe("build-time", 14.0, Uuid::new_v4());

        let forecast = core.predict("build-time").expect("predict success");
        assert_eq!(forecast.samples, 3);
        assert!((forecast.estimate - 12.0).abs() < 1e-6);

        core.settle("build-time", forecast.estimate, 13.0);

        let cal = core.calibration("build-time").expect("cal exists");
        assert_eq!(cal.settled, 1);
        assert!((cal.mean_error - 1.0).abs() < 1e-6);

        let reopened = PredictorCore::open(&state_path).expect("reopen");
        let reopened_cal = reopened.calibration("build-time").expect("reopened cal");
        assert_eq!(reopened_cal.settled, 1);
    }
}
