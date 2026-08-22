// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Browser implementation of the typed Mind boundary.

use async_trait::async_trait;
use cybou_web_contracts::{
    DisclosureProjection, MindProjection, SessionProjection, ShellExecRequest, ShellExecResponse,
    SnapshotProjection,
};
use gloo_net::http::Request;
use serde::de::DeserializeOwned;

use crate::{ClientError, MindClient};

/// Same-origin browser client for the bounded gateway API.
#[derive(Clone, Debug, Default)]
pub struct GatewayMindClient;

impl GatewayMindClient {
    async fn get<T: DeserializeOwned>(path: &str) -> Result<T, ClientError> {
        let response = Request::get(path)
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "{path} returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }
}

#[async_trait(?Send)]
impl MindClient for GatewayMindClient {
    async fn session(&self) -> Result<SessionProjection, ClientError> {
        Self::get("/api/v1/session").await
    }

    async fn snapshot(&self) -> Result<SnapshotProjection, ClientError> {
        Self::get("/api/v1/snapshot").await
    }

    async fn mind(&self) -> Result<MindProjection, ClientError> {
        Self::get("/api/v1/mind").await
    }

    async fn disclosure(&self) -> Result<DisclosureProjection, ClientError> {
        Self::get("/api/v1/disclosure").await
    }

    async fn execute_shell(&self, command: &str) -> Result<ShellExecResponse, ClientError> {
        let response = Request::post("/api/v1/shell/exec")
            .json(&ShellExecRequest {
                command: command.to_owned(),
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/shell/exec returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }
}

impl GatewayMindClient {
    /// Perform login attempt against the gateway.
    pub async fn login(&self, username: &str, password: &str) -> Result<bool, ClientError> {
        let response = Request::post("/api/v1/login")
            .json(&serde_json::json!({
                "username": username,
                "password": password,
            }))
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        Ok(response.status() == 200)
    }

    /// Terminate current session.
    pub async fn logout(&self) -> Result<(), ClientError> {
        let _ = Request::post("/api/v1/logout").send().await;
        Ok(())
    }
}
