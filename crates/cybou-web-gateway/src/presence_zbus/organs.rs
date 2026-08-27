// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! One reader per organ, and the rule they all follow.
//!
//! Every section of the Mind projection is optional because the owners are separate processes that
//! fail separately. A reader that could not reach its organ returns the shape that says so rather
//! than an error, so one silent organ leaves a gap on the page instead of taking the rest of it
//! down. That is a projection decision, not error handling.

use super::ZbusPresenceSource;
use super::wire::{
    OwnerBelief, OwnerConcept, OwnerIntention, OwnerLifecycle, OwnerMomentState,
    OwnerPerceptionState, OwnerSelfReport, OwnerVerification, PERSONAL, RECENT_CONTRIBUTIONS,
    kind_name, millis_to_rfc3339,
};
use cybou_fabric::{
    ACTION, CONTEXT, EPISTEMIC, EVENT, IDENTITY, INTENTION, LIFECYCLE, PERCEPTION, SELF, TELEMETRY,
    WORKSPACE, decode,
};
use cybou_protocol::{KnowledgeState, canonical::CanonicalEnvelope, disclosure::WithheldBecause};
use cybou_web_contracts::{
    AttentionProjection, BeliefProjection, BeliefsProjection, CommitmentProjection,
    CommitmentsProjection, ConceptProjection, ContextProjection, ContributionProjection,
    IdentityProjection, JournalProjection, LifecycleProjection, PerceptionProjection,
    SelfProjection,
};

impl ZbusPresenceSource {
    /// What the host currently makes of itself.
    ///
    /// Not filtered by sensitivity, and that is a decision rather than an oversight. A reading is
    /// about the machine, not about the person: that memory pressure is high is a fact about a
    /// server in the same way its kernel version is. What keeps it from a stranger is the route,
    /// which is gated with the rest of Mind — and the route is the right place, because the whole
    /// answer is either for this reader or not.
    pub(super) async fn insight(&self) -> cybou_web_contracts::InsightProjection {
        let Some(encoded) = self.read::<Vec<u8>>(TELEMETRY, "Insights").await else {
            // The organ did not answer. Distinct from a host with nothing to report, and the
            // projection carries that distinction itself.
            return crate::insight::unread();
        };
        let Ok(insights) = ciborium::from_reader::<Vec<cybou_protocol::telemetry::SystemInsight>, _>(
            encoded.as_slice(),
        ) else {
            return crate::insight::unread();
        };

        // Every watched thing and what is known about it, rather than the readings that worked. A
        // surface built from the readings alone cannot tell a certificate nobody declared from one
        // that was declared and never read, and those are opposites.
        let watched = self
            .read::<Vec<u8>>(TELEMETRY, "Watching")
            .await
            .and_then(|encoded| {
                ciborium::from_reader::<Vec<cybou_protocol::telemetry::WatchedResource>, _>(
                    encoded.as_slice(),
                )
                .ok()
            })
            .unwrap_or_default();

        // Readiness is what the organ says about whether it has a notion of ordinary yet, and it is
        // asked separately rather than inferred from the absence of findings. A quiet host and a
        // host that has not watched long enough both report no findings, and only one of them is
        // entitled to an all-clear.
        let watched_enough = self.read::<bool>(TELEMETRY, "Ready").await.unwrap_or(false);

        let projections = self
            .read::<Vec<u8>>(TELEMETRY, "Projections")
            .await
            .and_then(|encoded| {
                ciborium::from_reader::<
                    Vec<(
                        cybou_protocol::telemetry::MetricKey,
                        cybou_telemetryd::trend::Projection,
                    )>,
                    _,
                >(encoded.as_slice())
                .ok()
            })
            .unwrap_or_default();

        crate::insight::project(
            &insights,
            &watched,
            watched_enough,
            &projections,
            time::OffsetDateTime::now_utc(),
        )
    }

