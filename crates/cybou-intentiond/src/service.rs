// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Intention1` service implementation on zbus.

// The `#[interface]` expansion emits part of its dispatch surface with the attribute's own span,
// which an `allow` on the impl block cannot reach. Every handler written here is documented.
#![allow(missing_docs)]

use std::sync::Arc;

use cybou_fabric::event_client::EventClient;
use cybou_protocol::{canonical::CanonicalEnvelope, unix_millis};
use time::OffsetDateTime;
use uuid::Uuid;
use zbus::interface;

use crate::{IntentionCore, Resolution};

/// D-Bus Service exporting `org.cybou.Mind.Intention1`.
pub struct Intention1Service {
    core: Arc<IntentionCore>,
}

impl Intention1Service {
    /// Create a new Intention1 D-Bus service handler around `IntentionCore`.
    #[must_use]
    pub fn new(core: Arc<IntentionCore>) -> Self {
        Self { core }
    }
}

/// Record a formed intention in the Journal as a contribution caused by what prompted it.
///
/// Returns whether the Journal accepted it. The intention itself is already durable in this
/// organ's own state by the time this runs, so a Journal that refuses the contribution costs the
/// biography an entry and does not cost the person their commitment.
async fn submit_intention(
    id: &Uuid,
    description: &str,
    trigger: &str,
    cause: Uuid,
    now: OffsetDateTime,
) -> Option<Uuid> {
    let payload = IntentionPayload {
        intention_id: *id,
        description: description.to_owned(),
        trigger: trigger.to_owned(),
    };
    let mut encoded = Vec::new();
    ciborium::into_writer(&payload, &mut encoded).ok()?;

    let message_id = Uuid::new_v4();
    let envelope = CanonicalEnvelope {
        schema_version: 3,
        message_id,
        correlation_id: cause,
        causation_id: cause,
        origin_organ: "intentiond".to_string(),
        origin_node: String::new(),
        kind: 11, // Intention
        wall_time_ms: unix_millis(now),
        monotonic_time: 0,
        logical_clock: 1,
        confidence: 1.0,
        evidence: vec![],
        payload: encoded,
        privacy: 1, // Node
        capability_scope: String::new(),
        sealed: false,
        key_domain_id: Uuid::nil(),
        key_epoch: 0,
        retention_class: 2,
        retention_policy_version: 0,
        retain_until_ms: 0,
        // Personal: a commitment is about the person who made it, and theirs to release.
        sensitivity: 1,
    };

    let client = EventClient::session().await.ok()?;
    client.submit(&envelope).await.ok()?;
    Some(message_id)
}

/// Record the conclusion of an intention as the one terminal Outcome of its contribution.
///
/// The Journal enforces one terminal Outcome per cause, which is what makes a concluded
/// obligation impossible to conclude twice. An intention the Journal never took has nothing to
/// conclude against, so nothing is written for it rather than an Outcome citing thin air.
async fn submit_outcome(
    contribution_id: Uuid,
    resolution: Resolution,
    note: Option<&str>,
    now: OffsetDateTime,
) -> bool {
    let payload = OutcomePayload {
        resolution: format!("{resolution:?}").to_lowercase(),
        note: note.map(ToOwned::to_owned),
    };
    let mut encoded = Vec::new();
    if ciborium::into_writer(&payload, &mut encoded).is_err() {
        return false;
    }

    let envelope = CanonicalEnvelope {
        schema_version: 3,
        message_id: Uuid::new_v4(),
        correlation_id: contribution_id,
        causation_id: contribution_id,
        origin_organ: "intentiond".to_string(),
        origin_node: String::new(),
        kind: 12, // Outcome
        wall_time_ms: unix_millis(now),
        monotonic_time: 0,
        logical_clock: 1,
        confidence: 1.0,
        evidence: vec![],
        payload: encoded,
        privacy: 1, // Node
        capability_scope: String::new(),
        sealed: false,
        key_domain_id: Uuid::nil(),
        key_epoch: 0,
        retention_class: 2,
        retention_policy_version: 0,
        retain_until_ms: 0,
        // Personal: a commitment is about the person who made it, and theirs to release.
        sensitivity: 1,
    };

    let Ok(client) = EventClient::session().await else {
        return false;
    };
    client.submit(&envelope).await.is_ok()
}

