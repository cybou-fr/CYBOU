// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Capability health observation, dependency policy, and health snapshots.
//!
//! Evaluates the health of each registered Mind capability by combining component
//! reachability against declarative dependency policy (`CapabilityRegistry`).

use std::{collections::HashMap, sync::RwLock};

use cybou_protocol::{
    CapabilityState, KnowledgeState,
    capability::{CapabilityDeclaration, CapabilityRegistry},
};
use cybou_web_contracts::{CapabilityProjection, Freshness, SnapshotProjection, WEB_SCHEMA_V1};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[cfg(target_os = "linux")]
pub mod service;

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
            Self::Degraded => CapabilityState::Available, // degraded is still available with warning
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

/// Core evaluation engine for capability health and snapshots.
pub struct HealthCore {
    component_records: RwLock<HashMap<String, ComponentHealthRecord>>,
    snapshot: RwLock<Option<SnapshotProjection>>,
    projection_version: RwLock<u64>,
}

impl Default for HealthCore {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthCore {
    /// Create a new `HealthCore` engine.
    #[must_use]
    pub fn new() -> Self {
        let core = Self {
            component_records: RwLock::new(HashMap::new()),
            snapshot: RwLock::new(None),
            projection_version: RwLock::new(0),
        };
        core.recalculate(OffsetDateTime::now_utc());
        core
    }

    /// Update health record for a component and recalculate the aggregate snapshot.
    pub fn update_component(
        &self,
        component_id: impl Into<String>,
        record: ComponentHealthRecord,
        now: OffsetDateTime,
    ) {
        if let Ok(mut map) = self.component_records.write() {
            map.insert(component_id.into(), record);
        }
        self.recalculate(now);
    }

    /// Set all component records in bulk and recalculate.
    pub fn set_components(
        &self,
        records: HashMap<String, ComponentHealthRecord>,
        now: OffsetDateTime,
    ) {
        if let Ok(mut map) = self.component_records.write() {
            *map = records;
        }
        self.recalculate(now);
    }

    /// Evaluate overall health state string ("healthy", "degraded", "unavailable").
    #[must_use]
    pub fn overall_health(&self) -> &'static str {
        let snapshot = match self.snapshot.read() {
            Ok(guard) => guard.clone(),
            Err(_) => return "unavailable",
        };

        let Some(snap) = snapshot else {
            return "unavailable";
        };

        let mut has_unavailable_required = false;
        let mut has_degraded = false;

        for cap in &snap.capabilities {
            // Find declaration
            for decl in CapabilityRegistry::capabilities() {
                if decl.capability_id == cap.id {
                    if decl.required && cap.state == CapabilityState::Unavailable {
                        has_unavailable_required = true;
                    } else if cap.state == CapabilityState::Unavailable
                        || cap.state == CapabilityState::Unknown
                    {
                        has_degraded = true;
                    }
                }
            }
        }

        if has_unavailable_required {
            "unavailable"
        } else if has_degraded {
            "degraded"
        } else {
            "healthy"
        }
    }

    /// Current snapshot projection, if computed.
    #[must_use]
    pub fn current_snapshot(&self) -> Option<SnapshotProjection> {
        self.snapshot.read().ok().and_then(|g| g.clone())
    }

    /// Recalculate capability states based on registered declarations and component records.
    pub fn recalculate(&self, now: OffsetDateTime) {
        let records = self
            .component_records
            .read()
            .map(|g| g.clone())
            .unwrap_or_default();

        let mut capability_projections = Vec::new();

        for decl in CapabilityRegistry::capabilities() {
            let projection = evaluate_capability(decl, &records);
            capability_projections.push(projection);
        }

        let Ok(mut version_guard) = self.projection_version.write() else {
            return;
        };
        *version_guard += 1;
        let new_version = *version_guard;

        let observed_at = now.format(&Rfc3339).unwrap_or_default();

        let snapshot = SnapshotProjection {
            schema_version: WEB_SCHEMA_V1,
            projection_version: new_version,
            cursor: "0".into(),
            observed_at,
            freshness: Freshness::Current,
            knowledge: KnowledgeState::Known,
            capabilities: capability_projections,
        };

        if let Ok(mut snap_guard) = self.snapshot.write() {
            *snap_guard = Some(snapshot);
        }
    }
}