    pub(super) async fn identity(&self) -> IdentityProjection {
        let identity_id: Option<String> = self.read(IDENTITY, "IdentityId").await;
        let Some(identity_id) = identity_id.filter(|value| !value.is_empty()) else {
            return IdentityProjection {
                knowledge: KnowledgeState::Unknown,
                identity_id: None,
                origin: None,
                session_count: None,
                age_in_days: None,
                architecture_version: None,
            };
        };
        IdentityProjection {
            knowledge: KnowledgeState::Known,
            identity_id: Some(identity_id),
            origin: self.read(IDENTITY, "Origin").await,
            session_count: self.read(IDENTITY, "SessionCount").await,
            age_in_days: self.read(IDENTITY, "AgeInDays").await,
            architecture_version: self.read(IDENTITY, "ArchitectureVersion").await,
        }
    }

    pub(super) async fn journal(&self) -> JournalProjection {
        let Some(count) = self.read::<u64>(EVENT, "Count").await else {
            return JournalProjection {
                knowledge: KnowledgeState::Unknown,
                contribution_count: None,
                erasure_epoch: None,
                recent: Vec::new(),
                integrity: None,
            };
        };
        JournalProjection {
            knowledge: KnowledgeState::Known,
            contribution_count: Some(count),
            erasure_epoch: self.read(EVENT, "ErasureEpoch").await,
            recent: self.recent_contributions().await,
            integrity: self.integrity().await,
        }
    }

    /// How far the chain has been verified, stated as a position rather than a verdict.
    ///
    /// "Verified" without a position would claim something about rows nobody replayed, so a pass
    /// still catching up says how far it got.
    pub(super) async fn integrity(&self) -> Option<String> {
        let encoded = self.read::<Vec<u8>>(EVENT, "Verification").await?;
        if encoded.is_empty() {
            return None;
        }
        let state = ciborium::from_reader::<OwnerVerification, _>(encoded.as_slice()).ok()?;
        Some(match state.broken_at {
            Some(broken_at) => format!("chain broken at {broken_at}"),
            None if state.verified_through >= state.head => {
                format!("verified through {}", state.verified_through)
            }
            None => format!(
                "verified through {} of {}",
                state.verified_through, state.head
            ),
        })
    }

    /// The tail of the Journal, newest first.
    ///
    /// Event1 returns the rows in stored order; a reader watching a system live wants the newest
    /// line at the top, so the order is reversed here rather than left for the page to guess.
    pub(super) async fn recent_contributions(&self) -> Vec<ContributionProjection> {
        let Some(encoded) = self
            .read_with::<Vec<u8>, _>(EVENT, "Recent", &(RECENT_CONTRIBUTIONS,))
            .await
        else {
            return Vec::new();
        };
        if encoded.is_empty() {
            return Vec::new();
        }
        let Ok(envelopes) = ciborium::from_reader::<Vec<CanonicalEnvelope>, _>(encoded.as_slice())
        else {
            return Vec::new();
        };
        let mut rows: Vec<ContributionProjection> = envelopes
            .into_iter()
            .map(|envelope| ContributionProjection {
                message_id: envelope.message_id.to_string(),
                kind: kind_name(envelope.kind),
                origin_organ: envelope.origin_organ,
                recorded_at: millis_to_rfc3339(envelope.wall_time_ms),
            })
            .collect();
        rows.reverse();
        rows
    }

    pub(super) async fn commitments(&self) -> CommitmentsProjection {
        let Some(encoded) = self.read::<Vec<u8>>(INTENTION, "Open").await else {
            return CommitmentsProjection {
                knowledge: KnowledgeState::Unknown,
                open_count: None,
                open: Vec::new(),
            };
        };
        // An owner that answered with an empty body holds no open obligations; that is a known
        // empty list, not an unreachable owner.
        let open: Vec<OwnerIntention> = if encoded.is_empty() {
            Vec::new()
        } else {
            match ciborium::from_reader(encoded.as_slice()) {
                Ok(open) => open,
                Err(_) => {
                    return CommitmentsProjection {
                        knowledge: KnowledgeState::Unknown,
                        open_count: None,
                        open: Vec::new(),
                    };
                }
            }
        };
        // An obligation is something a person committed to, in their words. There is no class to
        // consult here and there does not need to be: a promise is about the person by
        // construction, which is why recording one raises what the Journal carries. A reader who
        // may only see ordinary things is told how many there are and not what they say — the
        // count is a fact about the system, the descriptions are the person's.
        if self.permitted_sensitivity < PERSONAL {
            for _ in &open {
                // Named by nothing at all. What a person promised is their words, and a subject
                // here would be those words: the count is the fact about the system, and the
                // descriptions are the person's.
                self.note_withheld(None, WithheldBecause::BelongsToThePerson);
            }
            return CommitmentsProjection {
                knowledge: KnowledgeState::Known,
                open_count: self.read(INTENTION, "OpenCount").await,
                open: Vec::new(),
            };
        }
        CommitmentsProjection {
            knowledge: KnowledgeState::Known,
            open_count: self.read(INTENTION, "OpenCount").await,
            open: open
                .into_iter()
                .map(|item| CommitmentProjection {
                    id: item.id.to_string(),
                    description: item.description,
                    trigger: item.trigger,
                    formed: item.formed,
                })
                .collect(),
        }
    }

