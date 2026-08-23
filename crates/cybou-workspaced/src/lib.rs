// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Global Workspace Theory attention coalitions and focus selection.
//!
//! Groups recent cognitive contributions into episodic coalitions by correlation ID,
//! calculates dynamic salience with exponential half-life recency decay,
//! and determines current attentional focus.

use std::{
    collections::{HashMap, HashSet},
    sync::RwLock,
};

use cybou_protocol::{Kind, canonical::CanonicalEnvelope, unix_millis};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

pub mod admission;

#[cfg(target_os = "linux")]
pub mod service;

pub use admission::{Admission, AttentionProposal, admit, proposal_quota};

const HALF_LIFE_SECONDS: f64 = 120.0;

/// Return attention weight for a given contribution kind.
#[must_use]
pub fn attention_weight(kind: Kind) -> f64 {
    match kind {
        Kind::NeedSignal | Kind::Objection => 3.0,
        Kind::Decision | Kind::Intention => 2.0,
        Kind::Outcome | Kind::SelfAssessment | Kind::AttentionCandidate => 1.5,
        Kind::Prediction | Kind::PlanProposal | Kind::Hypothesis | Kind::BeliefRevision => 1.0,
        _ => 0.5,
    }
}

/// Return attention weight for a raw u16 kind.
#[must_use]
pub fn attention_weight_u16(kind_u16: u16) -> f64 {
    Kind::from_u16(kind_u16).map_or(0.5, attention_weight)
}

/// A cluster of related cognitive contributions competing for conscious workspace focus.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Coalition {
    /// Episode correlation identity.
    pub correlation_id: Uuid,
    /// Contributing envelopes in chronological order.
    pub members: Vec<CanonicalEnvelope>,
    /// Calculated dynamic salience.
    pub salience: f64,
    /// Wall time ms of latest member.
    pub latest_ms: i64,
    /// Distinct organs participating in this coalition.
    pub organs: Vec<String>,
}

impl Coalition {
    /// Number of contributions in this thread.
    #[must_use]
    pub fn thread_count(&self) -> usize {
        self.members.len()
    }

    /// Whether this coalition is structurally valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.correlation_id.is_nil() && !self.members.is_empty()
    }
}

/// Snapshot of the conscious moment in the workspace.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MomentState {
    /// Current winning focus correlation ID if any.
    pub focus: Option<Uuid>,
    /// Salience score of the current focus.
    pub salience: f64,
    /// Organs active in the current focus.
    pub organs: Vec<String>,
}

/// Errors occurring in the workspace engine.
#[derive(Debug, Error)]
pub enum WorkspaceError {
    /// Internal lock poisoned.
    #[error("workspace lock poisoned")]
    LockPoisoned,
}

/// Core domain logic of the global workspace organ.
pub struct WorkspaceCore {
    /// Whether the tail of the Journal has been reached at least once.
    caught_up: std::sync::atomic::AtomicBool,
    capacity: usize,
    moment: RwLock<Vec<CanonicalEnvelope>>,
    last_focus: RwLock<Option<Uuid>>,
}

impl Default for WorkspaceCore {
    fn default() -> Self {
        Self::new(32)
    }
}

impl WorkspaceCore {
    /// Record that every contribution the Journal already held has now been delivered.
    pub fn mark_caught_up(&self) {
        self.caught_up
            .store(true, std::sync::atomic::Ordering::Release);
    }

