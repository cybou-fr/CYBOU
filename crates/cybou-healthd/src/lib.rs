// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Capability health observation, dependency policy, and health snapshots.
//!
//! Evaluates the health of each registered Mind capability by combining component
//! reachability against declarative dependency policy (`CapabilityRegistry`).

pub mod core;
pub mod types;

#[cfg(target_os = "linux")]
pub mod service;

pub use core::{HealthCore, evaluate_capability};
pub use types::{ComponentHealth, ComponentHealthRecord, HealthError};

#[cfg(test)]
mod tests {
    use cybou_protocol::{CapabilityState, KnowledgeState, capability::CapabilityRegistry};
    use std::collections::HashMap;
    use time::OffsetDateTime;

    use super::*;

    #[test]
    fn the_cursor_moves_only_when_the_projection_does() {
        let core = HealthCore::new();
        let now = OffsetDateTime::now_utc();
        let records: HashMap<_, _> = [(
            "eventd".to_string(),
            ComponentHealthRecord {
                health: ComponentHealth::Healthy,
                detail: None,
            },
        )]
        .into_iter()
        .collect();

        assert!(core.set_components(records.clone(), now));
        let first = core.current_snapshot().expect("a projection was built");

        assert!(!core.set_components(records.clone(), now));
        let unchanged = core.current_snapshot().expect("a projection was built");
        assert_eq!(unchanged.cursor, first.cursor);
        assert_eq!(unchanged.projection_version, first.projection_version);

        let mut changed_records = records;
        changed_records.insert(
            "eventd".to_string(),
            ComponentHealthRecord {
                health: ComponentHealth::Conflicted,
                detail: Some("chain broken".into()),
            },
        );
        assert!(core.set_components(changed_records, now));
        let changed = core.current_snapshot().expect("a projection was built");
        assert_ne!(changed.cursor, first.cursor);
    }

    #[test]
    fn a_conflicted_journal_takes_the_biography_capability_down() {
        let core = HealthCore::new();
        let now = OffsetDateTime::now_utc();

        let healthy = |component: &str| {
            (
                component.to_string(),
                ComponentHealthRecord {
                    health: ComponentHealth::Healthy,
                    detail: None,
                },
            )
        };
        let mut records: HashMap<_, _> = [
            "eventd",
            "identityd",
            "intentiond",
            "predictord",
            "selfd",
            "workspaced",
            "perceptiond",
            "epistemicd",
            "contextd",
            "lifecycled",
            "presenced",
        ]
        .into_iter()
        .map(healthy)
        .collect();
        assert!(core.set_components(records.clone(), now));
        assert_eq!(core.overall_health(), "healthy");

        records.insert(
            "eventd".to_string(),
            ComponentHealthRecord {
                health: ComponentHealth::Conflicted,
                detail: Some("chain broken".into()),
            },
        );
        core.set_components(records, now);
        assert_eq!(core.overall_health(), "unavailable");
    }

    #[test]
    fn nominal_health_evaluation() {
        let core = HealthCore::new();
        let now = OffsetDateTime::now_utc();

        let snap = core.current_snapshot().expect("snapshot exists");
        assert_eq!(core.overall_health(), "unavailable");
        assert_eq!(snap.capabilities.len(), 11);

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
