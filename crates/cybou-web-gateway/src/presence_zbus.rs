// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Linux session-bus adapter for the existing Qt `Presence1` service.

use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use ciborium::Value;
use cybou_fabric::{
    CONTEXT, EPISTEMIC, EVENT, IDENTITY, INTENTION, LIFECYCLE, PERCEPTION, PRESENCE, SELF,
    WORKSPACE, decode,
    rpc::{OperationSemantics, RetryPolicy, RpcOutcome},
    zbus_rpc::ResilientZbusClient,
};
use cybou_protocol::{
    CapabilityState, KnowledgeState,
    canonical::CanonicalEnvelope,
    disclosure::{Withheld, WithheldBecause},
};
use cybou_web_contracts::{
    AttentionProjection, BeliefProjection, BeliefsProjection, CapabilityProjection,
    CommitmentProjection, CommitmentsProjection, ConceptProjection, ContextProjection,
    ContributionProjection, Freshness, IdentityProjection, JournalProjection, LifecycleProjection,
    MindProjection, PerceptionProjection, SelfProjection, SnapshotProjection, WEB_SCHEMA_V1,
};
use futures_util::StreamExt;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::sync::Mutex;
use uuid::Uuid;
use zbus::{Connection, Proxy, proxy::SignalStream};

use crate::{Delivered, GatewayError, PresenceSource};

fn field<'a>(map: &'a [(Value, Value)], name: &str) -> Option<&'a Value> {
    map.iter()
        .find_map(|(key, value)| (key.as_text() == Some(name)).then_some(value))
}

fn text(value: &Value) -> Option<&str> {
    match value {
        Value::Text(value) => Some(value),
        Value::Tag(_, inner) => text(inner),
        _ => None,
    }
}

/// Read-only adapter that maps the existing Qt CBOR projection into the web v1 contract.
pub struct ZbusPresenceSource {
    rpc: ResilientZbusClient,
    /// Kept alongside the Presence1 client so the other owners can be read over the same bus.
    connection: Connection,
    changed: Mutex<SignalStream<'static>>,
    projection_version: AtomicU64,
    /// The most exposing class this source may pass on.
    ///
    /// Filtering here rather than at the route is deliberate: every consumer of this source gets
    /// the same answer, so a new route cannot forget to filter and publish what the last one did
    /// not.
    permitted_sensitivity: u8,
    /// What the last projection this source built left out, and why.
    ///
    /// ADR-0030 B6: an item quietly dropped for policy reasons and an item that was never relevant
    /// look identical unless something insists on the difference. The filter is the only place that
    /// knows, so it is the place that records it.
    withheld: Mutex<Vec<Withheld>>,
    /// The contributions the last projection's items were derived from.
    provenance: Mutex<Vec<Uuid>>,
    /// How many items the last projection carried, whether or not their provenance was known.
    supplied: AtomicU64,
}

impl ZbusPresenceSource {
    /// Connect to the user's existing D-Bus session.
    ///
    /// # Errors
    ///
    /// Returns a zbus error when no usable session bus can be established.
    /// Connect a source that may pass on anything, for a reader who is entitled to it.
    ///
    /// # Errors
    ///
    /// Returns the zbus error when the session bus or the Presence1 subscription is unavailable.
    pub async fn connect() -> zbus::Result<Self> {
        Self::connect_permitting(u8::MAX).await
    }

    /// Connect a source that passes on nothing above `permitted_sensitivity`.
    ///
    /// # Errors
    ///
    /// Returns the zbus error when the session bus or the Presence1 subscription is unavailable.
    pub async fn connect_permitting(permitted_sensitivity: u8) -> zbus::Result<Self> {
        let connection = Connection::session().await?;
        let proxy = Proxy::new(
            &connection,
            PRESENCE.service,
            PRESENCE.object_path,
            PRESENCE.interface,
        )
        .await?;
        let changed = proxy.receive_signal("Changed").await?;
        Ok(Self {
            rpc: ResilientZbusClient::new(connection.clone(), PRESENCE, RetryPolicy::default()),
            connection,
            changed: Mutex::new(changed),
            projection_version: AtomicU64::new(0),
            permitted_sensitivity,
            withheld: Mutex::new(Vec::new()),
            provenance: Mutex::new(Vec::new()),
            supplied: AtomicU64::new(0),
        })
    }

