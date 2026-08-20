// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Resilient typed client for `org.cybou.Mind.Event1`.

#[allow(unused_imports)]
use cybou_protocol::canonical::CanonicalEnvelope;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[allow(unused_imports)]
use crate::EVENT;

/// Outcome of submitting an envelope to Event1.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitOutcome {
    /// Journal sequence number assigned to the contribution.
    pub sequence: u64,
}

/// Read Event1's answer to a submission.
///
/// # Errors
///
/// Returns [`EventClientError::Rejected`] when the Journal refused the contribution, and
/// [`EventClientError::Encoding`] when the reply cannot be read at all. A refusal is an answer,
/// not a transport failure: the caller has to be able to tell "the Journal would not take this"
/// from "the Journal could not be reached".
#[cfg_attr(
    not(target_os = "linux"),
    allow(
        dead_code,
        reason = "the submit path that calls this is Linux-only, while the decoding itself is                   transport-independent and stays testable on every platform"
    )
)]
fn decode_submit_reply(reply: &[u8]) -> Result<SubmitOutcome, EventClientError> {
    let decoded: SubmitReply =
        ciborium::from_reader(reply).map_err(|e| EventClientError::Encoding(e.to_string()))?;

    if !decoded.error.is_empty() {
        return Err(EventClientError::Rejected(decoded.error));
    }

    let sequence = decoded.sequence.parse().map_err(|_| {
        EventClientError::Encoding(format!("sequence {} is not a number", decoded.sequence))
    })?;

    Ok(SubmitOutcome { sequence })
}

/// The reply Event1 actually sends, in the owner's own spelling.
///
/// The sequence is a string because that is the Qt wire spelling Event1 kept, and `error` is empty
/// on acceptance. Decoding some other shape here would report every accepted contribution as a
/// failure while the row sat in the Journal.
#[cfg_attr(
    not(target_os = "linux"),
    allow(
        dead_code,
        reason = "the submit path that calls this is Linux-only, while the decoding itself is                   transport-independent and stays testable on every platform"
    )
)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubmitReply {
    sequence: String,
    error: String,
}

/// Errors occurring during Event1 operations.
#[derive(Debug, Error)]
pub enum EventClientError {
    /// D-Bus connection or method call failed.
    #[error("event1 rpc failed: {0}")]
    Rpc(String),
    /// Serialization or deserialization error.
    #[error("event1 encoding/decoding failed: {0}")]
    Encoding(String),
    /// The submission was rejected by the Journal.
    #[error("event1 rejected submission: {0}")]
    Rejected(String),
}

/// Typed client for interacting with Event1.
pub struct EventClient {
    #[cfg(target_os = "linux")]
    connection: zbus::Connection,
}

impl EventClient {
    /// Create a new `EventClient` connected to the session bus.
    ///
    /// # Errors
    ///
    /// Returns [`EventClientError::Rpc`] if the session bus cannot be reached.
    #[cfg(target_os = "linux")]
    pub async fn session() -> Result<Self, EventClientError> {
        let connection = zbus::Connection::session()
            .await
            .map_err(|e| EventClientError::Rpc(e.to_string()))?;
        Ok(Self { connection })
    }

    /// Submit a canonical envelope to the Journal.
    ///
    /// # Errors
    ///
    /// Returns [`EventClientError::Rpc`] if the call fails and
    /// [`EventClientError::Encoding`] if the envelope or the reply cannot be coded.
    #[cfg(target_os = "linux")]
    pub async fn submit(
        &self,
        envelope: &CanonicalEnvelope,
    ) -> Result<SubmitOutcome, EventClientError> {
        let mut encoded = Vec::new();
        ciborium::into_writer(envelope, &mut encoded)
            .map_err(|e| EventClientError::Encoding(e.to_string()))?;

        let reply: Vec<u8> = self
            .connection
            .call_method(
                Some(EVENT.service),
                EVENT.object_path,
                Some(EVENT.interface),
                "Submit",
                &(encoded,),
            )
            .await
            .map_err(|e| EventClientError::Rpc(e.to_string()))?
            .body()
            .deserialize()
            .map_err(|e| EventClientError::Rpc(e.to_string()))?;

        decode_submit_reply(&reply)
    }