    /// Self1's own assessment, including the sentence it composes about itself.
    ///
    /// Measure and Narrate are two calls on purpose: the narration must come from the owner, so
    /// the gateway hands the report straight back rather than describing it in its own words.
    pub(super) async fn self_model(&self) -> SelfProjection {
        let unread = || SelfProjection {
            knowledge: KnowledgeState::Unknown,
            narration: None,
            age_in_days: None,
            sessions: None,
            open_intentions: None,
            settled_predictions: None,
        };
        let Some(encoded) = self.read::<Vec<u8>>(SELF, "Measure").await else {
            return unread();
        };
        let Ok(report) = ciborium::from_reader::<OwnerSelfReport, _>(encoded.as_slice()) else {
            return unread();
        };
        SelfProjection {
            knowledge: KnowledgeState::Known,
            narration: self
                .read_with::<String, _>(SELF, "Narrate", &(encoded,))
                .await
                .filter(|narration| !narration.is_empty()),
            age_in_days: Some(report.age_in_days),
            sessions: Some(report.sessions),
            open_intentions: Some(report.open_intentions),
            settled_predictions: Some(report.settled_predictions),
        }
    }

    pub(super) async fn attention(&self) -> AttentionProjection {
        let Some(encoded) = self.read::<Vec<u8>>(WORKSPACE, "MomentState").await else {
            return AttentionProjection {
                knowledge: KnowledgeState::Unknown,
                focus: None,
                salience: None,
                organs: Vec::new(),
            };
        };
        match ciborium::from_reader::<OwnerMomentState, _>(encoded.as_slice()) {
            // Workspace1 answering with no focus is knowledge, not absence: nothing currently
            // holds attention, and that is a fact about the system.
            Ok(state) => AttentionProjection {
                knowledge: KnowledgeState::Known,
                focus: state.focus.map(|id| id.to_string()),
                salience: Some(state.salience),
                organs: state.organs,
            },
            Err(_) => AttentionProjection {
                knowledge: KnowledgeState::Unknown,
                focus: None,
                salience: None,
                organs: Vec::new(),
            },
        }
    }

    pub(super) async fn beliefs(&self) -> BeliefsProjection {
        let Some(encoded) = self.read::<Vec<u8>>(EPISTEMIC, "Beliefs").await else {
            return BeliefsProjection {
                knowledge: KnowledgeState::Unknown,
                beliefs: Vec::new(),
            };
        };
        if encoded.is_empty() {
            return BeliefsProjection {
                knowledge: KnowledgeState::Known,
                beliefs: Vec::new(),
            };
        }
        match ciborium::from_reader::<Vec<OwnerBelief>, _>(encoded.as_slice()) {
            Ok(beliefs) => BeliefsProjection {
                knowledge: KnowledgeState::Known,
                beliefs: beliefs
                    .into_iter()
                    // A belief above what this reader is permitted is left out rather than
                    // blanked: an entry saying a subject exists but its value is withheld still
                    // tells a stranger the person said something about it.
                    // The subject is named to the record and the value is not. A person asking
                    // "why did it not tell me that?" needs the subject; anyone else must not learn
                    // the value from a record that outlives the erasure of the value.
                    .filter(|belief| {
                        self.decide(
                            belief.sensitivity,
                            || Some(belief.subject.clone()),
                            &belief.evidence,
                        )
                    })
                    .map(|belief| BeliefProjection {
                        subject: belief.subject,
                        value: belief.value,
                        confidence: belief.confidence,
                        status: belief.status,
                        last_corroborated_at: belief.last_corroborated_at,
                    })
                    .collect(),
            },
            Err(_) => BeliefsProjection {
                knowledge: KnowledgeState::Unknown,
                beliefs: Vec::new(),
            },
        }
    }