    /// Note that something was supplied, and what it came from.
    fn note_supplied(&self, evidence: &[Uuid]) {
        self.supplied.fetch_add(1, Ordering::Relaxed);
        if evidence.is_empty() {
            return;
        }
        if let Ok(mut provenance) = self.provenance.try_lock() {
            for id in evidence {
                if !provenance.contains(id) {
                    provenance.push(*id);
                }
            }
        }
    }

    /// Note that something was held back, and why.
    fn note_withheld(&self, subject: Option<String>, because: WithheldBecause) {
        if let Ok(mut withheld) = self.withheld.try_lock() {
            withheld.push(Withheld { subject, because });
        }
    }

    /// Start a fresh delivery, discarding what the last one recorded.
    fn begin_delivery(&self) {
        self.supplied.store(0, Ordering::Relaxed);
        if let Ok(mut withheld) = self.withheld.try_lock() {
            withheld.clear();
        }
        if let Ok(mut provenance) = self.provenance.try_lock() {
            provenance.clear();
        }
    }

    async fn encoded_snapshot(&self) -> Result<Vec<u8>, GatewayError> {
        let result = self
            .rpc
            .call(
                "Snapshot",
                &(),
                OperationSemantics::ReadOnly,
                900,
                0x50_52_45_53,
            )
            .await;
        match (result.outcome, result.reply) {
            (RpcOutcome::Succeeded, Some(reply)) => reply
                .body()
                .deserialize()
                .map_err(|_| GatewayError::InvalidProjection),
            (RpcOutcome::TimedOut, _) => Err(GatewayError::Timeout),
            _ => Err(GatewayError::Unavailable),
        }
    }

    fn decode_snapshot(
        encoded: &[u8],
        projection_version: u64,
    ) -> Result<SnapshotProjection, GatewayError> {
        // The Rust `presenced` owns Presence1 in production and already speaks the web v1
        // contract, unwrapped: it writes a `SnapshotProjection` straight onto the wire. Its
        // projection carries the owner's own version and cursor, so it is passed through rather
        // than renumbered by a counter that only the gateway can see.
        if let Ok(projection) = ciborium::from_reader::<SnapshotProjection, _>(encoded) {
            return Ok(projection);
        }

        // The frozen Qt Presence1 wraps a differently shaped payload in a fabric envelope. It is
        // no longer deployed, but it remains the compatibility reference, so it still decodes.
        let value: Value = decode(encoded).map_err(|_| GatewayError::InvalidProjection)?;
        let value = value.as_map().ok_or(GatewayError::InvalidProjection)?;
        if field(value, "runtimeReachable").and_then(Value::as_bool) != Some(true) {
            return Err(GatewayError::Unavailable);
        }

        let states = field(value, "capabilityStates")
            .and_then(Value::as_map)
            .ok_or(GatewayError::InvalidProjection)?;
        let capabilities = states
            .iter()
            .map(|(id, raw_state)| {
                let id = text(id).ok_or(GatewayError::InvalidProjection)?;
                let state = match text(raw_state) {
                    Some("available") => CapabilityState::Available,
                    Some("unknown") | None => CapabilityState::Unknown,
                    Some(_) => CapabilityState::Unavailable,
                };
                Ok(CapabilityProjection {
                    id: id.to_owned(),
                    state,
                    knowledge: if state == CapabilityState::Unknown {
                        KnowledgeState::Unknown
                    } else {
                        KnowledgeState::Known
                    },
                    freshness: Freshness::Current,
                    reason: None,
                })
            })
            .collect::<Result<Vec<_>, GatewayError>>()?;

        let observed_at = field(value, "capabilityObservedAt")
            .and_then(text)
            .filter(|value| !value.is_empty())
            .map_or_else(
                || {
                    OffsetDateTime::now_utc()
                        .format(&Rfc3339)
                        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
                },
                ToOwned::to_owned,
            );

        Ok(SnapshotProjection {
            schema_version: WEB_SCHEMA_V1,
            projection_version,
            cursor: format!("presence:{projection_version}"),
            observed_at,
            freshness: Freshness::Current,
            knowledge: KnowledgeState::Known,
            capabilities,
        })
    }
}