    /// Replay contributions strictly after `after_sequence`.
    ///
    /// # Errors
    ///
    /// Returns [`EventClientError::Rpc`] if the call fails and
    /// [`EventClientError::Encoding`] if the reply cannot be decoded.
    #[cfg(target_os = "linux")]
    pub async fn replay(
        &self,
        after_sequence: u64,
        limit: u32,
    ) -> Result<Vec<CanonicalEnvelope>, EventClientError> {
        let reply: Vec<u8> = self
            .connection
            .call_method(
                Some(EVENT.service),
                EVENT.object_path,
                Some(EVENT.interface),
                "Replay",
                &(after_sequence, i32::try_from(limit).unwrap_or(i32::MAX)),
            )
            .await
            .map_err(|e| EventClientError::Rpc(e.to_string()))?
            .body()
            .deserialize()
            .map_err(|e| EventClientError::Rpc(e.to_string()))?;

        if reply.is_empty() {
            return Ok(Vec::new());
        }

        let envelopes: Vec<CanonicalEnvelope> = ciborium::from_reader(reply.as_slice())
            .map_err(|e| EventClientError::Encoding(e.to_string()))?;

        Ok(envelopes)
    }

    /// Retrieve the newest contributions, oldest first within the returned window.
    ///
    /// This is not `replay(0, limit)`: replay returns the beginning of the Journal, which is the
    /// wrong end for anything that wants to know what just happened.
    ///
    /// # Errors
    ///
    /// Returns [`EventClientError::Rpc`] if the call fails and
    /// [`EventClientError::Encoding`] if the reply cannot be decoded.
    #[cfg(target_os = "linux")]
    pub async fn recent(&self, limit: u32) -> Result<Vec<CanonicalEnvelope>, EventClientError> {
        let reply: Vec<u8> = self
            .connection
            .call_method(
                Some(EVENT.service),
                EVENT.object_path,
                Some(EVENT.interface),
                "Recent",
                &(i32::try_from(limit).unwrap_or(i32::MAX),),
            )
            .await
            .map_err(|e| EventClientError::Rpc(e.to_string()))?
            .body()
            .deserialize()
            .map_err(|e| EventClientError::Rpc(e.to_string()))?;

        if reply.is_empty() {
            return Ok(Vec::new());
        }

        ciborium::from_reader(reply.as_slice())
            .map_err(|e| EventClientError::Encoding(e.to_string()))
    }

    /// Retrieve the head envelope, if any.
    ///
    /// # Errors
    ///
    /// Returns [`EventClientError::Rpc`] if the call fails and
    /// [`EventClientError::Encoding`] if the reply cannot be decoded.
    #[cfg(target_os = "linux")]
    pub async fn head(&self) -> Result<Option<CanonicalEnvelope>, EventClientError> {
        let reply: Vec<u8> = self
            .connection
            .call_method(
                Some(EVENT.service),
                EVENT.object_path,
                Some(EVENT.interface),
                "Head",
                &(),
            )
            .await
            .map_err(|e| EventClientError::Rpc(e.to_string()))?
            .body()
            .deserialize()
            .map_err(|e| EventClientError::Rpc(e.to_string()))?;

        if reply.is_empty() {
            return Ok(None);
        }

        let envelope: CanonicalEnvelope = ciborium::from_reader(reply.as_slice())
            .map_err(|e| EventClientError::Encoding(e.to_string()))?;

        Ok(Some(envelope))
    }

    /// Retrieve the current erasure epoch.
    ///
    /// # Errors
    ///
    /// Returns [`EventClientError::Rpc`] if the call fails.
    #[cfg(target_os = "linux")]
    pub async fn erasure_epoch(&self) -> Result<u64, EventClientError> {
        self.connection
            .call_method(
                Some(EVENT.service),
                EVENT.object_path,
                Some(EVENT.interface),
                "ErasureEpoch",
                &(),
            )
            .await
            .map_err(|e| EventClientError::Rpc(e.to_string()))?
            .body()
            .deserialize()
            .map_err(|e| EventClientError::Rpc(e.to_string()))
    }

