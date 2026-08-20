// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Presence1` service implementation on zbus.

// The `#[interface]` expansion emits part of its dispatch surface with the attribute's own span,
// which an `allow` on the impl block cannot reach. Every handler written here is documented, and
// the generated dispatch reads parameters that the fail-closed stubs below deliberately ignore.
#![allow(clippy::used_underscore_binding, missing_docs)]

use std::sync::Arc;

use cybou_fabric::PRESENCE;
use time::OffsetDateTime;
use zbus::{interface, object_server::SignalEmitter};

use crate::PresenceCore;

/// D-Bus Service exporting `org.cybou.Mind.Presence1`.
pub struct Presence1Service {
    core: Arc<PresenceCore>,
}

impl Presence1Service {
    /// Create a new Presence1 D-Bus service handler around `PresenceCore`.
    #[must_use]
    pub fn new(core: Arc<PresenceCore>) -> Self {
        Self { core }
    }

    /// Close the obligation at a position in the open list.
    ///
    /// Presence1 presents a list, so a person points at a position in it. Resolving that position
    /// to an identity has to happen against the list Intention1 currently holds, or the command
    /// would close whichever obligation had drifted into that slot.
    async fn close_at(&self, conn: &zbus::Connection, index: i32, resolution: &str) -> bool {
        #[cfg(target_os = "linux")]
        {
            use cybou_fabric::INTENTION;

            let Ok(index) = usize::try_from(index) else {
                return false;
            };
            let Some(encoded) = call::<Vec<u8>, _>(conn, INTENTION, "Open", &()).await else {
                return false;
            };
            let Ok(open) = ciborium::from_reader::<Vec<OpenIntention>, _>(encoded.as_slice())
            else {
                return false;
            };
            let Some(target) = open.get(index) else {
                return false;
            };
            // The obligation is closed either way; only the record may be missing. Presence1
            // reports the command as done and leaves the biography's own account to the Journal
            // panel, rather than telling a person their promise did not close when it did.
            let outcome: String = call(
                conn,
                INTENTION,
                "Close",
                &(target.id.to_string().as_str(), resolution, ""),
            )
            .await
            .unwrap_or_default();
            if outcome == "closed-but-unrecorded" {
                println!("[cybou-presenced] An obligation closed without reaching the Journal");
            }
            outcome.starts_with("closed")
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (conn, index, resolution);
            false
        }
    }
}

/// Record what a person observed as a contribution, and return the identity it was given.
///
/// An observation is a root kind: it records something that happened outside the Journal and has
/// no prior contribution to cite, which is exactly what a person reporting a measurement is.
#[cfg(target_os = "linux")]
async fn record_observation(subject: &str, value: ciborium::Value) -> Option<uuid::Uuid> {
    use cybou_fabric::event_client::EventClient;
    use cybou_protocol::{canonical::CanonicalEnvelope, observation::ObservationV1, unix_millis};
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};

    let now = OffsetDateTime::now_utc();
    let instant = now.format(&Rfc3339).ok()?;
    let observation = ObservationV1 {
        source_id: "presence.user".into(),
        subject: subject.to_owned(),
        value,
        acquired_at: instant.clone(),
        // A person's report is about now; it says nothing about how long it stays true.
        freshness_until: instant,
        provenance: "reported through Presence1 by the person using the system".into(),
    };
    let mut payload = Vec::new();
    ciborium::into_writer(&observation, &mut payload).ok()?;

    let message_id = uuid::Uuid::new_v4();
    let envelope = CanonicalEnvelope {
        schema_version: 3,
        message_id,
        correlation_id: uuid::Uuid::new_v4(),
        causation_id: uuid::Uuid::nil(),
        origin_organ: "presenced".to_string(),
        origin_node: String::new(),
        kind: 1, // Observation
        wall_time_ms: unix_millis(now),
        monotonic_time: 0,
        logical_clock: 1,
        confidence: 1.0,
        evidence: vec![],
        payload,
        privacy: 1, // Node
        capability_scope: String::new(),
        sealed: false,
        key_domain_id: uuid::Uuid::nil(),
        key_epoch: 0,
        retention_class: 2,
        retention_policy_version: 0,
        retain_until_ms: 0,
        // Personal: a person reported this, so it is about them and theirs to release.
        sensitivity: 1,
    };

    let client = EventClient::session().await.ok()?;
    client.submit(&envelope).await.ok()?;
    Some(message_id)
}

