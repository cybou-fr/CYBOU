// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Action1` service.

#![allow(missing_docs)]

use std::sync::Arc;

use cybou_fabric::{decode, encode, event_client::EventClient};
use cybou_protocol::telemetry::SystemInsight;
use time::OffsetDateTime;
use uuid::Uuid;
use zbus::{fdo, interface};

use crate::{ActionCore, journal};

/// Process-owned Action1 dispatch surface.
pub struct Action1Service {
    core: Arc<ActionCore>,
}

impl Action1Service {
    /// Wrap the lifecycle owner.
    #[must_use]
    pub fn new(core: Arc<ActionCore>) -> Self {
        Self { core }
    }
}

#[allow(clippy::unused_async, reason = "zbus handlers are futures")]
#[interface(name = "org.cybou.Mind.Action1")]
impl Action1Service {
    async fn ready(&self) -> bool {
        true
    }

    async fn evaluate_insight(
        &self,
        insight: Vec<u8>,
        operation: String,
    ) -> fdo::Result<(Vec<u8>, String)> {
        let insight: SystemInsight =
            decode(&insight).map_err(|error| fdo::Error::InvalidArgs(error.to_string()))?;
        let record = self
            .core
            .evaluate_insight(&insight, &operation, OffsetDateTime::now_utc())
            .map_err(|error| fdo::Error::Failed(error.to_string()))?;
        record_lifecycle(&record, OffsetDateTime::now_utc()).await;
        let permit_id = record
            .permit_id
            .map_or_else(String::new, |id| id.to_string());
        let encoded = encode(&record).map_err(|error| fdo::Error::Failed(error.to_string()))?;
        Ok((encoded, permit_id))
    }

    async fn claim_permit(&self, permit_id: String) -> fdo::Result<Vec<u8>> {
        let permit_id = Uuid::parse_str(&permit_id)
            .map_err(|_| fdo::Error::InvalidArgs("invalid permit identity".to_owned()))?;
        let permit = self
            .core
            .claim_permit(permit_id, OffsetDateTime::now_utc())
            .map_err(|error| fdo::Error::AccessDenied(error.to_string()))?;
        encode(&permit).map_err(|error| fdo::Error::Failed(error.to_string()))
    }
}

/// Write one decided lifecycle to the Journal, best effort.
///
/// Best effort on purpose, and it is the uncomfortable choice rather than the convenient one. The
/// alternative is refusing to decide when the Journal is unreachable, which turns a recording
/// failure into a host that cannot be repaired — and the reason a host acts on itself at all is that
/// nobody may be there to help it. So a decision stands whether or not it could be written, and the
/// failure is said out loud rather than swallowed, because a record with a hole in it that nobody
/// mentioned is worse than one with a hole somebody logged.
async fn record_lifecycle(record: &crate::ActionRecord, now: OffsetDateTime) {
    let Ok(client) = EventClient::session().await else {
        eprintln!("[cybou-actiond] The Journal is unreachable; this decision was not recorded");
        return;
    };
    for envelope in journal::contributions(record, now) {
        if let Err(error) = client.submit(&envelope).await {
            eprintln!("[cybou-actiond] A lifecycle step was not recorded: {error}");
        }
    }
}
