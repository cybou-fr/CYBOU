// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Linux session-bus adapter for the existing Qt `Presence1` service.

use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use ciborium::Value;
use cybou_fabric::{
    EVENT, IDENTITY, INTENTION, LIFECYCLE, PRESENCE, decode,
    rpc::{OperationSemantics, RetryPolicy, RpcOutcome},
    zbus_rpc::ResilientZbusClient,
};
use cybou_protocol::{CapabilityState, KnowledgeState};
use cybou_web_contracts::{
    CapabilityProjection, CommitmentProjection, CommitmentsProjection, Freshness,
    IdentityProjection, JournalProjection, LifecycleProjection, MindProjection, SnapshotProjection,
    WEB_SCHEMA_V1,
};
use futures_util::StreamExt;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::sync::Mutex;
use zbus::{Connection, Proxy, proxy::SignalStream};

use crate::{GatewayError, PresenceSource};

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
}

impl ZbusPresenceSource {
    /// Connect to the user's existing D-Bus session.
    ///
    /// # Errors
    ///
    /// Returns a zbus error when no usable session bus can be established.
    pub async fn connect() -> zbus::Result<Self> {
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
        })
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
        self.connection
            .call_method(
                Some(endpoint.service),
                endpoint.object_path,
                Some(endpoint.interface),
                method,
                &(),
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
            };
        };
        JournalProjection {
            knowledge: KnowledgeState::Known,
            contribution_count: Some(count),
            erasure_epoch: self.read(EVENT, "ErasureEpoch").await,
        }
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
        CommitmentsProjection {
            knowledge: KnowledgeState::Known,
            open_count: self.read(INTENTION, "OpenCount").await,
            open: open
                .into_iter()
                .map(|item| CommitmentProjection {
                    id: item.id,
                    description: item.description,
                    trigger: item.trigger,
                    formed: item.formed,
                })
                .collect(),
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

/// Intention1's own row shape, decoded only to be re-projected into the web contract.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnerIntention {
    id: String,
    description: String,
    trigger: String,
    formed: String,
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
        Ok(MindProjection {
            schema_version: WEB_SCHEMA_V1,
            observed_at: OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into()),
            identity: self.identity().await,
            journal: self.journal().await,
            commitments: self.commitments().await,
            lifecycle: self.lifecycle().await,
        })
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

    use super::ZbusPresenceSource;

    fn encoded(value: &Value) -> Vec<u8> {
        let root = json!({ "version": 1, "value": value });
        let mut bytes = Vec::new();
        ciborium::into_writer(&root, &mut bytes).expect("encode fixture envelope");
        bytes
    }

    #[test]
    fn rust_presenced_projection_is_passed_through_unchanged() {
        // Byte-for-byte what the deployed Rust presenced answers Snapshot with: the web v1
        // projection itself, with no fabric envelope and no Qt-era capabilityStates map.
        let projection = SnapshotProjection {
            schema_version: WEB_SCHEMA_V1,
            projection_version: 98,
            cursor: "0".into(),
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
        assert_eq!(decoded.cursor, "0");
        assert_eq!(decoded.capabilities.len(), 1);
        assert_eq!(decoded.capabilities[0].id, "identity-continuity");
        assert_eq!(decoded.capabilities[0].state, CapabilityState::Available);
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
