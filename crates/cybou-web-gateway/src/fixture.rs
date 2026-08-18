// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Deterministic non-production adapter for contract and HTTP tests.

use async_trait::async_trait;
use cybou_web_contracts::SnapshotProjection;

use crate::{GatewayError, PresenceSource};

/// Snapshot source backed by a checked-in deterministic W0 fixture.
#[derive(Clone, Debug)]
pub struct FixturePresenceSource {
    snapshot: SnapshotProjection,
}

impl FixturePresenceSource {
    /// Load the nominal repository fixture.
    ///
    /// # Panics
    ///
    /// Panics when a developer changes the checked-in fixture without updating the typed contract.
    #[must_use]
    pub fn nominal() -> Self {
        let snapshot = serde_json::from_str(include_str!(
            "../../../fixtures/web/v1/snapshot-nominal.json"
        ))
        .expect("checked nominal snapshot fixture");
        Self { snapshot }
    }
}

#[async_trait]
impl PresenceSource for FixturePresenceSource {
    async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError> {
        Ok(self.snapshot.clone())
    }
}
