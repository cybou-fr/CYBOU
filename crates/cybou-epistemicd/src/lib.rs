// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Epistemic projection and belief validity engine (observation != knowledge).
//!
//! Evaluates incoming observations against historical evidence, maintaining
//! validated epistemic propositions with dispute/staleness metrics.

use std::{
    collections::HashMap,
    sync::RwLock,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

#[cfg(target_os = "linux")]
pub mod service;

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
    /// Whether competing observations currently dispute this belief.
    pub disputed: bool,
}

/// Errors occurring in the epistemic engine.
#[derive(Debug, Error)]
pub enum EpistemicError {
    /// Internal lock poisoned.
    #[error("epistemic lock poisoned")]
    LockPoisoned,
}

/// Core domain logic of the epistemic organ.
pub struct EpistemicCore {
    beliefs: RwLock<HashMap<String, EpistemicBelief>>,
}

impl Default for EpistemicCore {
    fn default() -> Self {
        Self::new()
    }
}

impl EpistemicCore {
    /// Create a new EpistemicCore engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            beliefs: RwLock::new(HashMap::new()),
        }
    }

    /// Ingest an observation into the epistemic belief network.
    pub fn ingest(
        &self,
        subject: impl Into<String>,
        value: impl Into<String>,
        confidence: f64,
        evidence_id: Option<Uuid>,
        now: OffsetDateTime,
    ) {
        let subject_str = subject.into();
        let value_str = value.into();
        if let Ok(mut map) = self.beliefs.write() {
            let entry = map.entry(subject_str.clone()).or_insert_with(|| EpistemicBelief {
                subject: subject_str,
                value: value_str.clone(),
                confidence,
                evidence: Vec::new(),
                last_corroborated_at: now,
                disputed: false,
            });

            if entry.value == value_str {
                entry.confidence = (entry.confidence * 0.7 + confidence * 0.3).clamp(0.0, 1.0);
                entry.last_corroborated_at = now;
                entry.disputed = false;
            } else {
                // Competing value creates dispute
                entry.disputed = true;
                entry.confidence = (entry.confidence * 0.5).clamp(0.0, 1.0);
            }

            if let Some(id) = evidence_id {
                if !entry.evidence.contains(&id) {
                    entry.evidence.push(id);
                }
            }
        }
    }

    /// Query a single belief by subject.
    #[must_use]
    pub fn query(&self, subject: &str) -> Option<EpistemicBelief> {
        self.beliefs.read().ok()?.get(subject).cloned()
    }

    /// Full epistemic projection.
    #[must_use]
    pub fn projection(&self) -> Vec<EpistemicBelief> {
        let map = match self.beliefs.read() {
            Ok(g) => g.clone(),
            Err(_) => return vec![],
        };
        let mut list: Vec<_> = map.into_values().collect();
        list.sort_by(|a, b| a.subject.cmp(&b.subject));
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epistemic_corroboration_and_dispute() {
        let core = EpistemicCore::new();
        let now = OffsetDateTime::now_utc();
        let ev1 = Uuid::new_v4();

        core.ingest("system.os", "Debian 13", 1.0, Some(ev1), now);
        let b1 = core.query("system.os").expect("belief exists");
        assert_eq!(b1.value, "Debian 13");
        assert!(!b1.disputed);

        // Disputing observation reduces confidence and marks disputed
        core.ingest("system.os", "Fedora 40", 0.9, None, now);
        let b2 = core.query("system.os").expect("belief exists");
        assert!(b2.disputed);
        assert!(b2.confidence < 1.0);
    }
}
