// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Resilient typed client for `org.cybou.Mind.Event1`.

#[allow(unused_imports)]
use cybou_protocol::canonical::CanonicalEnvelope;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[allow(unused_imports)]
use crate::EVENT;

/// Outcome of submitting an envelope to Event1.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitOutcome {
    /// Journal sequence number assigned to the contribution.
    pub sequence: u64,
    /// Message ID of the accepted contribution.
    pub message_id: Uuid,
    /// SHA-256 hash chaining this row to the previous row.
    pub hash: Vec<u8>,
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
    /// Create a new EventClient connected to the session bus.
    #[cfg(target_os = "linux")]
    pub async fn session() -> Result<Self, EventClientError> {
        let connection = zbus::Connection::session()
            .await
            .map_err(|e| EventClientError::Rpc(e.to_string()))?;
        Ok(Self { connection })
    }

    /// Submit a canonical envelope to the Journal.
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
            .map_err(|e| EventClientError::Rpc(e.to_string()))?;

        let outcome: SubmitOutcome = ciborium::from_reader(reply.as_slice())
            .map_err(|e| EventClientError::Encoding(e.to_string()))?;

        Ok(outcome)
    }

    /// Replay contributions strictly after `after_sequence`.
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
                &(after_sequence, limit as i32),
            )
            .await
            .map_err(|e| EventClientError::Rpc(e.to_string()))?
            .body()
            .map_err(|e| EventClientError::Rpc(e.to_string()))?;

        if reply.is_empty() {
            return Ok(Vec::new());
        }

        let envelopes: Vec<CanonicalEnvelope> = ciborium::from_reader(reply.as_slice())
            .map_err(|e| EventClientError::Encoding(e.to_string()))?;

        Ok(envelopes)
    }

    /// Retrieve the head envelope, if any.
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
            .map_err(|e| EventClientError::Rpc(e.to_string()))?;

        if reply.is_empty() {
            return Ok(None);
        }

        let envelope: CanonicalEnvelope = ciborium::from_reader(reply.as_slice())
            .map_err(|e| EventClientError::Encoding(e.to_string()))?;

        Ok(Some(envelope))
    }

    /// Retrieve the total number of contributions in the Journal.
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
            .map_err(|e| EventClientError::Rpc(e.to_string()))?;

        Ok(count)
    }
}
