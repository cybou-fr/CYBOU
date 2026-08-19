// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Self-assessment, continuous self-model, and autobiographical narration.
//!
//! Answers "who am I, what do I owe, and how calibrated have my predictions been?"
//! producing verifiable self-assessment contributions for the Journal.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

#[cfg(target_os = "linux")]
pub mod service;

/// Calibration record for one prediction subject.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationEntry {
    /// Prediction subject.
    pub subject: String,
    /// Number of settled predictions on this subject.
    pub settled: u32,
    /// Signed empirical bias (-1.0 to 1.0).
    pub bias: f64,
}

/// A point-in-time assessment of the cognitive system by itself.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfReport {
    /// Instant when the assessment was taken.
    #[serde(with = "time::serde::rfc3339")]
    pub taken: OffsetDateTime,

    /// Age of the identity in whole days.
    pub age_in_days: i64,
    /// Total sessions recorded by identity.
    pub sessions: u64,
    /// Architecture version string.
    pub architecture_version: String,

    /// Open/unfulfilled obligations count.
    pub open_intentions: u32,
    /// Age in days of the oldest unfulfilled obligation.
    pub oldest_obligation_days: i64,
    /// Whether obligations were successfully read.
    pub obligations_known: bool,

    /// Whether calibrations were successfully read.
    pub calibrations_known: bool,
    /// Prediction calibrations.
    pub calibrations: Vec<CalibrationEntry>,
    /// Total settled predictions.
    pub settled_predictions: u32,

    /// Total accepted contributions in the Journal.
    pub contributions: u64,
    /// Whether cryptographic verification succeeded.
    pub journal_intact: bool,
    /// First broken sequence if integrity verification failed, or 0.
    pub first_broken_at: u64,
}

impl Default for SelfReport {
    fn default() -> Self {
        Self {
            taken: OffsetDateTime::UNIX_EPOCH,
            age_in_days: 0,
            sessions: 0,
            architecture_version: String::new(),
            open_intentions: 0,
            oldest_obligation_days: 0,
            obligations_known: false,
            calibrations_known: false,
            calibrations: vec![],
            settled_predictions: 0,
            contributions: 0,
            journal_intact: true,
            first_broken_at: 0,
        }
    }
}

impl SelfReport {
    /// Whether the report is structurally valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.taken != OffsetDateTime::UNIX_EPOCH
    }
}

/// Render a human-readable first-person narrative from a self report.
#[must_use]
pub fn narrate_self_report(report: &SelfReport) -> String {
    if !report.is_valid() {
        return "I cannot see myself clearly enough to say.".to_string();
    }

    let mut lines = Vec::new();

    // 1. Identity & Age
    if report.age_in_days <= 0 {
        lines.push(format!(
            "This is my first day. This is session {}.",
            report.sessions
        ));
    } else {
        lines.push(format!(
            "I am {} day(s) old. This is session {}.",
            report.age_in_days, report.sessions
        ));
    }

    // 2. Intentions & Obligations
    if report.open_intentions == 0 {
        lines.push("I owe you nothing right now.".to_string());
    } else if report.oldest_obligation_days >= 1 {
        lines.push(format!(
            "I owe you {} thing(s); the oldest has been waiting {} day(s).",
            report.open_intentions, report.oldest_obligation_days
        ));
    } else {
        lines.push(format!("I owe you {} thing(s).", report.open_intentions));
    }

    // 3. Calibrations & Predictions
    if report.settled_predictions == 0 {
        lines.push("I have not yet been tested against anything I predicted.".to_string());
    } else {
        lines.push(format!(
            "I have checked myself against reality {} time(s).",
            report.settled_predictions
        ));

        let mut worst_bias: f64 = 0.0;
        let mut worst_subject = "";
        for cal in &report.calibrations {
            if cal.bias.abs() > worst_bias.abs() {
                worst_bias = cal.bias;
                worst_subject = &cal.subject;
            }
        }

        if !worst_subject.is_empty() && worst_bias.abs() > 0.0 {
            if worst_bias > 0.0 {
                lines.push(format!("On {worst_subject} I tend to be optimistic."));
            } else {
                lines.push(format!("On {worst_subject} I tend to overestimate."));
            }
        }
    }

    // 4. Memory & Journal integrity
    if !report.journal_intact {
        lines.push(format!(
            "My memory is damaged from record {} onward.",
            report.first_broken_at
        ));
    }

    lines.join("\n")
}

/// Errors during self-model assessment.
#[derive(Debug, Error)]
pub enum SelfError {
    /// Assessment cause does not exist.
    #[error("assessment cause does not exist")]
    CauseMissing,
    /// Self model is uninitialized.
    #[error("self model is uninitialized")]
    Uninitialized,
}

/// Core domain logic of the self organ.
pub struct SelfCore {
    age_in_days: i64,
    sessions: u64,
    architecture_version: String,
}

impl SelfCore {
    /// Create a new SelfCore manager.
    #[must_use]
    pub fn new(age_in_days: i64, sessions: u64, architecture_version: impl Into<String>) -> Self {
        Self {
            age_in_days,
            sessions,
            architecture_version: architecture_version.into(),
        }
    }

    /// Measure the current self-state snapshot.
    #[must_use]
    pub fn measure(&self, now: OffsetDateTime, contributions: u64) -> SelfReport {
        SelfReport {
            taken: now,
            age_in_days: self.age_in_days,
            sessions: self.sessions,
            architecture_version: self.architecture_version.clone(),
            open_intentions: 0,
            oldest_obligation_days: 0,
            obligations_known: true,
            calibrations_known: true,
            calibrations: vec![],
            settled_predictions: 0,
            contributions,
            journal_intact: true,
            first_broken_at: 0,
        }
    }

    /// Produce a narrated self-reflection string.
    #[must_use]
    pub fn narrate(&self, report: &SelfReport) -> String {
        narrate_self_report(report)
    }
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use super::*;

    #[test]
    fn narration_first_day_clean_state() {
        let report = SelfReport {
            taken: OffsetDateTime::now_utc(),
            age_in_days: 0,
            sessions: 1,
            architecture_version: "debian-rust-1.0".into(),
            open_intentions: 0,
            oldest_obligation_days: 0,
            obligations_known: true,
            calibrations_known: true,
            calibrations: vec![],
            settled_predictions: 0,
            contributions: 5,
            journal_intact: true,
            first_broken_at: 0,
        };

        let text = narrate_self_report(&report);
        assert!(text.contains("This is my first day. This is session 1."));
        assert!(text.contains("I owe you nothing right now."));
        assert!(text.contains("I have not yet been tested against anything I predicted."));
    }

    #[test]
    fn narration_with_obligations_and_calibrations() {
        let report = SelfReport {
            taken: OffsetDateTime::now_utc(),
            age_in_days: 42,
            sessions: 15,
            architecture_version: "debian-rust-1.0".into(),
            open_intentions: 3,
            oldest_obligation_days: 5,
            obligations_known: true,
            calibrations_known: true,
            calibrations: vec![CalibrationEntry {
                subject: "build-duration".into(),
                settled: 8,
                bias: 0.25,
            }],
            settled_predictions: 8,
            contributions: 120,
            journal_intact: true,
            first_broken_at: 0,
        };

        let text = narrate_self_report(&report);
        assert!(text.contains("I am 42 day(s) old. This is session 15."));
        assert!(text.contains("I owe you 3 thing(s); the oldest has been waiting 5 day(s)."));
        assert!(text.contains("I have checked myself against reality 8 time(s)."));
        assert!(text.contains("On build-duration I tend to be optimistic."));
    }
}
