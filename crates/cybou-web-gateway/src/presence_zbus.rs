// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Linux session-bus adapter for the existing Qt `Presence1` service.

use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use ciborium::Value;
use cybou_fabric::{
    PRESENCE, decode,
    rpc::{OperationSemantics, RetryPolicy, RpcOutcome},
    zbus_rpc::ResilientZbusClient,
};
use cybou_protocol::{CapabilityState, KnowledgeState};
use cybou_web_contracts::{CapabilityProjection, Freshness, SnapshotProjection, WEB_SCHEMA_V1};
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
            rpc: ResilientZbusClient::new(connection, PRESENCE, RetryPolicy::default()),
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
            capabilities,
        })
    }
}

#[async_trait]
impl PresenceSource for ZbusPresenceSource {
    async fn snapshot(&self) -> Result<SnapshotProjection, GatewayError> {
        let encoded = self.encoded_snapshot().await?;
        let projection_version = self.projection_version.fetch_add(1, Ordering::Relaxed) + 1;
        Self::decode_snapshot(&encoded, projection_version)
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

    use cybou_protocol::CapabilityState;
    use serde_json::{Value, json};

    use super::ZbusPresenceSource;

    fn encoded(value: &Value) -> Vec<u8> {
        let root = json!({ "version": 1, "value": value });
        let mut bytes = Vec::new();
        ciborium::into_writer(&root, &mut bytes).expect("encode fixture envelope");
        bytes
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
