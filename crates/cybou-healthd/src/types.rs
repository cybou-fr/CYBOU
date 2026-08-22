// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Capability health states, component observation records, and errors.

use cybou_protocol::CapabilityState;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Health of a single system component / daemon.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentHealth {
    /// Fully operational and responsive.
    Healthy,
    /// Responding with deficits or elevated latency.
    Degraded,
    /// Unreachable or unresponsive.
    Unavailable,
    /// Initialization in progress.
    Starting,
    /// Rebuilding internal state.
    Recovering,
    /// Conflicting state observed.
    Conflicted,
}

impl ComponentHealth {
    /// Convert component health to capability state.
    #[must_use]
    #[allow(
        clippy::match_same_arms,
        reason = "Healthy and Degraded map to the same state for different reasons; keep them distinct"
    )]
    pub const fn to_capability_state(self) -> CapabilityState {
        match self {
            Self::Healthy => CapabilityState::Available,
            Self::Degraded => CapabilityState::Available,
            Self::Unavailable => CapabilityState::Unavailable,
            Self::Starting | Self::Recovering => CapabilityState::Unknown,
            Self::Conflicted => CapabilityState::Unavailable,
        }
    }
}

/// Observation record for one component.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ComponentHealthRecord {
    /// Observed health state.
    pub health: ComponentHealth,
    /// Diagnostic detail or error message.
    pub detail: Option<String>,
}

/// Errors occurring in the health evaluation subsystem.
#[derive(Debug, Error)]
pub enum HealthError {
    /// Internal lock poisoning.
    #[error("health core lock poisoned")]
    LockPoisoned,
}
