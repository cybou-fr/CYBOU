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