/// Only the identity is needed to close an obligation someone pointed at.
#[cfg(target_os = "linux")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenIntention {
    id: uuid::Uuid,
}

/// Ask one owner for something, treating any failure as no answer.
///
/// Presence1 presents and asks; it never holds the state it is talking about. Every command here
/// is a call to the owner that does.
#[cfg(target_os = "linux")]
async fn call<T, A>(
    conn: &zbus::Connection,
    endpoint: cybou_fabric::BusEndpoint,
    method: &str,
    args: &A,
) -> Option<T>
where
    T: serde::de::DeserializeOwned + zbus::zvariant::Type,
    A: serde::Serialize + zbus::zvariant::DynamicType,
{
    conn.call_method(
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

#[allow(
    clippy::unused_async,
    reason = "zbus dispatches every exported handler as a future"
)]
#[interface(name = "org.cybou.Mind.Presence1")]
impl Presence1Service {
    /// Service readiness.
    async fn ready(&self) -> bool {
        true
    }

    /// Overall health summary querying Health1 owner when available.
    async fn health(&self, #[zbus(connection)] conn: &zbus::Connection) -> String {
        #[cfg(target_os = "linux")]
        {
            use cybou_fabric::HEALTH;
            if let Ok(reply) = conn
                .call_method(
                    Some(HEALTH.service),
                    HEALTH.object_path,
                    Some(HEALTH.interface),
                    "Health",
                    &(),
                )
                .await
                && let Ok(h) = reply.body().deserialize::<String>()
            {
                return h;
            }
        }
        let _ = conn;
        "unknown".to_string()
    }

    /// Last error diagnostic.
    async fn last_error(&self) -> String {
        String::new()
    }

