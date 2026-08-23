// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Runtime-independent client boundary for Living Canvas.

use async_trait::async_trait;
use cybou_web_contracts::{
    DirectoryListingProjection, DisclosureProjection, FileContentProjection, MindProjection,
    SessionProjection, ShellExecResponse, SnapshotProjection,
};
use thiserror::Error;

pub mod card;
pub mod deck;
pub mod instant;
pub mod layout;

#[cfg(target_arch = "wasm32")]
pub mod components;
#[cfg(target_arch = "wasm32")]
pub mod interaction;
#[cfg(target_arch = "wasm32")]
pub mod state;
#[cfg(target_arch = "wasm32")]
pub mod tool_state;

#[cfg(target_arch = "wasm32")]
mod gateway_client;
#[cfg(all(test, target_arch = "wasm32"))]
mod interaction_gate;
pub use card::{CardGeometry, CardId, CardInstance, CardKind, CardPresentation, CardSpec};
pub use deck::DeckInstance;
#[cfg(target_arch = "wasm32")]
pub use gateway_client::GatewayMindClient;
pub use instant::{instant_label, time_label};
pub use layout::{
    ArrangementMode, DesktopItem, DesktopItemId, DesktopLayout, DesktopViewMode, LayoutHistory,
    MINIMAP_HEIGHT, MINIMAP_PADDING, MINIMAP_WIDTH, MinimapProjection, PlacementResolver, Rect,
    SnapGuide, SnapResult, UsableViewport, pan_centring, selected_rect, selected_z,
    visible_desktop_rect,
};

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

    /// Return what this reader was supplied, and what was kept from them.
    ///
    /// Read after the projection it describes, never before: a delivery is recorded when it
    /// happens, so asking first would report the previous one as if it were this one.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the gateway cannot produce the projection.
    async fn disclosure(&self) -> Result<DisclosureProjection, ClientError>;

    /// Return what this host currently makes of itself.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the gateway cannot produce the projection.
    async fn insight(&self) -> Result<cybou_web_contracts::InsightProjection, ClientError>;

    /// Execute a bounded Shell capability inside the Body host sandbox.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] on gateway transport failure or if forbidden in public preview.
    /// List one directory inside the sandbox, as structure rather than as terminal output.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the path names nothing readable, or the gateway refuses.
    async fn list_directory(&self, path: &str) -> Result<DirectoryListingProjection, ClientError>;

    /// Read one text file inside the sandbox.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the path names nothing readable, or the gateway refuses.
    async fn read_text_file(&self, path: &str) -> Result<FileContentProjection, ClientError>;

    /// End one of the caller's shells, because the card standing in it was closed.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the gateway refuses or cannot be reached.
    async fn close_shell(&self, instance: u32) -> Result<(), ClientError>;

    /// Execute a bounded Shell capability in one of the caller's shells.
    ///
    /// `instance` names which Shell card the command came from. Two cards are two places a person
    /// is standing, and passing a constant here would make them one.
    async fn execute_shell(
        &self,
        command: &str,
        instance: u32,
    ) -> Result<ShellExecResponse, ClientError>;
}

/// Deterministic client for component, visual, and state-vocabulary tests.
#[derive(Clone, Debug)]
pub struct MockMindClient {
    session: SessionProjection,
    snapshot: SnapshotProjection,
    mind: Option<MindProjection>,
    disclosure: Option<DisclosureProjection>,
    insight: Option<cybou_web_contracts::InsightProjection>,
}

impl MockMindClient {
    /// Construct a mock from explicit typed projections.
    #[must_use]
    pub const fn new(session: SessionProjection, snapshot: SnapshotProjection) -> Self {
        Self {
            session,
            snapshot,
            mind: None,
            disclosure: None,
            insight: None,
        }
    }

    /// Attach an owner projection to a mock that would otherwise report none.
    #[must_use]
    pub fn with_mind(mut self, mind: MindProjection) -> Self {
        self.mind = Some(mind);
        self
    }

    /// Attach a disclosure record to a mock that would otherwise report none.
    ///
    /// A mock without one reports that nothing has been supplied, which is the correct answer for
    /// a client that has never read a projection.
    #[must_use]
    pub fn with_disclosure(mut self, disclosure: DisclosureProjection) -> Self {
        self.disclosure = Some(disclosure);
        self
    }

    /// Attach a system insight to a mock that would otherwise report none.
    ///
    /// A mock without one reports that telemetry did not answer, which is the correct answer for a
    /// client with nothing behind it — and not the same as a host with nothing to report.
    #[must_use]
    pub fn with_insight(mut self, insight: cybou_web_contracts::InsightProjection) -> Self {
        self.insight = Some(insight);
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

    async fn insight(&self) -> Result<cybou_web_contracts::InsightProjection, ClientError> {
        // Not read, rather than nothing to report. A mock that answered "all clear" would put an
        // all-clear on every test surface that never configured one.
        Ok(self
            .insight
            .clone()
            .unwrap_or_else(|| cybou_web_contracts::InsightProjection {
                schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
                knowledge: cybou_protocol::KnowledgeState::Unknown,
                watched_enough: false,
                findings: Vec::new(),
                unobserved: Vec::new(),
                said: String::new(),
            }))
    }

    async fn disclosure(&self) -> Result<DisclosureProjection, ClientError> {
        // A mock that was given none reports a delivery that has not happened, rather than
        // inventing an empty one — the two are different facts to the surface that shows them.
        Ok(self
            .disclosure
            .clone()
            .unwrap_or_else(|| DisclosureProjection {
                schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
                consumer_id: self.session.consumer_id.clone(),
                external_boundary: true,
                retains: false,
                supplied: 0,
                accounted_for: 0,
                provenance_count: 0,
                items: Vec::new(),
                withheld: Vec::new(),
                subjects_visible: false,
                delivered: false,
            }))
    }

    async fn list_directory(&self, path: &str) -> Result<DirectoryListingProjection, ClientError> {
        // A mock holds no sandbox. Answering with an empty directory would be the failure the
        // typed routes exist to remove: nothing there, and nothing saying why.
        Err(ClientError::GatewayRequest(format!(
            "mock client holds no sandbox to list {path} in"
        )))
    }

    async fn read_text_file(&self, path: &str) -> Result<FileContentProjection, ClientError> {
        Err(ClientError::GatewayRequest(format!(
            "mock client holds no sandbox to read {path} from"
        )))
    }

    async fn close_shell(&self, _instance: u32) -> Result<(), ClientError> {
        Ok(())
    }

    async fn execute_shell(
        &self,
        command: &str,
        _instance: u32,
    ) -> Result<ShellExecResponse, ClientError> {
        Ok(ShellExecResponse {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            exit_code: 0,
            stdout: format!("mock shell output for: {command}\n"),
            stderr: String::new(),
            cwd: "/".to_owned(),
        })
    }
}

// Native only: the browser gate in `interaction_gate` covers what lives on wasm32, and this
// module's async tests need a tokio that cannot build for that target.
#[cfg(all(test, not(target_arch = "wasm32")))]
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