/// How an obligation ended.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct OutcomePayload {
    resolution: String,
    note: Option<String>,
}

/// What a recorded intention says. The obligation itself, not a description of the organ.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct IntentionPayload {
    intention_id: Uuid,
    description: String,
    trigger: String,
}

#[allow(
    clippy::unused_async,
    reason = "zbus dispatches every exported handler as a future"
)]
#[interface(name = "org.cybou.Mind.Intention1")]
impl Intention1Service {
    /// Service readiness.
    async fn ready(&self) -> bool {
        true
    }

    /// Form a new intention obligation and return its UUID string (rejects invalid cause UUIDs).
    async fn form(&self, description: String, trigger: String, cause_id: String) -> String {
        let now = OffsetDateTime::now_utc();
        let cause = if cause_id.is_empty() {
            None
        } else {
            match Uuid::parse_str(&cause_id) {
                Ok(id) => Some(id),
                Err(_) => return String::new(), // reject invalid cause
            }
        };
        let Ok(id) = self
            .core
            .form(description.clone(), trigger.clone(), cause, now)
        else {
            return String::new();
        };

        // An intention is a thought, and the admission rules require a thought to cite what it
        // came from: Kind::Intention is not a root kind, so a contribution with no cause and no
        // evidence is refused. An intention formed from nothing therefore stays in this organ's
        // own state and does not enter the biography — which is the rule working, not a failure
        // to record it.
        if let Some(cause) = cause {
            let Some(contribution_id) =
                submit_intention(&id, &description, &trigger, cause, now).await
            else {
                // A caused intention the Journal refused is not a commitment this organ may report
                // as made. Keeping it locally and answering with an identity would tell the caller
                // a biography holds something it does not, so it is withdrawn and the answer says
                // nothing was formed.
                println!("[cybou-intentiond] Intention {id} was not accepted into the Journal");
                let _ = self
                    .core
                    .close(id, Resolution::Obsolete, Some("not recorded"));
                return String::new();
            };
            let _ = self.core.record_contribution(id, contribution_id);
        }

        id.to_string()
    }

    /// Close an intention and say what became of the record, not only of the obligation.
    ///
    /// Answers `closed-and-recorded`, `closed-but-unrecorded`, or `not-closed`. A boolean could
    /// only say the obligation was closed, and a caller told `true` while the Journal never
    /// received the terminal Outcome has been told that a command was sent, not that its outcome
    /// was observed — the exact confusion this system exists to refuse.
    async fn close(&self, intention_id: String, resolution: String, note: String) -> String {
        let Ok(id) = Uuid::parse_str(&intention_id) else {
            return "not-closed".to_string();
        };
        let res = match resolution.to_lowercase().as_str() {
            "fulfilled" => Resolution::Fulfilled,
            "abandoned" => Resolution::Abandoned,
            "obsolete" => Resolution::Obsolete,
            _ => return "not-closed".to_string(), // reject unknown resolution
        };
        let note_opt = if note.is_empty() {
            None
        } else {
            Some(note.as_str())
        };
        // Read the contribution before closing: closing removes the intention, and with it the
        // only record of which contribution an Outcome would have to conclude.
        let contribution_id = self.core.contribution_of(id);

        if self.core.close(id, res, note_opt).is_err() {
            return "not-closed".to_string();
        }

        // An intention the Journal never took has no contribution to conclude, so there is nothing
        // to record and nothing was lost. One the Journal did take, whose Outcome it then refused,
        // leaves a biography that says an obligation was formed and never says how it ended.
        let recorded = match contribution_id {
            None => true,
            Some(contribution_id) => {
                let now = OffsetDateTime::now_utc();
                let accepted = submit_outcome(contribution_id, res, note_opt, now).await;
                if !accepted {
                    println!(
                        "[cybou-intentiond] Outcome for contribution {contribution_id} was not accepted"
                    );
                }
                accepted
            }
        };

        if recorded {
            "closed-and-recorded".to_string()
        } else {
            "closed-but-unrecorded".to_string()
        }
    }

    /// Return open intentions encoded as CBOR.
    async fn open(&self) -> Vec<u8> {
        let list = self.core.open_intentions();
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&list, &mut buf);
        buf
    }

    /// Return open intention count.
    async fn open_count(&self) -> u32 {
        u32::try_from(self.core.open_count()).unwrap_or(u32::MAX)
    }
}
