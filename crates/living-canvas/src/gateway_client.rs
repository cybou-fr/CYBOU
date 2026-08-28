// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Browser implementation of the typed Mind boundary.

use async_trait::async_trait;
use cybou_web_contracts::{
    DirectoryListingProjection, DisclosureProjection, FileContentProjection, FileCreateRequest,
    FilePathRequest, FileWriteProjection, FileWriteRequest, MindProjection, SessionProjection,
    ShellCloseRequest, ShellExecRequest, ShellExecResponse, SnapshotProjection,
    UserDraftDeleteRequest, UserDraftListProjection, UserDraftProjection, UserDraftSaveRequest,
};
use gloo_net::http::Request;
use serde::de::DeserializeOwned;

use crate::{ClientError, MindClient};

/// Same-origin browser client for the bounded gateway API.
#[derive(Clone, Debug, Default)]
pub struct GatewayMindClient;

impl GatewayMindClient {
    /// Exclusively create a new file inside the authenticated jail.
    pub async fn create_text_file(
        &self,
        request: &FileCreateRequest,
    ) -> Result<FileWriteProjection, ClientError> {
        let response = Request::post("/api/v1/files/create")
            .json(request)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if response.status() == 409 {
            return Err(ClientError::FileAlreadyExists);
        }
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/files/create returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    /// List durable drafts for the authenticated principal.
    pub async fn drafts(&self) -> Result<UserDraftListProjection, ClientError> {
        Self::get("/api/v1/drafts").await
    }

    /// Persist one bounded editor recovery snapshot.
    pub async fn save_draft(
        &self,
        request: &UserDraftSaveRequest,
    ) -> Result<UserDraftProjection, ClientError> {
        let response = Request::post("/api/v1/drafts/save")
            .json(request)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/drafts/save returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    /// Delete one recovery snapshot after save or explicit discard.
    pub async fn delete_draft(&self, draft_id: &str) -> Result<(), ClientError> {
        let response = Request::post("/api/v1/drafts/delete")
            .json(&UserDraftDeleteRequest {
                draft_id: draft_id.to_owned(),
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if response.ok() {
            Ok(())
        } else {
            Err(ClientError::GatewayRequest(format!(
                "/api/v1/drafts/delete returned HTTP {}",
                response.status()
            )))
        }
    }

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

    async fn agents(&self) -> Result<Vec<cybou_protocol::agent::SessionView>, ClientError> {
        Self::get("/api/v1/agents").await
    }

    async fn launch_agent(
        &self,
        request: &cybou_protocol::agent::LaunchRequest,
    ) -> Result<cybou_protocol::agent::SessionView, ClientError> {
        let response = Request::post("/api/v1/agents")
            .json(request)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/agents returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn agent_offers(
        &self,
    ) -> Result<cybou_protocol::agent::AgentOffersResponse, ClientError> {
        Self::get("/api/v1/agents/offers").await
    }

    async fn actions(
        &self,
        cause_id: Option<uuid::Uuid>,
    ) -> Result<Vec<cybou_web_contracts::ActionRecordProjection>, ClientError> {
        match cause_id {
            Some(id) => Self::get(&format!("/api/v1/actions?cause={id}")).await,
            None => Self::get("/api/v1/actions/recent").await,
        }
    }

    async fn stop_agent(&self, capsule_id: uuid::Uuid) -> Result<(), ClientError> {
        let path = format!("/api/v1/agents/{capsule_id}");
        let response = Request::delete(&path)
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if response.ok() {
            Ok(())
        } else {
            Err(ClientError::GatewayRequest(format!(
                "{path} returned HTTP {}",
                response.status()
            )))
        }
    }

    async fn list_directory(&self, path: &str) -> Result<DirectoryListingProjection, ClientError> {
        Self::post_path("/api/v1/files/list", path).await
    }

    async fn read_text_file(&self, path: &str) -> Result<FileContentProjection, ClientError> {
        Self::post_path("/api/v1/files/read", path).await
    }

    async fn write_text_file(
        &self,
        request: &FileWriteRequest,
    ) -> Result<FileWriteProjection, ClientError> {
        let response = Request::post("/api/v1/files/write")
            .json(request)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if response.status() == 409 {
            return Err(ClientError::FileChangedSinceRead);
        }
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/files/write returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
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
