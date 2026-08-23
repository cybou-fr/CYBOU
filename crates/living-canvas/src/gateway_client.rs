// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Browser implementation of the typed Mind boundary.

use async_trait::async_trait;
use cybou_web_contracts::{
    DirectoryListingProjection, DisclosureProjection, FileContentProjection, FilePathRequest,
    MindProjection, SessionProjection, ShellCloseRequest, ShellExecRequest, ShellExecResponse,
    SnapshotProjection,
};
use gloo_net::http::Request;
use serde::de::DeserializeOwned;

use crate::{ClientError, MindClient};

/// Same-origin browser client for the bounded gateway API.
#[derive(Clone, Debug, Default)]
pub struct GatewayMindClient;

impl GatewayMindClient {
    /// Ask one route about one sandbox path, and decode its typed answer.
    async fn post_path<T: DeserializeOwned>(route: &str, path: &str) -> Result<T, ClientError> {
        let response = Request::post(route)
            .json(&FilePathRequest {
                path: path.to_owned(),
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "{route} returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

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

    async fn insight(&self) -> Result<cybou_web_contracts::InsightProjection, ClientError> {
        Self::get("/api/v1/insight").await
    }

    async fn list_directory(&self, path: &str) -> Result<DirectoryListingProjection, ClientError> {
        Self::post_path("/api/v1/files/list", path).await
    }

    async fn read_text_file(&self, path: &str) -> Result<FileContentProjection, ClientError> {
        Self::post_path("/api/v1/files/read", path).await
    }

    async fn close_shell(&self, instance: u32) -> Result<(), ClientError> {
        let response = Request::post("/api/v1/shell/close")
            .json(&ShellCloseRequest { instance })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if response.ok() {
            Ok(())
        } else {
            Err(ClientError::GatewayRequest(format!(
                "/api/v1/shell/close returned HTTP {}",
                response.status()
            )))
        }
    }

    async fn execute_shell(
        &self,
        command: &str,
        instance: u32,
    ) -> Result<ShellExecResponse, ClientError> {
        let response = Request::post("/api/v1/shell/exec")
            .json(&ShellExecRequest {
                command: command.to_owned(),
                instance,
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