impl ZbusPresenceSource {
    /// Call one owner method and decode its reply, treating any failure as "not answered".
    ///
    /// Every section of the Mind projection is optional for exactly this reason: the owners are
    /// separate processes that fail separately, and one silent organ must not take the rest of
    /// the page with it.
    async fn read<T>(&self, endpoint: cybou_fabric::BusEndpoint, method: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned + zbus::zvariant::Type,
    {
        self.read_with(endpoint, method, &()).await
    }

    async fn read_with<T, A>(
        &self,
        endpoint: cybou_fabric::BusEndpoint,
        method: &str,
        args: &A,
    ) -> Option<T>
    where
        T: serde::de::DeserializeOwned + zbus::zvariant::Type,
        A: serde::Serialize + zbus::zvariant::DynamicType,
    {
        self.connection
            .call_method(
                Some(endpoint.service),
                endpoint.object_path,
                Some(endpoint.interface),
                method,
                args,
            )
            .await
            .ok()?
            .body()
            .deserialize()
            .ok()
    }

    async fn identity(&self) -> IdentityProjection {
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

    async fn journal(&self) -> JournalProjection {
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
    async fn integrity(&self) -> Option<String> {
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
    async fn recent_contributions(&self) -> Vec<ContributionProjection> {
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

    async fn commitments(&self) -> CommitmentsProjection {
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
    async fn self_model(&self) -> SelfProjection {
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

    async fn attention(&self) -> AttentionProjection {
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

    async fn beliefs(&self) -> BeliefsProjection {
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
                    .filter(|belief| {
                        let permitted = belief.sensitivity <= self.permitted_sensitivity;
                        if permitted {
                            self.note_supplied(&belief.evidence);
                        } else {
                            // The subject is named and the value is not. A person asking "why did
                            // it not tell me that?" needs the subject; anyone else must not learn
                            // the value from a record that outlives the erasure of the value.
                            self.note_withheld(
                                Some(belief.subject.clone()),
                                WithheldBecause::AboveConsumerTrust,
                            );
                        }
                        permitted
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

    async fn perception(&self) -> PerceptionProjection {
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

    async fn context(&self) -> ContextProjection {
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
                    .filter(|concept| {
                        let permitted = concept.sensitivity <= self.permitted_sensitivity;
                        if permitted {
                            // A concept does not carry what it was derived from, so it is counted
                            // as supplied and cannot be accounted for. The record says both.
                            self.note_supplied(&[]);
                        } else {
                            self.note_withheld(
                                Some(concept.label.clone()),
                                WithheldBecause::AboveConsumerTrust,
                            );
                        }
                        permitted
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

    async fn lifecycle(&self) -> LifecycleProjection {
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
}

/// How many Journal rows the panel asks for. Enough to show that a system is living, few enough
/// that one read stays inside the gateway's budget.
const RECENT_CONTRIBUTIONS: i32 = 12;

/// The frozen kind in its own spelling, or an explicit unknown.
///
/// A kind this contract version cannot name is reported as unknown rather than guessed at, for the
/// same reason `Kind::from_u16` refuses to default it.
fn kind_name(kind: u16) -> String {
    cybou_protocol::Kind::from_u16(kind).map_or_else(
        || format!("unknown kind {kind}"),
        |kind| format!("{kind:?}").to_lowercase(),
    )
}

fn millis_to_rfc3339(millis: i64) -> String {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(millis) * 1_000_000)
        .ok()
        .and_then(|instant| instant.format(&Rfc3339).ok())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".into())
}

/// Intention1's own row shape, decoded only to be re-projected into the web contract.
///
/// The identity is a `Uuid`, not a string: serde encodes a UUID as raw bytes in CBOR and as text
/// only in human-readable formats, so decoding it as `String` fails against the owner's real
/// bytes while looking correct in a JSON fixture.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnerIntention {
    id: uuid::Uuid,
    description: String,
    trigger: String,
    formed: String,
}

/// The fields of Self1's report the panel uses; the rest of the report stays with its owner.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnerSelfReport {
    age_in_days: i64,
    sessions: u64,
    open_intentions: u32,
    settled_predictions: u32,
}

/// Workspace1's momentary state.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnerMomentState {
    focus: Option<uuid::Uuid>,
    salience: f64,
    organs: Vec<String>,
}

/// Epistemic1's belief row.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnerBelief {
    subject: String,
    value: String,
    confidence: f64,
    status: String,
    last_corroborated_at: String,
    /// The contributions this belief was formed from.
    ///
    /// Absent in projections written before beliefs carried it, which is treated as provenance that
    /// cannot be accounted for rather than as a belief that came from nothing.
    #[serde(default)]
    evidence: Vec<Uuid>,
    /// What the belief was derived from, on the frozen sensitivity scale.
    ///
    /// Absent in projections written before beliefs carried it. Absent is not ordinary: a belief
    /// this gateway cannot classify is one it must not decide is safe to publish, so the default
    /// is the most exposing value rather than the least.
    #[serde(default = "unclassified")]
    sensitivity: u8,
}

/// The frozen sensitivity class of something that is about the person.
///
/// `Personal` on the scale in `cybou_protocol::admission`. Named here because two projections are
/// filtered against it without carrying a class of their own: what they hold is about the person by
/// construction rather than by classification.
const PERSONAL: u8 = 1;

/// What an owner projection written before it carried a class is treated as.
///
/// The top of the frozen scale. An older projection is not evidence that its contents are
/// ordinary, and defaulting to zero would publish exactly the rows nobody had classified yet.
const fn unclassified() -> u8 {
    u8::MAX
}

/// Perception1's last acquisition.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnerPerceptionState {
    status: String,
    acquired_at: String,
    source_id: String,
}

/// Context1's concept node.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnerConcept {
    label: String,
    salience: f64,
    activation_reason: String,
    last_activated_at: String,
    /// What activated the concept, on the frozen sensitivity scale.
    #[serde(default = "unclassified")]
    sensitivity: u8,
}

/// Event1's verification state.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnerVerification {
    verified_through: u64,
    head: u64,
    broken_at: Option<u64>,
}

/// Lifecycle1's own state shape, of which the panel uses two fields.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnerLifecycle {
    mode: String,
    last_user_activity_at: String,
}

#[async_trait]
impl PresenceSource for ZbusPresenceSource {
    async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError> {
        let encoded = self.encoded_snapshot().await?;
        let projection_version = self.projection_version.fetch_add(1, Ordering::Relaxed) + 1;
        Self::decode_snapshot(&encoded, projection_version)
    }

    async fn mind(&self) -> Result<MindProjection, GatewayError> {
        // One projection is one delivery, so what the last one withheld is cleared before this one
        // starts. Accumulating across requests would report an item as held back long after the
        // request it was held back from.
        self.begin_delivery();
        Ok(MindProjection {
            schema_version: WEB_SCHEMA_V1,
            observed_at: OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into()),
            identity: self.identity().await,
            journal: self.journal().await,
            commitments: self.commitments().await,
            lifecycle: self.lifecycle().await,
            self_model: self.self_model().await,
            attention: self.attention().await,
            beliefs: self.beliefs().await,
            perception: self.perception().await,
            context: self.context().await,
        })
    }

    fn last_delivery(&self) -> Delivered {
        Delivered {
            items: self
                .provenance
                .try_lock()
                .map(|held| held.clone())
                .unwrap_or_default(),
            item_count: u32::try_from(self.supplied.load(Ordering::Relaxed)).unwrap_or(u32::MAX),
            withheld: self
                .withheld
                .try_lock()
                .map(|held| held.clone())
                .unwrap_or_default(),
        }
    }

    async fn wait_for_change(&self) -> Result<(), GatewayError> {
        self.changed
            .lock()
            .await
            .next()
            .await
            .map(|_| ())
            .ok_or(GatewayError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use cybou_protocol::{CapabilityState, KnowledgeState};
    use cybou_web_contracts::{CapabilityProjection, Freshness, SnapshotProjection, WEB_SCHEMA_V1};
    use serde_json::{Value, json};

    use uuid::Uuid;

    use super::{ZbusPresenceSource, kind_name, millis_to_rfc3339};

    fn encoded(value: &Value) -> Vec<u8> {
        let root = json!({ "version": 1, "value": value });
        let mut bytes = Vec::new();
        ciborium::into_writer(&root, &mut bytes).expect("encode fixture envelope");
        bytes
    }

    fn beliefs_from(rows: &[(&str, u8)]) -> Vec<u8> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Row {
            subject: String,
            value: String,
            confidence: f64,
            status: String,
            last_corroborated_at: String,
            sensitivity: u8,
        }
        let rows: Vec<Row> = rows
            .iter()
            .map(|(subject, sensitivity)| Row {
                subject: (*subject).to_owned(),
                value: "something".into(),
                confidence: 1.0,
                status: "observed".into(),
                last_corroborated_at: "2026-08-20T00:00:00Z".into(),
                sensitivity: *sensitivity,
            })
            .collect();
        let mut encoded = Vec::new();
        ciborium::into_writer(&rows, &mut encoded).expect("encode owner beliefs");
        encoded
    }

    fn decoded(encoded: &[u8]) -> Vec<super::OwnerBelief> {
        ciborium::from_reader(encoded).expect("owner beliefs decode")
    }

    #[test]
    fn a_belief_above_the_line_is_left_out_rather_than_blanked() {
        // The filter runs where the owner rows are decoded, so this is the shape it acts on: an
        // entry saying a subject exists but its value is withheld would still tell a stranger the
        // person said something about it.
        let rows = decoded(&beliefs_from(&[("kernel-version", 0), ("utterance", 1)]));
        let permitted = 0;
        let published: Vec<_> = rows
            .into_iter()
            .filter(|belief| belief.sensitivity <= permitted)
            .map(|belief| belief.subject)
            .collect();
        assert_eq!(published, vec!["kernel-version"]);
    }

    #[test]
    fn an_owner_row_with_no_class_is_treated_as_the_most_exposing_one() {
        // An older owner writing rows without a class is not evidence that they are ordinary.
        // Defaulting to zero would publish exactly the rows nobody had classified yet.
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Unclassified {
            subject: String,
            value: String,
            confidence: f64,
            status: String,
            last_corroborated_at: String,
        }
        let mut encoded = Vec::new();
        ciborium::into_writer(
            &vec![Unclassified {
                subject: "from-an-older-owner".into(),
                value: "something".into(),
                confidence: 1.0,
                status: "observed".into(),
                last_corroborated_at: "2026-08-20T00:00:00Z".into(),
            }],
            &mut encoded,
        )
        .expect("encode owner beliefs");

        let rows = decoded(&encoded);
        assert_eq!(rows[0].sensitivity, u8::MAX);
        assert!(
            rows.iter().all(|belief| belief.sensitivity > 0),
            "an unclassified row must not pass a filter permitting only ordinary"
        );
    }

    #[test]
    fn rust_presenced_projection_is_passed_through_unchanged() {
        // Byte-for-byte what the deployed Rust presenced answers Snapshot with: the web v1
        // projection itself, with no fabric envelope and no Qt-era capabilityStates map.
        let projection = SnapshotProjection {
            schema_version: WEB_SCHEMA_V1,
            projection_version: 98,
            cursor: "presence:98".into(),
            observed_at: "2026-08-19T17:56:40.069132466Z".into(),
            freshness: Freshness::Current,
            knowledge: KnowledgeState::Known,
            capabilities: vec![CapabilityProjection {
                id: "identity-continuity".into(),
                state: CapabilityState::Available,
                knowledge: KnowledgeState::Known,
                freshness: Freshness::Current,
                reason: None,
            }],
        };
        let mut bytes = Vec::new();
        ciborium::into_writer(&projection, &mut bytes).expect("encode owner projection");

        let decoded = ZbusPresenceSource::decode_snapshot(&bytes, 1).expect("typed projection");
        assert_eq!(decoded.projection_version, 98);
        assert_eq!(decoded.cursor, "presence:98");
        assert_eq!(decoded.capabilities.len(), 1);
        assert_eq!(decoded.capabilities[0].id, "identity-continuity");
        assert_eq!(decoded.capabilities[0].state, CapabilityState::Available);
    }

    #[test]
    fn owner_rows_decode_from_the_bytes_the_owners_actually_write() {
        // Encoded exactly as the owners encode them: ciborium over their own types, where a Uuid
        // is raw bytes rather than the text a JSON fixture would have shown.
        #[derive(serde::Serialize)]
        struct OwnerIntentionWire {
            id: Uuid,
            description: String,
            trigger: String,
            formed: String,
        }
        #[derive(serde::Serialize)]
        struct OwnerMomentStateWire {
            focus: Option<Uuid>,
            salience: f64,
            organs: Vec<String>,
        }

        let id = Uuid::from_u128(0x8f14_e45f_ceea_467a_9c9e_4d3f_2a1b_7c60);
        let mut bytes = Vec::new();
        ciborium::into_writer(
            &vec![OwnerIntentionWire {
                id,
                description: "Run integration tests".into(),
                trigger: "Session startup".into(),
                formed: "2026-08-19T11:40:00Z".into(),
            }],
            &mut bytes,
        )
        .expect("encode owner intentions");
        let decoded: Vec<super::OwnerIntention> =
            ciborium::from_reader(bytes.as_slice()).expect("decode owner intentions");
        assert_eq!(decoded[0].id, id);

        let mut bytes = Vec::new();
        ciborium::into_writer(
            &OwnerMomentStateWire {
                focus: Some(id),
                salience: 0.75,
                organs: vec!["perceptiond".into()],
            },
            &mut bytes,
        )
        .expect("encode owner moment state");
        let decoded: super::OwnerMomentState =
            ciborium::from_reader(bytes.as_slice()).expect("decode owner moment state");
        assert_eq!(decoded.focus, Some(id));
    }

    #[test]
    fn an_unnameable_kind_is_reported_as_unknown_rather_than_guessed() {
        assert_eq!(kind_name(1), "observation");
        assert_eq!(kind_name(11), "intention");
        assert_eq!(kind_name(17), "contextdisclosed");
        assert_eq!(kind_name(999), "unknown kind 999");
    }

    #[test]
    fn journal_instants_are_rendered_from_the_stored_milliseconds() {
        assert_eq!(millis_to_rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(millis_to_rfc3339(1_787_175_856_000), "2026-08-19T21:44:16Z");
    }

    #[test]
    fn qt_shaped_cbor_envelope_maps_to_typed_web_projection() {
        let bytes = encoded(&json!({
            "runtimeReachable": true,
            "capabilityObservedAt": "2026-08-18T12:00:00Z",
            "capabilityStates": BTreeMap::from([
                ("mind.identity.read", "available"),
                ("mind.lifecycle.command", "unavailable")
            ])
        }));
        let projection = ZbusPresenceSource::decode_snapshot(&bytes, 1).expect("typed projection");
        assert_eq!(projection.projection_version, 1);
        assert_eq!(projection.capabilities[0].state, CapabilityState::Available);
    }
}