    /// Retrieve the total number of contributions in the Journal.
    ///
    /// # Errors
    ///
    /// Returns [`EventClientError::Rpc`] if the call fails.
    #[cfg(target_os = "linux")]
    pub async fn count(&self) -> Result<u64, EventClientError> {
        let count: u64 = self
            .connection
            .call_method(
                Some(EVENT.service),
                EVENT.object_path,
                Some(EVENT.interface),
                "Count",
                &(),
            )
            .await
            .map_err(|e| EventClientError::Rpc(e.to_string()))?
            .body()
            .deserialize()
            .map_err(|e| EventClientError::Rpc(e.to_string()))?;

        Ok(count)
    }
}

/// How many rows one replay page asks for while catching up.
#[cfg(target_os = "linux")]
const REPLAY_PAGE: u32 = 512;

/// How long to wait before following again after the connection to Event1 is lost.
#[cfg(target_os = "linux")]
const RECONNECT_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

/// Deliver everything between a cursor and an announced sequence, from the Journal.
///
/// Returns how far it got. A caller that does not reach `up_to - 1` must not treat the announced
/// contribution as the next one, because the rows before it have not been seen.
#[cfg(target_os = "linux")]
async fn deliver_gap<F>(
    client: &EventClient,
    mut cursor: u64,
    up_to: u64,
    on_contribution: &mut F,
) -> u64
where
    F: FnMut(u64, &CanonicalEnvelope),
{
    while cursor + 1 < up_to {
        let Ok(page) = client.replay(cursor, REPLAY_PAGE).await else {
            break;
        };
        if page.is_empty() {
            break;
        }
        for envelope in &page {
            if cursor + 1 >= up_to {
                break;
            }
            cursor += 1;
            on_contribution(cursor, envelope);
        }
    }
    cursor
}

/// Follow every contribution from a position, catching up first and then staying current.
///
/// Each of the derived organs had written its own version of this and each got it wrong in a
/// different way: one replayed a single page and called it caught up, one subscribed without
/// replaying at all, and all of them replayed before subscribing, so a contribution accepted
/// between the two steps belonged to neither.
///
/// The order here is the whole point. Subscribing first means the stream already holds anything
/// accepted while the catch-up runs; the catch-up then reads pages until it reaches the head it
/// saw; and the live phase skips sequences the catch-up already delivered. Nothing arrives twice
/// and nothing falls between the two.
///
/// Sequence numbers are derived from position because the Journal is contiguous — the writer
/// enforces it and the verifier checks it — and `Replay` answers with rows after a sequence in
/// order.
///
/// One attempt: it returns when the connection, the subscription, or the stream itself is lost,
/// leaving `cursor` at the last contribution actually delivered so the next attempt resumes there.
///
/// # Errors
///
/// Returns [`EventClientError`] when the session bus, the subscription, or a catch-up read fails,
/// and when the acceptance stream ends. Per-message failures skip that message without advancing.
#[cfg(target_os = "linux")]
async fn follow_once<F>(cursor: &mut u64, on_contribution: &mut F) -> Result<(), EventClientError>
where
    F: FnMut(u64, &CanonicalEnvelope),
{
    use futures_util::StreamExt as _;

    let connection = zbus::Connection::session()
        .await
        .map_err(|e| EventClientError::Rpc(e.to_string()))?;
    let proxy = zbus::Proxy::new(
        &connection,
        EVENT.service,
        EVENT.object_path,
        EVENT.interface,
    )
    .await
    .map_err(|e| EventClientError::Rpc(e.to_string()))?;

    // Subscribe before reading the head, so the gap between them is covered by the stream.
    let mut accepted = proxy
        .receive_signal("Accepted")
        .await
        .map_err(|e| EventClientError::Rpc(e.to_string()))?;

    let client = EventClient { connection };
    let head = client.count().await?;

    while *cursor < head {
        let page = client.replay(*cursor, REPLAY_PAGE).await?;
        if page.is_empty() {
            break;
        }
        for envelope in &page {
            *cursor += 1;
            on_contribution(*cursor, envelope);
        }
    }

    while let Some(message) = accepted.next().await {
        let Ok((encoded, sequence)) = message.body().deserialize::<(Vec<u8>, u64)>() else {
            // An announcement that cannot be read is not an announcement that nothing happened.
            // The cursor stays put, and the gap is filled from the Journal on the next one.
            continue;
        };
        // Already delivered by the catch-up: the stream held it while that ran.
        if sequence <= *cursor {
            continue;
        }

        // Signals are not a delivery guarantee. One arriving out of order, or one lost between a
        // sender and a subscriber, would otherwise move the cursor past a contribution nobody
        // read — silently, for ever, because nothing looks back. Anything missing is fetched from
        // the Journal, which is the record, before the announcement is acted on.
        if sequence > *cursor + 1 {
            *cursor = deliver_gap(&client, *cursor, sequence, on_contribution).await;
            if *cursor + 1 != sequence {
                // The gap could not be closed. Leaving the cursor behind means the next signal
                // tries again rather than declaring the missing rows delivered.
                continue;
            }
        }

        let Ok(envelope) = ciborium::from_reader::<CanonicalEnvelope, _>(encoded.as_slice()) else {
            continue;
        };
        *cursor = sequence;
        on_contribution(sequence, &envelope);
    }

    // The stream ending is not the end of the biography: the bus went away while this organ is
    // still expected to be following. Say so rather than returning as though the work were done.
    Err(EventClientError::Rpc(
        "the Event1 acceptance stream ended".into(),
    ))
}

