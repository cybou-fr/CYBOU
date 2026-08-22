// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Consumer offsets tracking and validation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Consumer offsets persisted schema.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PersistedOffsets {
    /// Schema format version.
    pub version: u32,
    /// Mapping from consumer name to sequence offset string.
    pub offsets: HashMap<String, String>,
}

/// Validate format of consumer identifier string.
#[must_use]
pub fn is_valid_consumer_id(consumer_id: &str) -> bool {
    if consumer_id.is_empty() || consumer_id.len() > 64 {
        return false;
    }
    let first = consumer_id.chars().next().unwrap_or('\0');
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    consumer_id
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
}
