// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Typed ACP initialization over an agent subprocess's stdio.

use std::time::Duration;

use agent_client_protocol::schema::{
    ProtocolVersion,
    v1::{Implementation, InitializeRequest},
};
use agent_client_protocol::{AcpAgent, AcpAgentConfig, Client};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const DEFAULT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Stable facts learned from the ACP `initialize` handshake.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentHandshake {
    /// Negotiated ACP wire version.
    pub protocol_version: u16,
    /// Agent implementation name, when advertised.
    pub name: Option<String>,
    /// Human-readable agent title, when advertised.
    pub title: Option<String>,
    /// Agent implementation version, when advertised.
    pub version: Option<String>,
    /// Authentication choices advertised during initialization.
    pub authentication_methods: Vec<AuthenticationMethod>,
    /// Agent capabilities exactly as represented by the official ACP schema.
    pub capabilities: serde_json::Value,
}

/// One authentication choice advertised by an ACP agent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationMethod {
    /// Protocol identity supplied back to `authenticate`.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Optional explanation supplied by the agent.
    pub description: Option<String>,
}

/// Failure to establish and initialize an ACP subprocess.
#[derive(Debug, Error)]
pub enum AcpClientError {
    /// The agent did not complete initialization before the fixed deadline.
    #[error("ACP initialize timed out")]
    Timeout,
    /// The official ACP runtime refused the process or wire exchange.
    #[error("ACP initialize failed: {0}")]
    Protocol(String),
    /// The official schema produced a value that could not be projected.
    #[error("ACP capability projection failed: {0}")]
    Projection(#[from] serde_json::Error),
}

/// A bounded client for the stable ACP v1 initialization handshake.
#[derive(Clone, Debug)]
pub struct AcpClient {
    handshake_timeout: Duration,
}

impl Default for AcpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AcpClient {
    /// Create a client with the production handshake deadline.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
        }
    }

    /// Initialize one agent process over ACP stdio and then close it.
    ///
    /// The supplied command must be a capsule entrypoint prepared by the caller; this low-level
    /// protocol client is not a sandbox. It discovers protocol identity and capabilities only. It
    /// does not authenticate, create a session, install software, or grant a Cybou capability.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the process cannot start, the wire exchange is invalid, the
    /// response cannot be projected, or the agent does not answer before the deadline.
    pub async fn initialize(
        &self,
        process: AcpAgentConfig,
    ) -> Result<AgentHandshake, AcpClientError> {
        let exchange = Client.builder().name("cybou-acp").connect_with(
            AcpAgent::new(process),
            async |connection| {
                connection
                    .send_request(
                        InitializeRequest::new(ProtocolVersion::V1).client_info(
                            Implementation::new("cybou-acp", env!("CARGO_PKG_VERSION"))
                                .title("Cybou ACP Client"),
                        ),
                    )
                    .block_task()
                    .await
            },
        );
        let response = tokio::time::timeout(self.handshake_timeout, exchange)
            .await
            .map_err(|_| AcpClientError::Timeout)?
            .map_err(|error| AcpClientError::Protocol(error.to_string()))?;
        if response.protocol_version != ProtocolVersion::V1 {
            return Err(AcpClientError::Protocol(format!(
                "agent selected unsupported protocol version {}",
                response.protocol_version
            )));
        }

        let authentication_methods = response
            .auth_methods
            .iter()
            .map(|method| AuthenticationMethod {
                id: method.id().to_string(),
                name: method.name().to_owned(),
                description: method.description().map(ToOwned::to_owned),
            })
            .collect();
        let (name, title, version) = response.agent_info.map_or((None, None, None), |info| {
            (Some(info.name), info.title, Some(info.version))
        });

        Ok(AgentHandshake {
            protocol_version: response.protocol_version.as_u16(),
            name,
            title,
            version,
            authentication_methods,
            capabilities: serde_json::to_value(response.agent_capabilities)?,
        })
    }
}