    pub(super) async fn perception(&self) -> PerceptionProjection {
        let unread = || PerceptionProjection {
            knowledge: KnowledgeState::Unknown,
            status: None,
            acquired_at: None,
            source_id: None,
        };
        let Some(encoded) = self.read::<Vec<u8>>(PERCEPTION, "State").await else {
            return unread();
        };
        match ciborium::from_reader::<OwnerPerceptionState, _>(encoded.as_slice()) {
            Ok(state) => PerceptionProjection {
                knowledge: KnowledgeState::Known,
                status: Some(state.status),
                acquired_at: Some(state.acquired_at),
                source_id: Some(state.source_id),
            },
            Err(_) => unread(),
        }
    }

    pub(super) async fn context(&self) -> ContextProjection {
        let Some(encoded) = self.read::<Vec<u8>>(CONTEXT, "ActiveContext").await else {
            return ContextProjection {
                knowledge: KnowledgeState::Unknown,
                concepts: Vec::new(),
            };
        };
        match ciborium::from_reader::<Vec<OwnerConcept>, _>(encoded.as_slice()) {
            Ok(concepts) => ContextProjection {
                knowledge: KnowledgeState::Known,
                concepts: concepts
                    .into_iter()
                    // A concept does not carry what it was derived from, so it is counted as
                    // supplied and cannot be accounted for. The record says both.
                    .filter(|concept| {
                        self.decide(concept.sensitivity, || Some(concept.label.clone()), &[])
                    })
                    .map(|concept| ConceptProjection {
                        label: concept.label,
                        salience: concept.salience,
                        activation_reason: concept.activation_reason,
                        last_activated_at: concept.last_activated_at,
                    })
                    .collect(),
            },
            Err(_) => ContextProjection {
                knowledge: KnowledgeState::Unknown,
                concepts: Vec::new(),
            },
        }
    }

    pub(super) async fn lifecycle(&self) -> LifecycleProjection {
        let Some(encoded) = self.read::<Vec<u8>>(LIFECYCLE, "State").await else {
            return LifecycleProjection {
                knowledge: KnowledgeState::Unknown,
                mode: None,
                last_user_activity_at: None,
            };
        };
        match ciborium::from_reader::<OwnerLifecycle, _>(encoded.as_slice()) {
            Ok(state) => LifecycleProjection {
                knowledge: KnowledgeState::Known,
                mode: Some(state.mode),
                last_user_activity_at: Some(state.last_user_activity_at),
            },
            Err(_) => LifecycleProjection {
                knowledge: KnowledgeState::Unknown,
                mode: None,
                last_user_activity_at: None,
            },
        }
    }

    /// All lifecycle records associated with one finding cause.
    pub(super) async fn actions_for_cause(
        &self,
        cause_id: uuid::Uuid,
    ) -> Option<Vec<cybou_web_contracts::ActionRecordProjection>> {
        let encoded = self
            .read_with::<Vec<u8>, (String,)>(
                ACTION,
                "RecordsForCause",
                &(cause_id.to_string(),),
            )
            .await?;
        let records: Vec<cybou_protocol::action::ActionRecord> = decode(&encoded)
            .or_else(|_| ciborium::from_reader(encoded.as_slice()))
            .ok()?;
        Some(records.iter().map(project_action_record).collect())
    }

