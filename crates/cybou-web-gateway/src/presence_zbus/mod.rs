// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Linux session-bus adapter for the existing Qt `Presence1` service.
//!
//! This module holds the connection, the disclosure bookkeeping every read passes through, and the
//! `PresenceSource` surface the gateway sees. The per-organ readers and the owners' wire shapes sit
//! beside it: what an organ answers changes for reasons that have nothing to do with how this
//! adapter connects.

use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use ciborium::Value;
use cybou_fabric::{
    PRESENCE, decode,
    rpc::{OperationSemantics, RetryPolicy, RpcOutcome},
    zbus_rpc::ResilientZbusClient,
};
use cybou_protocol::{CapabilityState, KnowledgeState, disclosure::WithheldBecause};
use cybou_web_contracts::{
    CapabilityProjection, Freshness, MindProjection, SnapshotProjection, WEB_SCHEMA_V1,
};
use futures_util::StreamExt;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::sync::Mutex;
use uuid::Uuid;
use zbus::{Connection, Proxy, proxy::SignalStream};

use crate::{Delivered, GatewayError, PresenceSource, redact::Ledger};

mod organs;
mod wire;

use wire::{field, text};

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
    /// What the last projection this source built passed on, left out, and why.
    ///
    /// ADR-0030 B6: an item quietly dropped for policy reasons and an item that was never relevant
    /// look identical unless something insists on the difference. The filter is the only place that
    /// knows, so it is the place that records it — and the rule it applies lives in
    /// [`crate::redact`], where a test can run it without a session bus.
    ledger: Ledger,
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
            ledger: Ledger::new(),
        })
    }

    /// Decide one item for this source's reader, and record what was decided.
    ///
    /// The only place `permitted_sensitivity` is read. A per-organ filter that compared for itself
    /// could be written slightly differently in one organ than another, and the difference would be
    /// a policy nobody wrote down.
    fn decide<F>(&self, sensitivity: u8, subject: F, evidence: &[Uuid]) -> bool
    where
        F: FnOnce() -> Option<String>,
    {
        self.ledger
            .decide(sensitivity, self.permitted_sensitivity, subject, evidence)
    }

    /// Note that something was held back, and why.
    fn note_withheld(&self, subject: Option<String>, because: WithheldBecause) {
        self.ledger.withhold(subject, because);
    }

    /// Start a fresh delivery, discarding what the last one recorded.
    fn begin_delivery(&self) {
        self.ledger.begin();
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
        self.ledger.delivered()
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

    use super::ZbusPresenceSource;
    use super::wire::{kind_name, millis_to_rfc3339};

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

    fn decoded(encoded: &[u8]) -> Vec<super::wire::OwnerBelief> {
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
        let decoded: Vec<super::wire::OwnerIntention> =
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
        let decoded: super::wire::OwnerMomentState =
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
