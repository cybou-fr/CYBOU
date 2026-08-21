// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The gateway's side of the conversation with `cybou-authd`.
//!
//! One connection per attempt, one message each way, and no state. The helper is the only thing on
//! the host that can answer this question, and this is the only thing that asks it.

use async_trait::async_trait;
use serde::Serialize;

use crate::access::CredentialVerifier;

/// A verifier that asks the privileged helper over its unix socket.
pub struct HelperVerifier {
    socket_path: std::path::PathBuf,
}

impl HelperVerifier {
    /// Ask the helper listening at `socket_path`.
    #[must_use]
    pub fn at(socket_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }
}

/// The request shape `cybou-authd` reads. Kept in step with `cybou_authd::Request` by the
/// integration gate, which runs a real helper rather than a stub.
#[derive(Serialize)]
struct Ask<'a> {
    username: &'a str,
    password: &'a str,
}

#[async_trait]
impl CredentialVerifier for HelperVerifier {
    async fn verify(&self, username: &str, password: &str) -> bool {
        use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

        let Ok(mut stream) = tokio::net::UnixStream::connect(&self.socket_path).await else {
            // A helper that is not there cannot vouch for anybody, and not knowing is not a reason
            // to let someone in.
            return false;
        };

        let mut encoded = Vec::new();
        if ciborium::into_writer(&Ask { username, password }, &mut encoded).is_err() {
            return false;
        }
        if stream.write_all(&encoded).await.is_err() {
            return false;
        }
        // The helper reads to end of stream, so the write side has to close before it will answer.
        if stream.shutdown().await.is_err() {
            return false;
        }

        let mut answer = Vec::new();
        if stream.read_to_end(&mut answer).await.is_err() {
            return false;
        }
        ciborium::from_reader::<Answer, _>(answer.as_slice())
            .is_ok_and(|answer| answer.authenticated)
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Answer {
    authenticated: bool,
}

/// Records disclosures by submitting them to Event1.
pub struct JournalSink;

#[async_trait]
impl crate::DisclosureSink for JournalSink {
    async fn record(&self, envelope: &cybou_protocol::canonical::CanonicalEnvelope) -> bool {
        // A fresh connection per record. Deliveries are rare — one per change in what a consumer is
        // being supplied — so a held connection would exist mostly to be stale.
        let Ok(client) = cybou_fabric::event_client::EventClient::session().await else {
            return false;
        };
        client.submit(envelope).await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_helper_that_is_not_there_vouches_for_nobody() {
        let verifier = HelperVerifier::at("/nonexistent/cybou-auth.sock");
        assert!(!verifier.verify("alice", "hunter2").await);
    }
}
