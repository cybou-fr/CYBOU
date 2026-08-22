// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `HealthCore` evaluation engine and capability dependency mapping.

use std::{collections::HashMap, sync::RwLock};

use cybou_protocol::{
    CapabilityState, KnowledgeState,
    capability::{CapabilityDeclaration, CapabilityRegistry},
};
use cybou_web_contracts::{CapabilityProjection, Freshness, SnapshotProjection, WEB_SCHEMA_V1};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::types::{ComponentHealth, ComponentHealthRecord};

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
        let _ = core.recalculate(OffsetDateTime::now_utc());
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
        let _ = self.recalculate(now);
    }

    /// Set all component records in bulk and recalculate.
    pub fn set_components(
        &self,
        records: HashMap<String, ComponentHealthRecord>,
        now: OffsetDateTime,
    ) -> bool {
        if let Ok(mut map) = self.component_records.write() {
            *map = records;
        }
        self.recalculate(now)
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
    pub fn recalculate(&self, now: OffsetDateTime) -> bool {
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

        let previous_states = self
            .snapshot
            .read()
            .ok()
            .and_then(|guard| guard.as_ref().map(Self::state_fingerprint));
        let new_states = Self::state_fingerprint_of(&capability_projections);
        let changed = previous_states.as_ref() != Some(&new_states);

        let Ok(mut version_guard) = self.projection_version.write() else {
            return false;
        };
        if changed {
            *version_guard += 1;
        }
        let new_version = *version_guard;

        let observed_at = now.format(&Rfc3339).unwrap_or_default();

        let snapshot = SnapshotProjection {
            schema_version: WEB_SCHEMA_V1,
            projection_version: new_version,
            cursor: format!("presence:{new_version}"),
            observed_at,
            freshness: Freshness::Current,
            knowledge: KnowledgeState::Known,
            capabilities: capability_projections,
        };

        if let Ok(mut snap_guard) = self.snapshot.write() {
            *snap_guard = Some(snapshot);
        }

        changed
    }

    fn state_fingerprint(snapshot: &SnapshotProjection) -> Vec<(String, CapabilityState)> {
        Self::state_fingerprint_of(&snapshot.capabilities)
    }

    fn state_fingerprint_of(
        capabilities: &[CapabilityProjection],
    ) -> Vec<(String, CapabilityState)> {
        capabilities
            .iter()
            .map(|cap| (cap.id.clone(), cap.state))
            .collect()
    }
}

/// Evaluate single capability projection based on component records.
#[must_use]
pub fn evaluate_capability<S: std::hash::BuildHasher>(
    decl: &CapabilityDeclaration,
    records: &HashMap<String, ComponentHealthRecord, S>,
) -> CapabilityProjection {
    let mut state = CapabilityState::Available;
    let mut reason = None;

    for comp in decl.components {
        match records.get(*comp) {
            None => {
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