    /// Whether this projection has seen the whole Journal at least once.
    #[must_use]
    pub fn is_caught_up(&self) -> bool {
        self.caught_up.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Create a new `WorkspaceCore` with bounded capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            caught_up: std::sync::atomic::AtomicBool::new(false),
            capacity: capacity.max(1),
            moment: RwLock::new(Vec::new()),
            last_focus: RwLock::new(None),
        }
    }

    /// Capacity limit of the workspace buffer.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Last observed winning focus UUID.
    #[must_use]
    pub fn last_focus(&self) -> Option<Uuid> {
        self.last_focus.read().ok().and_then(|g| *g)
    }

    /// Accept a newly committed contribution into the short-term conscious moment.
    pub fn accept(&self, envelope: CanonicalEnvelope) {
        if envelope.message_id.is_nil() {
            return;
        }

        if let Ok(mut moment) = self.moment.write() {
            if moment.iter().any(|e| e.message_id == envelope.message_id) {
                return;
            }
            moment.insert(0, envelope);
            if moment.len() > self.capacity {
                moment.truncate(self.capacity);
            }
        }
    }

    /// Offer proposals to the moment, admitting only what may enter it.
    ///
    /// Nothing here calls [`Self::accept`]. That is the whole point: `accept` takes something that
    /// happened and makes room for it by dropping the oldest, which is right for a contribution and
    /// catastrophic for a proposal — a thousand associations would stay within every declared
    /// bound and leave the moment holding nothing but associations. What comes back is a decision
    /// the caller can act on, count, or show.
    #[must_use]
    pub fn consider(&self, proposals: &[AttentionProposal]) -> Admission {
        let occupied = self
            .moment
            .read()
            .map_or(self.capacity, |moment| moment.len());
        admit(proposals, self.capacity, occupied)
    }

    /// Calculate dynamic salience for a coalition at a given moment.
    ///
    /// Salience is a ranking heuristic, not a stored quantity: ages and member counts are
    /// converted to `f64` for the decay curve, where losing sub-millisecond precision on
    /// implausibly large values cannot change which coalition wins focus.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        reason = "attention ranking tolerates f64 rounding of ages and counts"
    )]
    pub fn salience_of(&self, coalition: &Coalition, now: OffsetDateTime) -> f64 {
        let now_ms = unix_millis(now);
        let mut total = 0.0;

        for env in &coalition.members {
            let age_seconds = (now_ms - env.wall_time_ms).max(0) as f64 / 1000.0;
            let recency = 0.5_f64.powf(age_seconds / HALF_LIFE_SECONDS);
            let weight = attention_weight_u16(env.kind);
            total += weight * env.confidence * recency;
        }

        let distinct_organs = coalition.organs.len().max(1);
        total * (distinct_organs as f64).sqrt()
    }

    /// Group all recent envelopes into coalitions and sort by salience descending.
    #[must_use]
    pub fn coalitions(&self, now: OffsetDateTime) -> Vec<Coalition> {
        let moment = match self.moment.read() {
            Ok(g) => g.clone(),
            Err(_) => return vec![],
        };

        let mut map: HashMap<Uuid, Vec<CanonicalEnvelope>> = HashMap::new();

        // Iterate in reverse (oldest to newest) to preserve member order
        for env in moment.iter().rev() {
            let key = if env.correlation_id.is_nil() {
                env.message_id
            } else {
                env.correlation_id
            };
            map.entry(key).or_default().push(env.clone());
        }

        let mut result = Vec::new();
        for (key, members) in map {
            let mut organs_set = HashSet::new();
            let mut latest_ms = 0;
            for m in &members {
                if !m.origin_organ.is_empty() {
                    organs_set.insert(m.origin_organ.clone());
                }
                if m.wall_time_ms > latest_ms {
                    latest_ms = m.wall_time_ms;
                }
            }
            let mut organs: Vec<String> = organs_set.into_iter().collect();
            organs.sort();

            let mut coalition = Coalition {
                correlation_id: key,
                members,
                salience: 0.0,
                latest_ms,
                organs,
            };
            coalition.salience = self.salience_of(&coalition, now);
            result.push(coalition);
        }

        result.sort_by(|a, b| {
            b.salience
                .partial_cmp(&a.salience)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.latest_ms.cmp(&a.latest_ms))
        });

        result
    }

    /// Return the winning attentional focus coalition, if any.
    #[must_use]
    pub fn focus(&self, now: OffsetDateTime) -> Option<Coalition> {
        self.coalitions(now).into_iter().next()
    }

    /// Snapshot of current conscious moment.
    #[must_use]
    pub fn moment_state(&self, now: OffsetDateTime) -> MomentState {
        let f = self.focus(now);
        match f {
            Some(c) => MomentState {
                focus: Some(c.correlation_id),
                salience: c.salience,
                organs: c.organs,
            },
            None => MomentState {
                focus: None,
                salience: 0.0,
                organs: vec![],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use super::*;

    fn make_envelope(
        message_id: Uuid,
        correlation_id: Uuid,
        origin_organ: &str,
        kind: u16,
        confidence: f64,
        wall_time_ms: i64,
    ) -> CanonicalEnvelope {
        CanonicalEnvelope {
            schema_version: 3,
            message_id,
            correlation_id,
            causation_id: Uuid::nil(),
            origin_organ: origin_organ.to_string(),
            origin_node: String::new(),
            kind,
            wall_time_ms,
            monotonic_time: 100,
            logical_clock: 1,
            confidence,
            evidence: vec![],
            payload: vec![],
            privacy: 1,
            capability_scope: String::new(),
            sealed: false,
            key_domain_id: Uuid::nil(),
            key_epoch: 0,
            retention_class: 2,
            retention_policy_version: 0,
            retain_until_ms: 0,
            sensitivity: 1,
        }
    }

    #[test]
    fn a_flood_of_associations_does_not_cost_the_workspace_its_focus() {
        // ADR-0029 A11, at the seam where it actually lives. The workspace was answering a
        // NeedSignal; five thousand things came to mind; it is still answering the NeedSignal.
        let core = WorkspaceCore::new(32);
        let now = OffsetDateTime::now_utc();
        let now_ms = unix_millis(now);
        let urgent = Uuid::new_v4();
        core.accept(make_envelope(
            Uuid::new_v4(),
            urgent,
            "healthd",
            5,
            1.0,
            now_ms,
        ));

        let proposals: Vec<AttentionProposal> = (0..5000)
            .map(|index| AttentionProposal {
                label: format!("concept-{index:04}"),
                relevance: 0.9,
                reason: "lemon → something".to_owned(),
            })
            .collect();
        let admission = core.consider(&proposals);

        assert!(admission.admitted.len() <= proposal_quota(32));
        assert!(
            !admission.complete,
            "five thousand did not all fit, and it says so"
        );
        // The moment is untouched: considering is not accepting, and this is the whole distinction.
        assert_eq!(
            core.focus(now).expect("focus survives").correlation_id,
            urgent
        );
        assert_eq!(core.coalitions(now).len(), 1);
    }

    #[test]
    fn what_the_same_flood_would_have_done_through_the_contribution_path() {
        // Kept as an executable statement of why the two paths must differ. `accept` is right for
        // something that happened: it makes room by dropping the oldest. Run a flood through it and
        // the workspace is exactly as bounded as it promised and has lost the NeedSignal entirely.
        let core = WorkspaceCore::new(32);
        let now = OffsetDateTime::now_utc();
        let now_ms = unix_millis(now);
        let urgent = Uuid::new_v4();
        core.accept(make_envelope(
            Uuid::new_v4(),
            urgent,
            "healthd",
            5,
            1.0,
            now_ms,
        ));

        for _ in 0..5000 {
            core.accept(make_envelope(
                Uuid::new_v4(),
                Uuid::new_v4(),
                "contextd",
                1,
                0.9,
                now_ms,
            ));
        }

        assert_eq!(core.coalitions(now).len(), 32, "bounded, as promised");
        assert_ne!(
            core.focus(now)
                .expect("something is in focus")
                .correlation_id,
            urgent,
            "and the thing worth attending to is gone — which is why proposals do not come this way"
        );
    }

    #[test]
    fn attention_focus_competition() {
        let core = WorkspaceCore::new(10);
        let now = OffsetDateTime::now_utc();
        let now_ms = unix_millis(now);

        let ep1 = Uuid::new_v4();
        let ep2 = Uuid::new_v4();

        // Episode 1 has low priority Observation
        let env1 = make_envelope(Uuid::new_v4(), ep1, "perceptiond", 1, 1.0, now_ms);
        core.accept(env1);

        // Episode 2 has high priority NeedSignal from 2 organs
        let env2_health = make_envelope(Uuid::new_v4(), ep2, "healthd", 5, 1.0, now_ms);
        let env2_self = make_envelope(Uuid::new_v4(), ep2, "selfd", 5, 1.0, now_ms);
        core.accept(env2_health);
        core.accept(env2_self);

        let coalitions = core.coalitions(now);
        assert_eq!(coalitions.len(), 2);
        assert_eq!(coalitions[0].correlation_id, ep2); // ep2 wins focus!

        let focus = core.focus(now).expect("focus exists");
        assert_eq!(focus.correlation_id, ep2);
        assert_eq!(
            focus.organs,
            vec!["healthd".to_string(), "selfd".to_string()]
        );
    }
}