fn evaluate_capability(
    decl: &CapabilityDeclaration,
    records: &HashMap<String, ComponentHealthRecord>,
) -> CapabilityProjection {
    let mut state = CapabilityState::Available;
    let mut reason = None;

    for comp in decl.components {
        match records.get(*comp) {
            None => {
                // If not recorded yet, assume Unavailable
                state = CapabilityState::Unavailable;
                reason = Some(format!(
                    "{comp} is unavailable: {impact}",
                    impact = decl.unavailable_impact
                ));
                break;
            }
            Some(rec) => match rec.health {
                ComponentHealth::Healthy => {}
                ComponentHealth::Degraded => {
                    if state == CapabilityState::Available {
                        reason = rec
                            .detail
                            .clone()
                            .or_else(|| Some(format!("{comp} is degraded")));
                    }
                }
                ComponentHealth::Unavailable | ComponentHealth::Conflicted => {
                    state = CapabilityState::Unavailable;
                    reason = rec.detail.clone().or_else(|| {
                        Some(format!(
                            "{comp} is unavailable: {impact}",
                            impact = decl.unavailable_impact
                        ))
                    });
                    break;
                }
                ComponentHealth::Starting | ComponentHealth::Recovering => {
                    if state != CapabilityState::Unavailable {
                        state = CapabilityState::Unknown;
                        reason = Some(format!("{comp} is starting/recovering"));
                    }
                }
            },
        }
    }

    CapabilityProjection {
        id: decl.capability_id.to_string(),
        state,
        knowledge: KnowledgeState::Known,
        freshness: Freshness::Current,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nominal_health_evaluation() {
        let core = HealthCore::new();
        let now = OffsetDateTime::now_utc();

        // Initially no components reported -> unavailable
        let snap = core.current_snapshot().expect("snapshot exists");
        assert_eq!(core.overall_health(), "unavailable");
        assert_eq!(snap.capabilities.len(), 11);

        // Mark all components healthy
        let mut map = HashMap::new();
        for comp in CapabilityRegistry::component_ids() {
            map.insert(
                comp.to_string(),
                ComponentHealthRecord {
                    health: ComponentHealth::Healthy,
                    detail: None,
                },
            );
        }
        core.set_components(map, now);

        assert_eq!(core.overall_health(), "healthy");
        let snap = core.current_snapshot().expect("snapshot exists");
        for cap in snap.capabilities {
            assert_eq!(cap.state, CapabilityState::Available);
            assert_eq!(cap.knowledge, KnowledgeState::Known);
        }
    }

    #[test]
    fn single_optional_deficit_degrades_gracefully() {
        let core = HealthCore::new();
        let now = OffsetDateTime::now_utc();

        let mut map = HashMap::new();
        for comp in CapabilityRegistry::component_ids() {
            map.insert(
                comp.to_string(),
                ComponentHealthRecord {
                    health: ComponentHealth::Healthy,
                    detail: None,
                },
            );
        }
        // Degrade perceptiond (optional capability)
        map.insert(
            "perceptiond".to_string(),
            ComponentHealthRecord {
                health: ComponentHealth::Unavailable,
                detail: Some("perception sensor absent".into()),
            },
        );

        core.set_components(map, now);
        assert_eq!(core.overall_health(), "degraded");

        let snap = core.current_snapshot().expect("snapshot exists");
        let perception_cap = snap
            .capabilities
            .iter()
            .find(|c| c.id == "local-perception")
            .expect("local-perception exists");
        assert_eq!(perception_cap.state, CapabilityState::Unavailable);
        assert!(
            perception_cap
                .reason
                .as_deref()
                .unwrap()
                .contains("perception sensor absent")
        );
    }
}
