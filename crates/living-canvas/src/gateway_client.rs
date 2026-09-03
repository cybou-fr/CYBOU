// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Browser implementation of the typed Mind boundary.

use async_trait::async_trait;
use cybou_web_contracts::{
    DirectoryListingProjection, DisclosureProjection, FileContentProjection, FileCreateRequest,
    FilePathRequest, FileWriteProjection, FileWriteRequest, HostDirectoryCreateRequest,
    HostDirectoryListingProjection, HostFileCreateRequest, HostFileWriteRequest,
    HostPathCopyRequest, HostPathDeleteRequest, HostPathRenameRequest, MindProjection,
    SessionProjection, SnapshotProjection, UserDraftDeleteRequest, UserDraftListProjection,
    UserDraftProjection, UserDraftSaveRequest,
};
use gloo_net::http::Request;
use serde::de::DeserializeOwned;

use crate::{ClientError, MindClient};

/// Same-origin browser client for the bounded gateway API.
#[derive(Clone, Copy, Debug, Default)]
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

    async fn control_agent(
        &self,
        capsule_id: uuid::Uuid,
        action: cybou_protocol::agent::CapsuleAction,
    ) -> Result<(), ClientError> {
        let path = format!("/api/v1/agents/{capsule_id}/action");
        let response = Request::post(&path)
            .json(&cybou_web_contracts::CapsuleControlRequest { action })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
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

    async fn agent_telemetry(
        &self,
        capsule_id: uuid::Uuid,
    ) -> Result<cybou_web_contracts::CapsuleTelemetryProjection, ClientError> {
        let path = format!("/api/v1/agents/{capsule_id}/telemetry");
        let response = Request::get(&path)
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

    async fn create_file(
        &self,
        request: &cybou_web_contracts::FileCreateRequest,
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

    async fn confirm_action(
        &self,
        request: &cybou_web_contracts::ConfirmActionRequest,
    ) -> Result<cybou_web_contracts::ActionRecordProjection, ClientError> {
        let response = Request::post("/api/v1/actions/confirm")
            .json(request)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/actions/confirm returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn upload_file(
        &self,
        request: &cybou_web_contracts::FileUploadRequest,
    ) -> Result<cybou_web_contracts::FileUploadProjection, ClientError> {
        let response = Request::post("/api/v1/files/upload")
            .json(request)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        // The gateway refuses rather than replaces, so this is the ordinary answer to dropping a
        // file onto a directory that already holds one by that name, not an error condition.
        if response.status() == 409 {
            return Err(ClientError::FileAlreadyExists);
        }
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/files/upload returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn download_file(&self, path: &str) -> Result<Vec<u8>, ClientError> {
        let response = Request::post("/api/v1/files/download")
            .json(&cybou_web_contracts::FilePathRequest {
                path: path.to_owned(),
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/files/download returned HTTP {}",
                response.status()
            )));
        }
        response
            .binary()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn host_list_directory(
        &self,
        path: &str,
    ) -> Result<HostDirectoryListingProjection, ClientError> {
        Self::post_path("/api/v1/host-files/list", path).await
    }

    async fn host_read_file(&self, path: &str) -> Result<FileContentProjection, ClientError> {
        Self::post_path("/api/v1/host-files/read", path).await
    }

    async fn host_write_file(
        &self,
        request: &HostFileWriteRequest,
    ) -> Result<FileWriteProjection, ClientError> {
        let response = Request::post("/api/v1/host-files/write")
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
                "/api/v1/host-files/write returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn host_create_file(
        &self,
        request: &HostFileCreateRequest,
    ) -> Result<FileWriteProjection, ClientError> {
        let response = Request::post("/api/v1/host-files/create")
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
                "/api/v1/host-files/create returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn host_create_directory(
        &self,
        request: &HostDirectoryCreateRequest,
    ) -> Result<(), ClientError> {
        let response = Request::post("/api/v1/host-files/mkdir")
            .json(request)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if response.ok() {
            Ok(())
        } else {
            Err(ClientError::GatewayRequest(format!(
                "/api/v1/host-files/mkdir returned HTTP {}",
                response.status()
            )))
        }
    }

    async fn host_rename_path(&self, request: &HostPathRenameRequest) -> Result<(), ClientError> {
        let response = Request::post("/api/v1/host-files/rename")
            .json(request)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if response.ok() {
            Ok(())
        } else {
            Err(ClientError::GatewayRequest(format!(
                "/api/v1/host-files/rename returned HTTP {}",
                response.status()
            )))
        }
    }

    async fn host_delete_path(&self, request: &HostPathDeleteRequest) -> Result<(), ClientError> {
        let response = Request::post("/api/v1/host-files/delete")
            .json(request)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if response.ok() {
            Ok(())
        } else {
            Err(ClientError::GatewayRequest(format!(
                "/api/v1/host-files/delete returned HTTP {}",
                response.status()
            )))
        }
    }

    async fn host_copy_path(&self, request: &HostPathCopyRequest) -> Result<(), ClientError> {
        let response = Request::post("/api/v1/host-files/copy")
            .json(request)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if response.ok() {
            Ok(())
        } else {
            Err(ClientError::GatewayRequest(format!(
                "/api/v1/host-files/copy returned HTTP {}",
                response.status()
            )))
        }
    }

    async fn list_operations(
        &self,
    ) -> Result<cybou_web_contracts::OperationsListProjection, ClientError> {
        let response = Request::get("/api/v1/operations")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/operations returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn get_operation_logs(
        &self,
        id: uuid::Uuid,
    ) -> Result<cybou_web_contracts::OperationLogsProjection, ClientError> {
        let response = Request::get(&format!("/api/v1/operations/{id}/logs"))
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/operations/{id}/logs returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn cancel_operation(
        &self,
        id: uuid::Uuid,
        reason: Option<String>,
    ) -> Result<cybou_protocol::operation::CancelOutcome, ClientError> {
        let response = Request::post("/api/v1/operations/cancel")
            .json(&cybou_web_contracts::OperationCancelRequest {
                operation_id: id,
                reason,
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        match response.status() {
            // 202 is the owner accepting a request; only the worker may later publish Cancelled.
            202 => Ok(cybou_protocol::operation::CancelOutcome::CancellationAccepted),
            200 => Ok(cybou_protocol::operation::CancelOutcome::CancellationConfirmed),
            status => Err(ClientError::GatewayRequest(format!(
                "/api/v1/operations/cancel returned HTTP {status}"
            ))),
        }
    }

    async fn list_notifications(
        &self,
    ) -> Result<cybou_web_contracts::NotificationsListProjection, ClientError> {
        let response = Request::get("/api/v1/notifications")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/notifications returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn dismiss_notifications(
        &self,
        id: Option<uuid::Uuid>,
        dismiss_all: bool,
    ) -> Result<(), ClientError> {
        let response = Request::post("/api/v1/notifications/dismiss")
            .json(&cybou_web_contracts::NotificationDismissRequest {
                notification_id: id,
                dismiss_all,
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if response.ok() {
            Ok(())
        } else {
            Err(ClientError::GatewayRequest(format!(
                "/api/v1/notifications/dismiss returned HTTP {}",
                response.status()
            )))
        }
    }

    async fn execute_notification_action(
        &self,
        id: uuid::Uuid,
        action_id: &str,
    ) -> Result<String, ClientError> {
        let response = Request::post("/api/v1/notifications/action")
            .json(&cybou_web_contracts::NotificationActionRequest {
                notification_id: id,
                action_id: action_id.to_owned(),
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/notifications/action returned HTTP {}",
                response.status()
            )));
        }
        let outcome: serde_json::Value = response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        Ok(outcome["outcome"]
            .as_str()
            .unwrap_or("Action executed")
            .to_owned())
    }

    async fn list_services(
        &self,
    ) -> Result<cybou_web_contracts::ServicesListProjection, ClientError> {
        let response = Request::get("/api/v1/system/services")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/services returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn execute_service_action(
        &self,
        name: &str,
        action: cybou_protocol::system::ServiceAction,
    ) -> Result<cybou_web_contracts::ActionRecordProjection, ClientError> {
        let response = Request::post("/api/v1/system/services/action")
            .json(&cybou_web_contracts::ServiceActionRequest {
                name: name.to_owned(),
                action,
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/services/action returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn list_processes(
        &self,
    ) -> Result<cybou_web_contracts::ProcessesListProjection, ClientError> {
        let response = Request::get("/api/v1/system/processes")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/processes returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn send_process_signal(
        &self,
        pid: u32,
        signal: cybou_protocol::system::ProcessSignal,
    ) -> Result<cybou_web_contracts::ActionRecordProjection, ClientError> {
        let response = Request::post("/api/v1/system/processes/signal")
            .json(&cybou_web_contracts::ProcessSignalRequest { pid, signal })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/processes/signal returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn get_system_monitor(
        &self,
    ) -> Result<cybou_web_contracts::SystemMonitorProjection, ClientError> {
        let response = Request::get("/api/v1/system/monitor")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/monitor returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn get_system_logs(
        &self,
        query: &cybou_web_contracts::SystemLogsQueryRequest,
    ) -> Result<cybou_web_contracts::SystemLogsProjection, ClientError> {
        let mut url = "/api/v1/system/logs?".to_owned();
        if let Some(ref u) = query.unit {
            url.push_str(&format!("unit={u}&"));
        }
        if let Some(ref s) = query.severity {
            url.push_str(&format!("severity={s}&"));
        }
        if let Some(ref q) = query.search {
            url.push_str(&format!("search={q}&"));
        }
        if let Some(limit) = query.limit {
            url.push_str(&format!("limit={limit}&"));
        }
        let response = Request::get(&url)
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/logs returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn get_storage(&self) -> Result<cybou_web_contracts::StorageProjection, ClientError> {
        let response = Request::get("/api/v1/system/storage")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/storage returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn create_snapshot(
        &self,
        subvolume: &str,
        name: &str,
        readonly: bool,
    ) -> Result<cybou_protocol::system::SnapshotRecord, ClientError> {
        let response = Request::post("/api/v1/system/storage/snapshots")
            .json(&cybou_web_contracts::CreateSnapshotRequest {
                subvolume_path: subvolume.to_owned(),
                name: name.to_owned(),
                readonly,
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/storage/snapshots returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn restore_snapshot(&self, snapshot_id: &str) -> Result<String, ClientError> {
        let response = Request::post("/api/v1/system/storage/restore")
            .json(&cybou_web_contracts::RestoreSnapshotRequest {
                snapshot_id: snapshot_id.to_owned(),
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/storage/restore returned HTTP {}",
                response.status()
            )));
        }
        let outcome: serde_json::Value = response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        Ok(outcome["outcome"]
            .as_str()
            .unwrap_or("Snapshot restored")
            .to_owned())
    }

    async fn get_network(&self) -> Result<cybou_web_contracts::NetworkProjection, ClientError> {
        let response = Request::get("/api/v1/system/network")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/network returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn connect_network(
        &self,
        connection_id: &str,
        activate: bool,
    ) -> Result<String, ClientError> {
        let response = Request::post("/api/v1/system/network/connect")
            .json(&cybou_web_contracts::NetworkConnectRequest {
                connection_id: connection_id.to_owned(),
                activate,
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/network/connect returned HTTP {}",
                response.status()
            )));
        }
        let outcome: serde_json::Value = response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        Ok(outcome["outcome"]
            .as_str()
            .unwrap_or("Network updated")
            .to_owned())
    }

    async fn get_packages(&self) -> Result<cybou_web_contracts::PackagesProjection, ClientError> {
        let response = Request::get("/api/v1/system/packages")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/packages returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn execute_package_action(
        &self,
        name: &str,
        action: cybou_protocol::system::PackageActionKind,
    ) -> Result<String, ClientError> {
        let response = Request::post("/api/v1/system/packages/action")
            .json(&cybou_web_contracts::PackageActionRequest {
                name: name.to_owned(),
                action,
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/packages/action returned HTTP {}",
                response.status()
            )));
        }
        let outcome: serde_json::Value = response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        Ok(outcome["outcome"]
            .as_str()
            .unwrap_or("Package action executed")
            .to_owned())
    }

    async fn get_system_updates(
        &self,
    ) -> Result<cybou_web_contracts::SystemUpdatesProjection, ClientError> {
        let response = Request::get("/api/v1/system/updates")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/updates returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn apply_system_updates(
        &self,
        package_names: Option<Vec<String>>,
    ) -> Result<String, ClientError> {
        let response = Request::post("/api/v1/system/updates/apply")
            .json(&cybou_web_contracts::ApplyUpdatesRequest { package_names })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/updates/apply returned HTTP {}",
                response.status()
            )));
        }
        let outcome: serde_json::Value = response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        Ok(outcome["outcome"]
            .as_str()
            .unwrap_or("System updates applied")
            .to_owned())
    }

    async fn get_users_settings(
        &self,
    ) -> Result<cybou_web_contracts::UsersSettingsProjection, ClientError> {
        let response = Request::get("/api/v1/system/users")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/users returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn create_user(
        &self,
        username: &str,
        full_name: &str,
        is_admin: bool,
    ) -> Result<cybou_protocol::system::UserAccountRecord, ClientError> {
        let response = Request::post("/api/v1/system/users")
            .json(&cybou_web_contracts::CreateUserRequest {
                username: username.to_owned(),
                full_name: full_name.to_owned(),
                is_admin,
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/users returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn add_ssh_key(
        &self,
        name: &str,
        public_key: &str,
    ) -> Result<cybou_protocol::system::SshKeyRecord, ClientError> {
        let response = Request::post("/api/v1/system/users/ssh-keys")
            .json(&cybou_web_contracts::AddSshKeyRequest {
                name: name.to_owned(),
                public_key: public_key.to_owned(),
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/users/ssh-keys returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn delete_ssh_key(&self, key_id: &str) -> Result<String, ClientError> {
        let response = Request::post("/api/v1/system/users/ssh-keys/delete")
            .json(&cybou_web_contracts::DeleteSshKeyRequest {
                key_id: key_id.to_owned(),
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/users/ssh-keys/delete returned HTTP {}",
                response.status()
            )));
        }
        let outcome: serde_json::Value = response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        Ok(outcome["outcome"]
            .as_str()
            .unwrap_or("SSH key deleted")
            .to_owned())
    }

    async fn get_security_settings(
        &self,
    ) -> Result<cybou_web_contracts::SecuritySettingsProjection, ClientError> {
        let response = Request::get("/api/v1/system/security")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/security returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn update_security_policy(
        &self,
        req: cybou_web_contracts::UpdateSecurityPolicyRequest,
    ) -> Result<cybou_protocol::system::SecurityPolicyRecord, ClientError> {
        let response = Request::post("/api/v1/system/security/policy")
            .json(&req)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/security/policy returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn get_backup_settings(
        &self,
    ) -> Result<cybou_web_contracts::BackupSettingsProjection, ClientError> {
        let response = Request::get("/api/v1/system/backup")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/backup returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn trigger_backup(
        &self,
        name: Option<String>,
    ) -> Result<cybou_protocol::system::BackupArchiveRecord, ClientError> {
        let response = Request::post("/api/v1/system/backup/trigger")
            .json(&cybou_web_contracts::TriggerBackupRequest { name })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/backup/trigger returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn restore_archive(
        &self,
        archive_id: &str,
        target_path: Option<String>,
    ) -> Result<String, ClientError> {
        let response = Request::post("/api/v1/system/backup/restore")
            .json(&cybou_web_contracts::RestoreArchiveRequest {
                archive_id: archive_id.to_owned(),
                target_path,
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/backup/restore returned HTTP {}",
                response.status()
            )));
        }
        let outcome: serde_json::Value = response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        Ok(outcome["outcome"]
            .as_str()
            .unwrap_or("Archive restored")
            .to_owned())
    }

    async fn update_backup_schedule(
        &self,
        req: cybou_web_contracts::UpdateBackupScheduleRequest,
    ) -> Result<cybou_protocol::system::BackupScheduleRecord, ClientError> {
        let response = Request::post("/api/v1/system/backup/schedule")
            .json(&req)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/system/backup/schedule returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn get_mail(
        &self,
        account_id: Option<String>,
        folder: Option<cybou_protocol::personal::MailFolderKind>,
    ) -> Result<cybou_web_contracts::MailProjection, ClientError> {
        let mut url = "/api/v1/personal/mail".to_owned();
        let mut query_params = Vec::new();
        if let Some(acc) = account_id {
            query_params.push(format!("accountId={acc}"));
        }
        if let Some(f) = folder {
            let f_str = match f {
                cybou_protocol::personal::MailFolderKind::Inbox => "inbox",
                cybou_protocol::personal::MailFolderKind::Sent => "sent",
                cybou_protocol::personal::MailFolderKind::Drafts => "drafts",
                cybou_protocol::personal::MailFolderKind::Archive => "archive",
                cybou_protocol::personal::MailFolderKind::Trash => "trash",
            };
            query_params.push(format!("folder={f_str}"));
        }
        if !query_params.is_empty() {
            url = format!("{url}?{}", query_params.join("&"));
        }

        let response = Request::get(&url)
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/personal/mail returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn send_mail(
        &self,
        req: cybou_web_contracts::SendMailRequest,
    ) -> Result<cybou_protocol::personal::MailMessageRecord, ClientError> {
        let response = Request::post("/api/v1/personal/mail/send")
            .json(&req)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/personal/mail/send returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn get_calendar(&self) -> Result<cybou_web_contracts::CalendarProjection, ClientError> {
        let response = Request::get("/api/v1/personal/calendar")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/personal/calendar returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn create_calendar_event(
        &self,
        req: cybou_web_contracts::CreateCalendarEventRequest,
    ) -> Result<cybou_protocol::personal::CalendarEventRecord, ClientError> {
        let response = Request::post("/api/v1/personal/calendar/events")
            .json(&req)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/personal/calendar/events returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn get_notes(&self) -> Result<cybou_web_contracts::NotesProjection, ClientError> {
        let response = Request::get("/api/v1/personal/notes")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/personal/notes returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn create_note(
        &self,
        req: cybou_web_contracts::CreateNoteRequest,
    ) -> Result<cybou_protocol::personal::NoteRecord, ClientError> {
        let response = Request::post("/api/v1/personal/notes")
            .json(&req)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/personal/notes returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn update_note(
        &self,
        req: cybou_web_contracts::UpdateNoteRequest,
    ) -> Result<cybou_protocol::personal::NoteRecord, ClientError> {
        let response = Request::post("/api/v1/personal/notes/update")
            .json(&req)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/personal/notes/update returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn get_contacts(&self) -> Result<cybou_web_contracts::ContactsProjection, ClientError> {
        let response = Request::get("/api/v1/personal/contacts")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/personal/contacts returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn create_contact(
        &self,
        req: cybou_web_contracts::CreateContactRequest,
    ) -> Result<cybou_protocol::personal::ContactRecord, ClientError> {
        let response = Request::post("/api/v1/personal/contacts")
            .json(&req)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/personal/contacts returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn get_cognitive_graph(
        &self,
        focus: Option<String>,
    ) -> Result<cybou_web_contracts::CognitiveGraphProjection, ClientError> {
        let mut url = "/api/v1/cognitive/graph".to_owned();
        if let Some(f) = focus {
            url = format!("{url}?focus={f}");
        }
        let response = Request::get(&url)
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/cognitive/graph returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn query_cognitive_graph(
        &self,
        req: cybou_web_contracts::CognitiveQueryRequest,
    ) -> Result<cybou_web_contracts::CognitiveGraphProjection, ClientError> {
        let response = Request::post("/api/v1/cognitive/query")
            .json(&req)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/cognitive/query returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn get_event_journal(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<cybou_web_contracts::EventJournalProjection, ClientError> {
        let mut url = "/api/v1/cognitive/journal".to_owned();
        let mut params = Vec::new();
        if let Some(lim) = limit {
            params.push(format!("limit={lim}"));
        }
        if let Some(off) = offset {
            params.push(format!("offset={off}"));
        }
        if !params.is_empty() {
            url = format!("{url}?{}", params.join("&"));
        }

        let response = Request::get(&url)
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/cognitive/journal returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn interpret_meaning(
        &self,
        req: &cybou_web_contracts::MeaningInterpretRequest,
    ) -> Result<cybou_web_contracts::MeaningInterpretProjection, ClientError> {
        let response = Request::post("/api/v1/meaning/interpret")
            .json(req)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/meaning/interpret returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn get_dialogue_memory(
        &self,
    ) -> Result<cybou_web_contracts::DialogueMemoryProjection, ClientError> {
        let response = Request::get("/api/v1/meaning/dialogue")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/meaning/dialogue returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn get_learning_candidates(
        &self,
        layer: Option<String>,
    ) -> Result<cybou_web_contracts::LearningCandidatesProjection, ClientError> {
        let url = if let Some(l) = layer {
            format!("/api/v1/learning/candidates?layer={l}")
        } else {
            "/api/v1/learning/candidates".to_string()
        };
        let response = Request::get(&url)
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "{url} returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn propose_learning_candidate(
        &self,
        req: &cybou_web_contracts::ProposeLearningCandidateRequest,
    ) -> Result<cybou_protocol::learning::LearningCandidate, ClientError> {
        let response = Request::post("/api/v1/learning/candidates")
            .json(req)
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/learning/candidates returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn evaluate_learning_candidate(
        &self,
        candidate_id: uuid::Uuid,
    ) -> Result<cybou_web_contracts::CandidateEvaluationProjection, ClientError> {
        let url = format!("/api/v1/learning/candidates/{candidate_id}/evaluate");
        let response = Request::post(&url)
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "{url} returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn get_learned_artifacts(
        &self,
    ) -> Result<cybou_web_contracts::LearnedArtifactsProjection, ClientError> {
        let response = Request::get("/api/v1/learning/artifacts")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/learning/artifacts returned HTTP {}",
                response.status()
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))
    }

    async fn revoke_learned_artifact(
        &self,
        artifact_id: uuid::Uuid,
        reason: &str,
    ) -> Result<(), ClientError> {
        let url = format!("/api/v1/learning/artifacts/{artifact_id}/revoke");
        let response = Request::post(&url)
            .json(&cybou_web_contracts::RevokeArtifactRequest {
                artifact_id,
                reason: reason.to_string(),
            })
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "{url} returned HTTP {}",
                response.status()
            )));
        }
        Ok(())
    }

    async fn get_governance_scopes(
        &self,
    ) -> Result<cybou_web_contracts::GovernanceScopesProjection, ClientError> {
        let response = Request::get("/api/v1/governance/scopes")
            .send()
            .await
            .map_err(|error| ClientError::GatewayRequest(error.to_string()))?;
        if !response.ok() {
            return Err(ClientError::GatewayRequest(format!(
                "/api/v1/governance/scopes returned HTTP {}",
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