/// Follow every contribution from `from_sequence` onwards, for as long as this process runs.
///
/// A follower that stopped at the first lost connection would leave its organ answering from a
/// projection frozen at whatever moment the bus blinked, with nothing saying so. Instead each
/// attempt resumes from the cursor the last one reached, so a reconnection replays exactly what
/// was missed and no contribution is delivered twice.
///
/// # Errors
///
/// Only when following is abandoned entirely, which today means never: it retries indefinitely.
#[cfg(target_os = "linux")]
pub async fn follow_contributions<F>(
    from_sequence: u64,
    mut on_contribution: F,
) -> Result<(), EventClientError>
where
    F: FnMut(u64, &CanonicalEnvelope),
{
    let mut cursor = from_sequence;
    loop {
        if let Err(error) = follow_once(&mut cursor, &mut on_contribution).await {
            println!("[cybou-fabric] Following Event1 stopped at {cursor}: {error}; reconnecting");
        }
        tokio::time::sleep(RECONNECT_DELAY).await;
    }
}

#[cfg(test)]
mod tests {
    use super::{EventClientError, decode_submit_reply};

    #[derive(serde::Serialize)]
    struct OwnerReply<'a> {
        sequence: &'a str,
        error: &'a str,
    }

    /// Encode exactly what Event1 sends back.
    fn owner_reply(sequence: &str, error: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        ciborium::into_writer(&OwnerReply { sequence, error }, &mut bytes)
            .expect("encode owner reply");
        bytes
    }

    #[test]
    fn an_accepted_contribution_is_not_reported_as_a_failure() {
        // Decoding some other shape here turned every accepted contribution into an error while
        // its row sat in the Journal, which is how intentions came to be recorded and forgotten.
        let outcome = decode_submit_reply(&owner_reply("42", "")).expect("accepted");
        assert_eq!(outcome.sequence, 42);
    }

    #[test]
    fn a_refusal_is_distinguishable_from_a_transport_failure() {
        let error =
            decode_submit_reply(&owner_reply("0", "duplicate identity")).expect_err("refused");
        assert!(
            matches!(error, EventClientError::Rejected(reason) if reason == "duplicate identity")
        );
    }
}
