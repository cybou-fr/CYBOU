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

    /// What was proposed, argued and decided for one proposal.
    ///
    /// Writing the lifecycle down was half of answering *why did nginx restart on the fourteenth*.
    /// This is the other half: a record nothing can read is a record only in the sense that the bytes
    /// exist. It answers from what this owner holds, which after a restart is what it read back out
    /// of the Journal.
    ///
    /// No permit is ever in the answer. It is not stored and not restored, and a lifecycle read a
    /// month later is a history rather than a key.
    async fn record(&self, proposal_id: String) -> fdo::Result<Vec<u8>> {
        let proposal_id = Uuid::parse_str(&proposal_id)
            .map_err(|_| fdo::Error::InvalidArgs("invalid proposal identity".to_owned()))?;
        let record = self
            .core
            .record(proposal_id)
            .ok_or_else(|| fdo::Error::FileNotFound("no such proposal".to_owned()))?;
        encode(&record).map_err(|error| fdo::Error::Failed(error.to_string()))
    }

    /// Record what was carried out under a decision this owner made.
    ///
    /// Reported here rather than kept by whoever carried it out, so *what was authorized* and *what
    /// was done* are one record. Two records somebody has to correlate afterwards is the shape that
    /// makes a month-old question unanswerable.
    async fn record_attempt(&self, attempt: Vec<u8>) -> fdo::Result<()> {
        let attempt: cybou_protocol::action::ExecutionAttempt =
            decode(&attempt).map_err(|error| fdo::Error::InvalidArgs(error.to_string()))?;
        self.core
            .record_attempt(attempt)
            .map_err(|error| fdo::Error::Failed(error.to_string()))
    }

    /// Record what the host saw for itself afterwards.
    async fn record_outcome(&self, outcome: Vec<u8>) -> fdo::Result<()> {
        let outcome: cybou_protocol::action::ActionOutcome =
            decode(&outcome).map_err(|error| fdo::Error::InvalidArgs(error.to_string()))?;
        self.core
            .record_outcome(outcome)
            .map_err(|error| fdo::Error::Failed(error.to_string()))
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
    let contributions = match journal::contributions(record, now) {
        Ok(contributions) => contributions,
        Err(why) => {
            eprintln!("[cybou-actiond] This decision cannot be recorded: {why}");
            return;
        }
    };
    for envelope in contributions {
        if let Err(error) = client.submit(&envelope).await {
            eprintln!("[cybou-actiond] A lifecycle step was not recorded: {error}");
        }
    }
}
