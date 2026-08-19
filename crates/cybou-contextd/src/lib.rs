// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Associative and situational context management engine (association != truth).
//!
//! Maintains active context vectors, associative graphs between entities and
//! concepts, preventing raw associative proximity from being mistaken for causal truth.

use std::{
    collections::HashMap,
    sync::RwLock,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

#[cfg(target_os = "linux")]
pub mod service;

/// An active situational context element.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextEntry {
    /// Context tag or topic name.
    pub tag: String,
    /// Salience weight in [0.0, 1.0].
    pub weight: f64,
    /// When this context was last activated.
    #[serde(with = "time::serde::rfc3339")]
    pub last_activated_at: OffsetDateTime,
}

/// Errors occurring in the context organ.
#[derive(Debug, Error)]
pub enum ContextError {
    /// Internal lock poisoned.
    #[error("context lock poisoned")]
    LockPoisoned,
}

/// Core domain logic of the associative context organ.
pub struct ContextCore {
    active_tags: RwLock<HashMap<String, ContextEntry>>,
    associations: RwLock<HashMap<String, HashMap<String, f64>>>,
}

impl Default for ContextCore {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextCore {
    /// Create a new ContextCore engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            active_tags: RwLock::new(HashMap::new()),
            associations: RwLock::new(HashMap::new()),
        }
    }

    /// Activate or update a situational context tag.
    pub fn activate(&self, tag: impl Into<String>, weight: f64, now: OffsetDateTime) {
        let tag_str = tag.into();
        if let Ok(mut map) = self.active_tags.write() {
            let entry = map.entry(tag_str.clone()).or_insert_with(|| ContextEntry {
                tag: tag_str,
                weight,
                last_activated_at: now,
            });
            entry.weight = (entry.weight * 0.5 + weight * 0.5).clamp(0.0, 1.0);
            entry.last_activated_at = now;
        }
    }

    /// Link two concepts associatively.
    pub fn associate(&self, a: impl Into<String>, b: impl Into<String>, strength: f64) {
        let a_str = a.into();
        let b_str = b.into();
        if let Ok(mut map) = self.associations.write() {
            map.entry(a_str.clone())
                .or_default()
                .insert(b_str.clone(), strength.clamp(0.0, 1.0));
            map.entry(b_str)
                .or_default()
                .insert(a_str, strength.clamp(0.0, 1.0));
        }
    }

    /// Return active context entries ordered by weight descending.
    #[must_use]
    pub fn active_context(&self) -> Vec<ContextEntry> {
        let map = match self.active_tags.read() {
            Ok(g) => g.clone(),
            Err(_) => return vec![],
        };
        let mut list: Vec<_> = map.into_values().collect();
        list.sort_by(|a, b| b.weight.partial_cmp(&a.weight).unwrap_or(std::cmp::Ordering::Equal));
        list
    }

    /// Return related tags for a given concept.
    #[must_use]
    pub fn related_tags(&self, tag: &str) -> Vec<String> {
        if let Ok(map) = self.associations.read() {
            if let Some(neighbors) = map.get(tag) {
                let mut list: Vec<_> = neighbors.keys().cloned().collect();
                list.sort();
                return list;
            }
        }
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_activation_and_association() {
        let core = ContextCore::new();
        let now = OffsetDateTime::now_utc();

        core.activate("system-maintenance", 0.9, now);
        core.associate("system-maintenance", "backup", 0.8);
        core.associate("system-maintenance", "cleanup", 0.7);

        let active = core.active_context();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].tag, "system-maintenance");

        let related = core.related_tags("system-maintenance");
        assert_eq!(related, vec!["backup".to_string(), "cleanup".to_string()]);
    }
}