    /// Compound snapshot projection encoded as CBOR from Health1, or honest unknown default.
    async fn snapshot(&self, #[zbus(connection)] conn: &zbus::Connection) -> Vec<u8> {
        #[cfg(target_os = "linux")]
        {
            use cybou_fabric::HEALTH;
            if let Ok(reply) = conn
                .call_method(
                    Some(HEALTH.service),
                    HEALTH.object_path,
                    Some(HEALTH.interface),
                    "Snapshot",
                    &(),
                )
                .await
                && let Ok(snap_bytes) = reply.body().deserialize::<Vec<u8>>()
                && !snap_bytes.is_empty()
            {
                return snap_bytes;
            }
        }
        let _ = conn;
        let now = OffsetDateTime::now_utc();
        let snap = self.core.build_snapshot(now);
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&snap, &mut buf);
        buf
    }

    /// Recent contributions as CBOR, as Event1 holds them.
    async fn activity(&self, #[zbus(connection)] conn: &zbus::Connection, limit: i32) -> Vec<u8> {
        #[cfg(target_os = "linux")]
        {
            use cybou_fabric::EVENT;
            return call(conn, EVENT, "Recent", &(limit.clamp(1, 128),))
                .await
                .unwrap_or_default();
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (conn, limit);
            Vec::new()
        }
    }

    /// Open obligations as CBOR, as Intention1 holds them.
    async fn detailed_obligations(&self, #[zbus(connection)] conn: &zbus::Connection) -> Vec<u8> {
        #[cfg(target_os = "linux")]
        {
            use cybou_fabric::INTENTION;
            return call(conn, INTENTION, "Open", &()).await.unwrap_or_default();
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = conn;
            Vec::new()
        }
    }

    /// Promise an obligation. Intention1 owns it; this only asks.
    async fn promise(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        description: String,
    ) -> String {
        #[cfg(target_os = "linux")]
        {
            use cybou_fabric::INTENTION;

            // A promise made with no cause cannot enter the Journal — Kind::Intention is derived
            // and must name what it came from — so the main way a person creates a commitment used
            // to leave it outside the biography entirely. What a person asked for is itself
            // something that happened outside the Journal, which is what a root Observation
            // records, and the intention is caused by it.
            let cause =
                record_observation("user-promise", ciborium::Value::Text(description.clone()))
                    .await;
            let cause = cause.map(|id| id.to_string()).unwrap_or_default();

            return call(
                conn,
                INTENTION,
                "Form",
                &(description.as_str(), "user promise", cause.as_str()),
            )
            .await
            .unwrap_or_default();
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (conn, description);
            String::new()
        }
    }

    /// Ask Self1 to assess itself. The assessment is its own; this only asks for one.
    async fn reflect(&self, #[zbus(connection)] conn: &zbus::Connection) -> bool {
        #[cfg(target_os = "linux")]
        {
            use cybou_fabric::SELF;
            return call::<Vec<u8>, _>(conn, SELF, "Measure", &())
                .await
                .is_some_and(|report| !report.is_empty());
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = conn;
            false
        }
    }

    /// Fulfil the obligation at a position in the open list.
    async fn fulfill_index(&self, #[zbus(connection)] conn: &zbus::Connection, index: i32) -> bool {
        self.close_at(conn, index, "fulfilled").await
    }

    /// Abandon the obligation at a position in the open list.
    async fn abandon_index(&self, #[zbus(connection)] conn: &zbus::Connection, index: i32) -> bool {
        self.close_at(conn, index, "abandoned").await
    }

    /// Record a numeric observation for Predictor1 to calibrate against.
    ///
    /// The subject is whatever the person chose to track, which is why nothing in the system
    /// invents one: a forecast is only worth making about something someone cares about.
    async fn observe(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        subject: String,
        value: f64,
    ) -> bool {
        #[cfg(target_os = "linux")]
        {
            use cybou_fabric::PREDICTOR;

            // Predictor1 asks for the contribution that produced the sample. A fresh UUID answers
            // the type and lies about the fact: it names no contribution, so nothing could ever be
            // traced back to what was actually observed. Record the observation first and pass the
            // identity the Journal gave it.
            let Some(contribution_id) =
                record_observation(&subject, ciborium::Value::Float(value)).await
            else {
                return false;
            };
            return call(
                conn,
                PREDICTOR,
                "Observe",
                &(
                    subject.as_str(),
                    value,
                    contribution_id.to_string().as_str(),
                ),
            )
            .await
            .unwrap_or(false);
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (conn, subject, value);
            false
        }
    }

    /// Ask Predictor1 for a forecast as CBOR. Empty when it has no history to forecast from.
    async fn predict(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        subject: String,
    ) -> Vec<u8> {
        #[cfg(target_os = "linux")]
        {
            use cybou_fabric::PREDICTOR;
            return call(conn, PREDICTOR, "Predict", &(subject.as_str(),))
                .await
                .unwrap_or_default();
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (conn, subject);
            Vec::new()
        }
    }

    /// Tell Lifecycle1 a person is present, which is what interrupts consolidation.
    async fn interrupt_lifecycle(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        cause: String,
    ) -> bool {
        #[cfg(target_os = "linux")]
        {
            use cybou_fabric::LIFECYCLE;
            return call(conn, LIFECYCLE, "NotifyUserActivity", &(cause.as_str(),))
                .await
                .unwrap_or(false);
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (conn, cause);
            false
        }
    }

    /// Signal emitted when compound projection changes.
    #[zbus(signal)]
    async fn changed(ctxt: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// Emit `Changed` from outside a method call, over the connection that owns Presence1.
///
/// Presence1 has no clock of its own: it changes when the owners it presents change. The
/// re-emission task is therefore not a D-Bus method and needs an emitter bound to the owning
/// connection.
///
/// # Errors
///
/// Returns the zbus error when the path is invalid or the signal cannot be sent.
pub async fn emit_changed(connection: &zbus::Connection) -> zbus::Result<()> {
    let emitter = SignalEmitter::new(connection, PRESENCE.object_path)?;
    Presence1Service::changed(&emitter).await
}