    /// Recent lifecycle records held by Action1.
    pub(super) async fn recent_actions(&self) -> Option<Vec<cybou_web_contracts::ActionRecordProjection>> {
        let encoded = self
            .read::<Vec<u8>>(ACTION, "RecentRecords")
            .await?;
        let records: Vec<cybou_protocol::action::ActionRecord> = decode(&encoded)
            .or_else(|_| ciborium::from_reader(encoded.as_slice()))
            .ok()?;
        Some(records.iter().map(project_action_record).collect())
    }
}

/// Project one ActionRecord into its web contract representation.
#[must_use]
pub fn project_action_record(
    record: &cybou_protocol::action::ActionRecord,
) -> cybou_web_contracts::ActionRecordProjection {
    use cybou_protocol::action::{
        Agreement, AttemptReport, AuthorizationVerdict, RiskLevel,
    };
    use time::format_description::well_known::Rfc3339;

    cybou_web_contracts::ActionRecordProjection {
        proposal_id: record.proposal.proposal_id,
        cause_id: record.proposal.cause_id,
        proposer: record.proposal.proposed_by.describe(),
        intent: record.proposal.intent.clone(),
        operation: record.proposal.operation.clone(),
        target_resource: record.proposal.target_resource.clone(),
        risk_level: match record.proposal.risk_level {
            RiskLevel::Low => "low".to_owned(),
            RiskLevel::Medium => "medium".to_owned(),
            RiskLevel::High => "high".to_owned(),
            RiskLevel::Critical => "critical".to_owned(),
        },
        reversible: record.proposal.reversible,
        proposed_at: record
            .proposal
            .proposed_at
            .format(&Rfc3339)
            .unwrap_or_default(),
        checks: record
            .checks
            .iter()
            .map(|c| cybou_web_contracts::CriticismCheckProjection {
                rule_id: c.rule_id.clone(),
                description: c.description.clone(),
                passed: c.passed,
                objection: c.objection.clone(),
            })
            .collect(),
        verdict: match &record.decision.verdict {
            AuthorizationVerdict::Granted => "granted".to_owned(),
            AuthorizationVerdict::RequiresUserConfirmation { .. } => {
                "requires-confirmation".to_owned()
            }
            AuthorizationVerdict::Denied { .. } => "denied".to_owned(),
        },
        verdict_reason: match &record.decision.verdict {
            AuthorizationVerdict::Granted => None,
            AuthorizationVerdict::RequiresUserConfirmation { prompt } => Some(prompt.clone()),
            AuthorizationVerdict::Denied { reason } => Some(reason.clone()),
        },
        execution_started: record
            .execution_started
            .as_ref()
            .map(|s| cybou_web_contracts::ExecutionStartedProjection {
                attempt_id: s.attempt_id,
                proposal_id: s.proposal_id,
                operation: s.operation.clone(),
                target_resource: s.target_resource.clone(),
                started_at: s.started_at.format(&Rfc3339).unwrap_or_default(),
            }),
        attempt: record
            .attempt
            .as_ref()
            .map(|a| cybou_web_contracts::ExecutionAttemptProjection {
                attempt_id: a.attempt_id,
                proposal_id: a.proposal_id,
                operation: a.operation.clone(),
                target_resource: a.target_resource.clone(),
                report: a.report.name().to_owned(),
                reason: match &a.report {
                    AttemptReport::Failed { because } | AttemptReport::Refused { because } => {
                        Some(because.clone())
                    }
                    _ => None,
                },
                ended_at: a.ended_at.and_then(|t| t.format(&Rfc3339).ok()),
            }),
        outcome: record
            .outcome
            .as_ref()
            .map(|o| cybou_web_contracts::ActionOutcomeProjection {
                outcome_id: o.outcome_id,
                proposal_id: o.proposal_id,
                relief: o.observed.name().to_owned(),
                agreement: match &o.agreement {
                    Agreement::Agree => "agree".to_owned(),
                    Agreement::Disagree { .. } => "disagree".to_owned(),
                    Agreement::NotComparable => "not-comparable".to_owned(),
                },
                disagreement: match &o.agreement {
                    Agreement::Disagree { about } => Some(about.clone()),
                    _ => None,
                },
                observation_before: None,
                observation_after: None,
                concluded_at: o.concluded_at.format(&Rfc3339).unwrap_or_default(),
            }),
    }
}
