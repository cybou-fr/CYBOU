// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Browser implementation of the typed Mind boundary.

use async_trait::async_trait;
use cybou_web_contracts::{MindProjection, SessionProjection, SnapshotProjection};
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
}
