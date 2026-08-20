// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Runtime-independent client boundary for Living Canvas.

use async_trait::async_trait;
use cybou_web_contracts::{
    MindProjection, SessionProjection, ShellExecResponse, SnapshotProjection,
};
use thiserror::Error;

pub mod card;
pub mod layout;

#[cfg(target_arch = "wasm32")]
mod gateway_client;
pub use card::{CardGeometry, CardId, CardInstance, CardKind, CardPresentation, CardSpec};
#[cfg(target_arch = "wasm32")]
pub use gateway_client::GatewayMindClient;
pub use layout::{ArrangementMode, DesktopLayout};

/// Error returned by a typed Mind client operation.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ClientError {
    /// No accepted gateway session is available.
    #[error("gateway session is unavailable")]
    SessionUnavailable,
    /// The current fixture/client cannot supply the requested projection.
    #[error("projection is unavailable: {0}")]
    ProjectionUnavailable(String),
    /// A deterministic fixture violates the typed web contract.
    #[error("invalid deterministic fixture: {0}")]
    InvalidFixture(String),
    /// The same-origin gateway request or typed response failed.
    #[error("gateway request failed: {0}")]
    GatewayRequest(String),
}

/// Only data boundary used by the frontend. Browser code never receives D-Bus or native handles.
#[async_trait(?Send)]
pub trait MindClient {
    /// Return the server-established trust context.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when no accepted gateway session can be established.
    async fn session(&self) -> Result<SessionProjection, ClientError>;

    /// Return one atomic projection snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the projection cannot be obtained as a typed atomic value.
    async fn snapshot(&self) -> Result<SnapshotProjection, ClientError>;

    /// Return what the owners behind Mind currently hold.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the gateway cannot produce the projection.
    async fn mind(&self) -> Result<MindProjection, ClientError>;

    /// Execute a bounded Shell capability inside the Body host sandbox.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] on gateway transport failure or if forbidden in public preview.
    async fn execute_shell(&self, command: &str) -> Result<ShellExecResponse, ClientError>;
}

/// Deterministic client for component, visual, and state-vocabulary tests.
#[derive(Clone, Debug)]
pub struct MockMindClient {
    session: SessionProjection,
    snapshot: SnapshotProjection,
    mind: Option<MindProjection>,
}

impl MockMindClient {
    /// Construct a mock from explicit typed projections.
    #[must_use]
    pub const fn new(session: SessionProjection, snapshot: SnapshotProjection) -> Self {
        Self {
            session,
            snapshot,
            mind: None,
        }
    }

    /// Attach an owner projection to a mock that would otherwise report none.
    #[must_use]
    pub fn with_mind(mut self, mind: MindProjection) -> Self {
        self.mind = Some(mind);
        self
    }

    /// Load the repository's nominal W0 fixture pair.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::InvalidFixture`] if either checked-in fixture drifts from the typed
    /// web contract.
    pub fn nominal_fixture() -> Result<Self, ClientError> {
        let session =
            serde_json::from_str(include_str!("../../../fixtures/web/v1/session-local.json"))
                .map_err(|error| ClientError::InvalidFixture(error.to_string()))?;
        let snapshot = serde_json::from_str(include_str!(
            "../../../fixtures/web/v1/snapshot-nominal.json"
        ))
        .map_err(|error| ClientError::InvalidFixture(error.to_string()))?;
        let mind = serde_json::from_str(include_str!("../../../fixtures/web/v1/mind-nominal.json"))
            .map_err(|error| ClientError::InvalidFixture(error.to_string()))?;
        Ok(Self::new(session, snapshot).with_mind(mind))
    }
}

#[async_trait(?Send)]
impl MindClient for MockMindClient {
    async fn session(&self) -> Result<SessionProjection, ClientError> {
        Ok(self.session.clone())
    }

    async fn snapshot(&self) -> Result<SnapshotProjection, ClientError> {
        Ok(self.snapshot.clone())
    }

    async fn mind(&self) -> Result<MindProjection, ClientError> {
        self.mind.clone().ok_or_else(|| {
            ClientError::GatewayRequest("mock client holds no owner projection".into())
        })
    }

    async fn execute_shell(&self, command: &str) -> Result<ShellExecResponse, ClientError> {
        Ok(ShellExecResponse {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            exit_code: 0,
            stdout: format!("mock shell output for: {command}\n"),
            stderr: String::new(),
            cwd: "/".to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use cybou_protocol::{CapabilityState, KnowledgeState, SchemaVersion};
    use cybou_web_contracts::{
        CapabilityProjection, Freshness, SessionMode, SessionProjection, SnapshotProjection,
    };
    use uuid::Uuid;

    use super::{MindClient, MockMindClient};

    #[tokio::test]
    async fn mock_client_preserves_server_established_mode_and_unknown_state() {
        let client = MockMindClient::new(
            SessionProjection {
                schema_version: SchemaVersion(1),
                session_id: Uuid::nil(),
                mode: SessionMode::RemoteBrowser,
                consumer_id: "fixture:remote".into(),
                expires_at: "2026-08-18T12:30:00Z".into(),
            },
            SnapshotProjection {
                schema_version: SchemaVersion(1),
                projection_version: 1,
                cursor: "fixture:1".into(),
                observed_at: "2026-08-18T12:00:00Z".into(),
                freshness: Freshness::Unknown,
                knowledge: KnowledgeState::Unknown,
                capabilities: vec![CapabilityProjection {
                    id: "mind.context.read".into(),
                    state: CapabilityState::Unknown,
                    knowledge: KnowledgeState::Unknown,
                    freshness: Freshness::Unknown,
                    reason: Some("bounded request timed out".into()),
                }],
            },
        );

        assert_eq!(
            client.session().await.expect("fixture session").mode,
            SessionMode::RemoteBrowser
        );
        assert_eq!(
            client
                .snapshot()
                .await
                .expect("fixture snapshot")
                .capabilities[0]
                .state,
            CapabilityState::Unknown
        );
    }

    #[tokio::test]
    async fn nominal_repository_fixtures_are_usable_by_the_frontend_boundary() {
        let client = MockMindClient::nominal_fixture().expect("valid repository fixtures");
        assert_eq!(
            client.session().await.expect("fixture session").mode,
            SessionMode::LocalDesktop
        );
        assert_eq!(
            client
                .snapshot()
                .await
                .expect("fixture snapshot")
                .projection_version,
            42
        );
    }
}
