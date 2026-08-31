// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Runtime-independent client boundary for Living Canvas.

#![allow(missing_docs)]
// Every Leptos `#[component]` is a function returning `impl IntoView`, and `pedantic` asks all 120
// of them to be `#[must_use]`. They are: the `view!` macro consumes what they return, and there is
// no call site in this crate or any other that could discard one. Silenced with a reason rather
// than answered one attribute at a time, because 120 warnings nobody can act on are what stops the
// other 190 from being read — and an earlier attempt to answer them mechanically wrote the
// attribute twice on every component and failed the build on `duplicated_attributes`.
#![allow(
    clippy::must_use_candidate,
    reason = "a Leptos component's return value is consumed by the view macro that calls it"
)]

use async_trait::async_trait;
use cybou_web_contracts::{
    DirectoryListingProjection, DisclosureProjection, FileContentProjection, FileWriteProjection,
    FileWriteRequest, HostDirectoryCreateRequest, HostDirectoryListingProjection,
    HostFileCreateRequest, HostFileWriteRequest, HostPathCopyRequest, HostPathDeleteRequest,
    HostPathRenameRequest, MindProjection, SessionProjection, ShellExecResponse,
    SnapshotProjection,
};
use thiserror::Error;

pub mod ansi;
pub mod card;
pub mod deck;
pub mod heading;
pub mod instant;
pub mod layout;
pub mod markdown;
pub mod refresh;
pub mod terminal;
pub mod workspace_sync;

#[cfg(target_arch = "wasm32")]
pub mod components;
#[cfg(target_arch = "wasm32")]
pub mod interaction;
#[cfg(target_arch = "wasm32")]
pub mod state;
pub mod text_diff;
#[cfg(target_arch = "wasm32")]
pub mod tool_state;

#[cfg(target_arch = "wasm32")]
mod gateway_client;
#[cfg(all(test, target_arch = "wasm32"))]
mod interaction_gate;
pub use card::{
    CardGeometry, CardId, CardInstance, CardKind, CardPresentation, CardSpec, PanelRepresentation,
};
pub use deck::DeckInstance;
#[cfg(target_arch = "wasm32")]
pub use gateway_client::GatewayMindClient;
pub use instant::{instant_label, time_label};
pub use layout::{
    ArrangementMode, CameraHistory, CameraState, CanvasAnchor, DesktopCluster, DesktopItem,
    DesktopItemId, DesktopLayout, DesktopViewMode, LayoutHistory, MINIMAP_HEIGHT, MINIMAP_PADDING,
    MINIMAP_WIDTH, MinimapProjection, PlacementResolver, Rect, SnapGuide, SnapResult,
    UsableViewport, pan_centring, selected_rect, selected_z, visible_desktop_rect,
};
#[cfg(target_arch = "wasm32")]
pub use layout::{apply_camera_back, apply_camera_fly_to, apply_camera_forward, camera_center};

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
    /// A conditional file write was based on content that is no longer current.
    #[error("file changed since this editor read it")]
    FileChangedSinceRead,
    /// Exclusive creation refused because the requested file already exists.
    #[error("file already exists")]
    FileAlreadyExists,
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

    /// Every agent session this host is holding, as the runtime that holds them describes them.
    ///
    /// Carried whole rather than reshaped into a projection of its own. The owner's answer is the
    /// only one that is right about what is running, and a second shape assembled here would
    /// disagree with it the moment a session started between a listing and a reading.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::GatewayRequest`] when the surface cannot be asked. That is not the
    /// same as no sessions, and the two must not collapse into one another.
    async fn agents(&self) -> Result<Vec<cybou_protocol::agent::SessionView>, ClientError>;

    /// Ask the agent runtime owner to launch one profile-bounded session.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::GatewayRequest`] when the caller is not entitled to launch, the
    /// profile refuses the selection, host capacity is exhausted, or Agent1 is unavailable.
    async fn launch_agent(
        &self,
        request: &cybou_protocol::agent::LaunchRequest,
    ) -> Result<cybou_protocol::agent::SessionView, ClientError>;

    /// Return operator-approved profile offers and launch readiness.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::GatewayRequest`] when Agent1 is unavailable.
    async fn agent_offers(&self)
    -> Result<cybou_protocol::agent::AgentOffersResponse, ClientError>;

    /// Return action records matching the optional cause query.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::GatewayRequest`] when Action1 is unavailable.
    async fn actions(
        &self,
        cause_id: Option<uuid::Uuid>,
    ) -> Result<Vec<cybou_web_contracts::ActionRecordProjection>, ClientError>;

    /// Answer a proposal that was waiting on a person.
    ///
    /// The request names the proposal and the decision that was on screen when the person
    /// answered. It names no operation and no target: those are on the proposal Action1 already
    /// holds, and a confirmation that could carry them would be a request rather than an answer.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the answer was not accepted.
    async fn confirm_action(
        &self,
        request: &cybou_web_contracts::ConfirmActionRequest,
    ) -> Result<cybou_web_contracts::ActionRecordProjection, ClientError>;

    /// Ask the agent runtime owner to end one session and confirm its teardown.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::GatewayRequest`] when the caller is not entitled to stop it, the
    /// teardown cannot be confirmed, or Agent1 is unavailable.
    async fn stop_agent(&self, capsule_id: uuid::Uuid) -> Result<(), ClientError>;

    /// Perform a lifecycle or boundary control action (Freeze, Resume, Quarantine, Stop) on a live capsule.
    async fn control_agent(
        &self,
        capsule_id: uuid::Uuid,
        action: cybou_protocol::agent::CapsuleAction,
    ) -> Result<(), ClientError>;

    /// Retrieve live fine-grained telemetry for an active capsule session.
    async fn agent_telemetry(
        &self,
        capsule_id: uuid::Uuid,
    ) -> Result<cybou_web_contracts::CapsuleTelemetryProjection, ClientError>;

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

    /// Conditionally replace a file previously read through the bounded gateway.
    async fn write_text_file(
        &self,
        request: &FileWriteRequest,
    ) -> Result<FileWriteProjection, ClientError>;

    /// Exclusively create a new UTF-8 file inside the sandbox.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when creation is refused or fails.
    async fn create_file(
        &self,
        request: &cybou_web_contracts::FileCreateRequest,
    ) -> Result<FileWriteProjection, ClientError>;

    /// Place one file into the sandbox, whatever its bytes are.
    ///
    /// Separate from [`Self::create_file`], which carries text. A file a person drops onto the
    /// desktop is not necessarily text, and a desktop that could only accept what it could also
    /// display would not be accepting files.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the upload is refused or fails.
    async fn upload_file(
        &self,
        request: &cybou_web_contracts::FileUploadRequest,
    ) -> Result<cybou_web_contracts::FileUploadProjection, ClientError>;

    /// Fetch one file from the sandbox as bytes.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the download is refused or fails.
    async fn download_file(&self, path: &str) -> Result<Vec<u8>, ClientError>;

    /// List a directory in the user's home authority domain.
    async fn host_list_directory(
        &self,
        path: &str,
    ) -> Result<HostDirectoryListingProjection, ClientError>;

    /// Read a file from the user's home authority domain.
    async fn host_read_file(&self, path: &str) -> Result<FileContentProjection, ClientError>;

    /// Write a file to the user's home authority domain.
    async fn host_write_file(
        &self,
        request: &HostFileWriteRequest,
    ) -> Result<FileWriteProjection, ClientError>;

    /// Create a new file in the user's home authority domain.
    async fn host_create_file(
        &self,
        request: &HostFileCreateRequest,
    ) -> Result<FileWriteProjection, ClientError>;

    /// Create a directory in the user's home authority domain.
    async fn host_create_directory(
        &self,
        request: &HostDirectoryCreateRequest,
    ) -> Result<(), ClientError>;

    /// Rename or move a path in the user's home authority domain.
    async fn host_rename_path(&self, request: &HostPathRenameRequest) -> Result<(), ClientError>;

    /// Delete a path in the user's home authority domain.
    async fn host_delete_path(&self, request: &HostPathDeleteRequest) -> Result<(), ClientError>;

    /// Copy a path in the user's home authority domain.
    async fn host_copy_path(&self, request: &HostPathCopyRequest) -> Result<(), ClientError>;

    /// List active and historical server operations.
    async fn list_operations(
        &self,
    ) -> Result<cybou_web_contracts::OperationsListProjection, ClientError>;

    /// Get execution logs for a specific operation.
    async fn get_operation_logs(
        &self,
        id: uuid::Uuid,
    ) -> Result<cybou_web_contracts::OperationLogsProjection, ClientError>;

    /// Cancel a running server operation.
    async fn cancel_operation(
        &self,
        id: uuid::Uuid,
        reason: Option<String>,
    ) -> Result<(), ClientError>;

    /// List desktop notifications.
    async fn list_notifications(
        &self,
    ) -> Result<cybou_web_contracts::NotificationsListProjection, ClientError>;

    /// Dismiss one or all notifications.
    async fn dismiss_notifications(
        &self,
        id: Option<uuid::Uuid>,
        dismiss_all: bool,
    ) -> Result<(), ClientError>;

    /// Trigger an interactive notification action.
    async fn execute_notification_action(
        &self,
        id: uuid::Uuid,
        action_id: &str,
    ) -> Result<String, ClientError>;

    /// List system services and daemons.
    async fn list_services(
        &self,
    ) -> Result<cybou_web_contracts::ServicesListProjection, ClientError>;

    /// Execute a state action on a system service.
    /// Ask for a service action, and receive what Action1 decided about it.
    ///
    /// A lifecycle record rather than a sentence: a refusal is a record too, and the reason it
    /// carries is the one the boundary gave rather than one this client composed.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError`] when the request could not be put or the answer not read.
    async fn execute_service_action(
        &self,
        name: &str,
        action: cybou_protocol::system::ServiceAction,
    ) -> Result<cybou_web_contracts::ActionRecordProjection, ClientError>;

    /// List active operating system processes.
    async fn list_processes(
        &self,
    ) -> Result<cybou_web_contracts::ProcessesListProjection, ClientError>;

    /// Ask the action boundary to deliver a signal to an operating system process.
    ///
    /// Returns the record, not a sentence: a refusal is a record too, and a caller that received a
    /// string could not tell one from a success without reading the English in it.
    async fn send_process_signal(
        &self,
        pid: u32,
        signal: cybou_protocol::system::ProcessSignal,
    ) -> Result<cybou_web_contracts::ActionRecordProjection, ClientError>;

    /// Get current hardware telemetry & resource monitor metrics.
    async fn get_system_monitor(
        &self,
    ) -> Result<cybou_web_contracts::SystemMonitorProjection, ClientError>;

    /// Query system log entries.
    async fn get_system_logs(
        &self,
        query: &cybou_web_contracts::SystemLogsQueryRequest,
    ) -> Result<cybou_web_contracts::SystemLogsProjection, ClientError>;

    /// List Btrfs subvolumes and snapshots.
    async fn get_storage(&self) -> Result<cybou_web_contracts::StorageProjection, ClientError>;

    /// Create a point-in-time filesystem snapshot.
    async fn create_snapshot(
        &self,
        subvolume: &str,
        name: &str,
        readonly: bool,
    ) -> Result<cybou_protocol::system::SnapshotRecord, ClientError>;

    /// Restore a filesystem snapshot.
    async fn restore_snapshot(&self, snapshot_id: &str) -> Result<String, ClientError>;

    /// List network interfaces and connections.
    async fn get_network(&self) -> Result<cybou_web_contracts::NetworkProjection, ClientError>;

    /// Connect or disconnect a network profile.
    async fn connect_network(
        &self,
        connection_id: &str,
        activate: bool,
    ) -> Result<String, ClientError>;

    /// List software packages.
    async fn get_packages(&self) -> Result<cybou_web_contracts::PackagesProjection, ClientError>;

    /// Execute a package operation (install/upgrade/remove).
    async fn execute_package_action(
        &self,
        name: &str,
        action: cybou_protocol::system::PackageActionKind,
    ) -> Result<String, ClientError>;

    /// Get system update status summary.
    async fn get_system_updates(
        &self,
    ) -> Result<cybou_web_contracts::SystemUpdatesProjection, ClientError>;

    /// Apply pending system updates.
    async fn apply_system_updates(
        &self,
        package_names: Option<Vec<String>>,
    ) -> Result<String, ClientError>;

    /// Get user accounts and authorized SSH keys.
    async fn get_users_settings(
        &self,
    ) -> Result<cybou_web_contracts::UsersSettingsProjection, ClientError>;

    /// Create a new local user account.
    async fn create_user(
        &self,
        username: &str,
        full_name: &str,
        is_admin: bool,
    ) -> Result<cybou_protocol::system::UserAccountRecord, ClientError>;

    /// Add an authorized SSH public key.
    async fn add_ssh_key(
        &self,
        name: &str,
        public_key: &str,
    ) -> Result<cybou_protocol::system::SshKeyRecord, ClientError>;

    /// Delete an authorized SSH public key.
    async fn delete_ssh_key(&self, key_id: &str) -> Result<String, ClientError>;

    /// Get security policy and audit log.
    async fn get_security_settings(
        &self,
    ) -> Result<cybou_web_contracts::SecuritySettingsProjection, ClientError>;

    /// Update security sandboxing policy.
    async fn update_security_policy(
        &self,
        req: cybou_web_contracts::UpdateSecurityPolicyRequest,
    ) -> Result<cybou_protocol::system::SecurityPolicyRecord, ClientError>;

    /// Get Borg/Btrfs backup repository settings and archives.
    async fn get_backup_settings(
        &self,
    ) -> Result<cybou_web_contracts::BackupSettingsProjection, ClientError>;

    /// Trigger an immediate backup snapshot.
    async fn trigger_backup(
        &self,
        name: Option<String>,
    ) -> Result<cybou_protocol::system::BackupArchiveRecord, ClientError>;

    /// Restore a backup archive.
    async fn restore_archive(
        &self,
        archive_id: &str,
        target_path: Option<String>,
    ) -> Result<String, ClientError>;

    /// Update automated backup schedule.
    async fn update_backup_schedule(
        &self,
        req: cybou_web_contracts::UpdateBackupScheduleRequest,
    ) -> Result<cybou_protocol::system::BackupScheduleRecord, ClientError>;

    /// Get personal email messages and accounts.
    async fn get_mail(
        &self,
        account_id: Option<String>,
        folder: Option<cybou_protocol::personal::MailFolderKind>,
    ) -> Result<cybou_web_contracts::MailProjection, ClientError>;

    /// Compose and send a new email.
    async fn send_mail(
        &self,
        req: cybou_web_contracts::SendMailRequest,
    ) -> Result<cybou_protocol::personal::MailMessageRecord, ClientError>;

    /// Get personal calendar events.
    async fn get_calendar(&self) -> Result<cybou_web_contracts::CalendarProjection, ClientError>;

    /// Create a new calendar event.
    async fn create_calendar_event(
        &self,
        req: cybou_web_contracts::CreateCalendarEventRequest,
    ) -> Result<cybou_protocol::personal::CalendarEventRecord, ClientError>;

    /// Get personal notes.
    async fn get_notes(&self) -> Result<cybou_web_contracts::NotesProjection, ClientError>;

    /// Create a new personal note.
    async fn create_note(
        &self,
        req: cybou_web_contracts::CreateNoteRequest,
    ) -> Result<cybou_protocol::personal::NoteRecord, ClientError>;

    /// Update an existing personal note.
    async fn update_note(
        &self,
        req: cybou_web_contracts::UpdateNoteRequest,
    ) -> Result<cybou_protocol::personal::NoteRecord, ClientError>;

    /// Get address book contacts.
    async fn get_contacts(&self) -> Result<cybou_web_contracts::ContactsProjection, ClientError>;

    /// Create a new address book contact.
    async fn create_contact(
        &self,
        req: cybou_web_contracts::CreateContactRequest,
    ) -> Result<cybou_protocol::personal::ContactRecord, ClientError>;

    /// Retrieve the deep unified Cognitive Graph.
    async fn get_cognitive_graph(
        &self,
        focus: Option<String>,
    ) -> Result<cybou_web_contracts::CognitiveGraphProjection, ClientError>;

    /// Query subgraphs and causal relations in the Cognitive Graph.
    async fn query_cognitive_graph(
        &self,
        req: cybou_web_contracts::CognitiveQueryRequest,
    ) -> Result<cybou_web_contracts::CognitiveGraphProjection, ClientError>;

    /// Retrieve the canonical Event1 chronological journal.
    async fn get_event_journal(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<cybou_web_contracts::EventJournalProjection, ClientError>;

    /// Interpret a natural language query into a typed cognitive act and realize a qualified response.
    async fn interpret_meaning(
        &self,
        req: &cybou_web_contracts::MeaningInterpretRequest,
    ) -> Result<cybou_web_contracts::MeaningInterpretProjection, ClientError>;

    /// Retrieve active dialogue memory status and referents.
    async fn get_dialogue_memory(
        &self,
    ) -> Result<cybou_web_contracts::DialogueMemoryProjection, ClientError>;

    /// Retrieve active learning candidates.
    async fn get_learning_candidates(
        &self,
        layer: Option<String>,
    ) -> Result<cybou_web_contracts::LearningCandidatesProjection, ClientError>;

    /// Propose a new learning candidate.
    async fn propose_learning_candidate(
        &self,
        req: &cybou_web_contracts::ProposeLearningCandidateRequest,
    ) -> Result<cybou_protocol::learning::LearningCandidate, ClientError>;

    /// Evaluate a candidate against demonstrated episodic outcomes and promotion criteria.
    async fn evaluate_learning_candidate(
        &self,
        candidate_id: uuid::Uuid,
        req: Option<&cybou_web_contracts::EvaluateCandidateRequest>,
    ) -> Result<cybou_web_contracts::CandidateEvaluationProjection, ClientError>;

    /// Retrieve promoted durable artifacts and lineages.
    async fn get_learned_artifacts(
        &self,
    ) -> Result<cybou_web_contracts::LearnedArtifactsProjection, ClientError>;

    /// Revoke or deprecate a promoted artifact.
    async fn revoke_learned_artifact(
        &self,
        artifact_id: uuid::Uuid,
        reason: &str,
    ) -> Result<(), ClientError>;

    /// Retrieve active task scopes and capability grants.
    async fn get_governance_scopes(
        &self,
    ) -> Result<cybou_web_contracts::GovernanceScopesProjection, ClientError>;

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
    agents: Option<Vec<cybou_protocol::agent::SessionView>>,
    agent_offers: Option<cybou_protocol::agent::AgentOffersResponse>,
    actions: Option<Vec<cybou_web_contracts::ActionRecordProjection>>,
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
            agents: None,
            agent_offers: None,
            actions: None,
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

    /// Attach agent sessions to a mock that would otherwise refuse to be asked.
    ///
    /// A mock without them reports that the runtime could not be reached rather than that nothing
    /// is running, because a surface drawing "no agents" from a client that was never wired to one
    /// would be stating a fact about the host that nothing here established.
    #[must_use]
    pub fn with_agents(mut self, agents: Vec<cybou_protocol::agent::SessionView>) -> Self {
        self.agents = Some(agents);
        self
    }

    /// Attach agent offers to a mock client.
    #[must_use]
    pub fn with_agent_offers(mut self, offers: cybou_protocol::agent::AgentOffersResponse) -> Self {
        self.agent_offers = Some(offers);
        self
    }

    /// Attach action records to a mock client.
    #[must_use]
    pub fn with_actions(
        mut self,
        actions: Vec<cybou_web_contracts::ActionRecordProjection>,
    ) -> Self {
        self.actions = Some(actions);
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
        let offers = cybou_protocol::agent::AgentOffersResponse {
            profiles: vec![cybou_protocol::agent::OfferedProfileView {
                id: "sandboxed-autonomous".to_owned(),
                agents: vec!["opencode".to_owned()],
                workspace_roots: vec!["/srv/workspace".to_owned()],
                memory_mib: 4096,
                cpus: 2,
                tasks_max: 64,
                lifetime_seconds: 14400,
                hosts: vec!["github.com".to_owned(), "registry.npmjs.org".to_owned()],
                models: vec![
                    cybou_protocol::agent::OfferedModelView {
                        class: "Strong".to_owned(),
                        zero_cost: false,
                        spend_limit: Some(100),
                    },
                    cybou_protocol::agent::OfferedModelView {
                        class: "Fast".to_owned(),
                        zero_cost: true,
                        spend_limit: None,
                    },
                ],
                may_execute: true,
            }],
            profiles_state: "ready".to_owned(),
            capacity_state: "ready".to_owned(),
            provider_state: "ready".to_owned(),
            capacity_bounded: true,
            provider_connected: true,
        };
        Ok(Self::new(session, snapshot)
            .with_mind(mind)
            .with_agent_offers(offers))
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
                watched: Vec::new(),
                projections: Vec::new(),
                said: String::new(),
            }))
    }

    async fn agents(&self) -> Result<Vec<cybou_protocol::agent::SessionView>, ClientError> {
        self.agents
            .clone()
            .ok_or_else(|| ClientError::GatewayRequest("mock client holds no agent runtime".into()))
    }

    async fn launch_agent(
        &self,
        _request: &cybou_protocol::agent::LaunchRequest,
    ) -> Result<cybou_protocol::agent::SessionView, ClientError> {
        Err(ClientError::GatewayRequest(
            "mock client launches no agent sessions".into(),
        ))
    }

    async fn agent_offers(
        &self,
    ) -> Result<cybou_protocol::agent::AgentOffersResponse, ClientError> {
        Ok(self.agent_offers.clone().unwrap_or_default())
    }

    async fn actions(
        &self,
        _cause_id: Option<uuid::Uuid>,
    ) -> Result<Vec<cybou_web_contracts::ActionRecordProjection>, ClientError> {
        Ok(self.actions.clone().unwrap_or_default())
    }

    async fn stop_agent(&self, capsule_id: uuid::Uuid) -> Result<(), ClientError> {
        Err(ClientError::GatewayRequest(format!(
            "mock client stops no agent session {capsule_id}"
        )))
    }

    async fn control_agent(
        &self,
        _capsule_id: uuid::Uuid,
        _action: cybou_protocol::agent::CapsuleAction,
    ) -> Result<(), ClientError> {
        Ok(())
    }

    async fn agent_telemetry(
        &self,
        capsule_id: uuid::Uuid,
    ) -> Result<cybou_web_contracts::CapsuleTelemetryProjection, ClientError> {
        Ok(cybou_web_contracts::CapsuleTelemetryProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            telemetry: cybou_protocol::agent::CapsuleTelemetryRecord {
                capsule_id,
                standing: cybou_protocol::agent::Standing::Running,
                pids_count: 3,
                memory_used_mib: 64,
                memory_max_mib: 512,
                cpu_usage_pct: 2.5,
                egress_requests_count: 8,
                egress_denied_count: 0,
                files_modified_count: 2,
                tokens_in: 600,
                tokens_out: 180,
                active_tool: Some("read_file".to_string()),
                recent_activity: vec!["Capsule boundary verified".to_string()],
            },
        })
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
                history: Vec::new(),
                history_complete: false,
                history_covers_since: None,
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

    async fn write_text_file(
        &self,
        _request: &FileWriteRequest,
    ) -> Result<FileWriteProjection, ClientError> {
        Err(ClientError::ProjectionUnavailable(
            "mock file writes are unavailable".to_string(),
        ))
    }

    async fn confirm_action(
        &self,
        _request: &cybou_web_contracts::ConfirmActionRequest,
    ) -> Result<cybou_web_contracts::ActionRecordProjection, ClientError> {
        Err(ClientError::ProjectionUnavailable(
            "mock confirmations are unavailable".to_string(),
        ))
    }

    async fn create_file(
        &self,
        _request: &cybou_web_contracts::FileCreateRequest,
    ) -> Result<FileWriteProjection, ClientError> {
        Err(ClientError::ProjectionUnavailable(
            "mock file creation is unavailable".to_string(),
        ))
    }

    async fn upload_file(
        &self,
        _request: &cybou_web_contracts::FileUploadRequest,
    ) -> Result<cybou_web_contracts::FileUploadProjection, ClientError> {
        Err(ClientError::ProjectionUnavailable(
            "mock file uploads are unavailable".to_string(),
        ))
    }

    async fn download_file(&self, _path: &str) -> Result<Vec<u8>, ClientError> {
        Err(ClientError::ProjectionUnavailable(
            "mock file downloads are unavailable".to_string(),
        ))
    }

    async fn host_list_directory(
        &self,
        path: &str,
    ) -> Result<HostDirectoryListingProjection, ClientError> {
        Err(ClientError::GatewayRequest(format!(
            "mock client holds no host filesystem to list {path} in"
        )))
    }

    async fn host_read_file(&self, path: &str) -> Result<FileContentProjection, ClientError> {
        Err(ClientError::GatewayRequest(format!(
            "mock client holds no host filesystem to read {path} from"
        )))
    }

    async fn host_write_file(
        &self,
        _request: &HostFileWriteRequest,
    ) -> Result<FileWriteProjection, ClientError> {
        Err(ClientError::ProjectionUnavailable(
            "mock host file writes are unavailable".to_string(),
        ))
    }

    async fn host_create_file(
        &self,
        _request: &HostFileCreateRequest,
    ) -> Result<FileWriteProjection, ClientError> {
        Err(ClientError::ProjectionUnavailable(
            "mock host file creation is unavailable".to_string(),
        ))
    }

    async fn host_create_directory(
        &self,
        _request: &HostDirectoryCreateRequest,
    ) -> Result<(), ClientError> {
        Err(ClientError::ProjectionUnavailable(
            "mock host directory creation is unavailable".to_string(),
        ))
    }

    async fn host_rename_path(&self, _request: &HostPathRenameRequest) -> Result<(), ClientError> {
        Err(ClientError::ProjectionUnavailable(
            "mock host path rename is unavailable".to_string(),
        ))
    }

    async fn host_delete_path(&self, _request: &HostPathDeleteRequest) -> Result<(), ClientError> {
        Err(ClientError::ProjectionUnavailable(
            "mock host path delete is unavailable".to_string(),
        ))
    }

    async fn host_copy_path(&self, _request: &HostPathCopyRequest) -> Result<(), ClientError> {
        Err(ClientError::ProjectionUnavailable(
            "mock host path copy is unavailable".to_string(),
        ))
    }

    async fn list_operations(
        &self,
    ) -> Result<cybou_web_contracts::OperationsListProjection, ClientError> {
        Ok(cybou_web_contracts::OperationsListProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            active_count: 0,
            operations: Vec::new(),
        })
    }

    async fn get_operation_logs(
        &self,
        id: uuid::Uuid,
    ) -> Result<cybou_web_contracts::OperationLogsProjection, ClientError> {
        Ok(cybou_web_contracts::OperationLogsProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            operation_id: id,
            logs: Vec::new(),
        })
    }

    async fn cancel_operation(
        &self,
        _id: uuid::Uuid,
        _reason: Option<String>,
    ) -> Result<(), ClientError> {
        Ok(())
    }

    async fn list_notifications(
        &self,
    ) -> Result<cybou_web_contracts::NotificationsListProjection, ClientError> {
        Ok(cybou_web_contracts::NotificationsListProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            unread_count: 0,
            attention_count: 0,
            notifications: Vec::new(),
        })
    }

    async fn dismiss_notifications(
        &self,
        _id: Option<uuid::Uuid>,
        _dismiss_all: bool,
    ) -> Result<(), ClientError> {
        Ok(())
    }

    async fn execute_notification_action(
        &self,
        _id: uuid::Uuid,
        _action_id: &str,
    ) -> Result<String, ClientError> {
        Ok("mock action executed".to_owned())
    }

    async fn list_services(
        &self,
    ) -> Result<cybou_web_contracts::ServicesListProjection, ClientError> {
        Ok(cybou_web_contracts::ServicesListProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            active_count: 0,
            failed_count: 0,
            services: Vec::new(),
        })
    }

    async fn execute_service_action(
        &self,
        name: &str,
        _action: cybou_protocol::system::ServiceAction,
    ) -> Result<cybou_web_contracts::ActionRecordProjection, ClientError> {
        Err(ClientError::ProjectionUnavailable(format!(
            "mock service actions are unavailable, so {name} was not asked for"
        )))
    }

    async fn list_processes(
        &self,
    ) -> Result<cybou_web_contracts::ProcessesListProjection, ClientError> {
        Ok(cybou_web_contracts::ProcessesListProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            total_count: 0,
            total_cpu_percent: 0.0,
            total_memory_bytes: 0,
            processes: Vec::new(),
        })
    }

    async fn send_process_signal(
        &self,
        pid: u32,
        _signal: cybou_protocol::system::ProcessSignal,
    ) -> Result<cybou_web_contracts::ActionRecordProjection, ClientError> {
        // Refused rather than pretended. The mock used to answer "Mock sent signal to PID 42",
        // which is a claim that a process was signalled, made by something that cannot signal one.
        Err(ClientError::ProjectionUnavailable(format!(
            "mock process signals are unavailable, so pid {pid} was not asked about"
        )))
    }

    async fn get_system_monitor(
        &self,
    ) -> Result<cybou_web_contracts::SystemMonitorProjection, ClientError> {
        Ok(cybou_web_contracts::SystemMonitorProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            hostname: "mock-host".to_owned(),
            os_release: "Linux Mock".to_owned(),
            uptime_seconds: 1000,
            load_avg: [0.1, 0.1, 0.1],
            total_cpu_percent: 5.0,
            cores: Vec::new(),
            memory_total_bytes: 8_000_000_000,
            memory_used_bytes: 2_000_000_000,
            memory_free_bytes: 6_000_000_000,
            swap_total_bytes: 2_000_000_000,
            swap_used_bytes: 0,
            disk_partitions: Vec::new(),
            network_interfaces: Vec::new(),
        })
    }

    async fn get_system_logs(
        &self,
        _query: &cybou_web_contracts::SystemLogsQueryRequest,
    ) -> Result<cybou_web_contracts::SystemLogsProjection, ClientError> {
        Ok(cybou_web_contracts::SystemLogsProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            logs: Vec::new(),
            unavailable: None,
            system_journal_readable: true,
        })
    }

    async fn get_storage(&self) -> Result<cybou_web_contracts::StorageProjection, ClientError> {
        Ok(cybou_web_contracts::StorageProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            subvolumes: Vec::new(),
            snapshots: Vec::new(),
            total_space_bytes: 1_000_000_000_000,
            free_space_bytes: 500_000_000_000,
        })
    }

    async fn create_snapshot(
        &self,
        subvolume: &str,
        name: &str,
        readonly: bool,
    ) -> Result<cybou_protocol::system::SnapshotRecord, ClientError> {
        Ok(cybou_protocol::system::SnapshotRecord {
            id: "mock-snap-01".to_owned(),
            subvolume_path: subvolume.to_owned(),
            name: name.to_owned(),
            timestamp: "2026-08-28T22:00:00Z".to_owned(),
            size_bytes: 100_000_000,
            readonly,
        })
    }

    async fn restore_snapshot(&self, snapshot_id: &str) -> Result<String, ClientError> {
        Ok(format!("Mock restored snapshot {snapshot_id}"))
    }

    async fn get_network(&self) -> Result<cybou_web_contracts::NetworkProjection, ClientError> {
        Ok(cybou_web_contracts::NetworkProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            connections: Vec::new(),
        })
    }

    async fn connect_network(
        &self,
        connection_id: &str,
        activate: bool,
    ) -> Result<String, ClientError> {
        Ok(format!("Mock network {connection_id} activate={activate}"))
    }

    async fn get_packages(&self) -> Result<cybou_web_contracts::PackagesProjection, ClientError> {
        Ok(cybou_web_contracts::PackagesProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            installed_count: 0,
            upgradable_count: 0,
            packages: Vec::new(),
        })
    }

    async fn execute_package_action(
        &self,
        name: &str,
        action: cybou_protocol::system::PackageActionKind,
    ) -> Result<String, ClientError> {
        Ok(format!("Mock package action on {name}: {action:?}"))
    }

    async fn get_system_updates(
        &self,
    ) -> Result<cybou_web_contracts::SystemUpdatesProjection, ClientError> {
        Ok(cybou_web_contracts::SystemUpdatesProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            summary: cybou_protocol::system::SystemUpdatesSummary {
                pending_count: 0,
                security_updates_count: 0,
                kernel_update: false,
                reboot_required: false,
                total_download_bytes: 0,
                packages: Vec::new(),
            },
        })
    }

    async fn apply_system_updates(
        &self,
        _package_names: Option<Vec<String>>,
    ) -> Result<String, ClientError> {
        Ok("Mock applied system updates".to_owned())
    }

    async fn get_users_settings(
        &self,
    ) -> Result<cybou_web_contracts::UsersSettingsProjection, ClientError> {
        Ok(cybou_web_contracts::UsersSettingsProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            users: Vec::new(),
            ssh_keys: Vec::new(),
        })
    }

    async fn create_user(
        &self,
        username: &str,
        full_name: &str,
        is_admin: bool,
    ) -> Result<cybou_protocol::system::UserAccountRecord, ClientError> {
        Ok(cybou_protocol::system::UserAccountRecord {
            uid: 1001,
            username: username.to_owned(),
            full_name: full_name.to_owned(),
            home_dir: format!("/home/{username}"),
            shell: "/bin/bash".to_owned(),
            groups: vec![username.to_owned()],
            is_admin,
            is_locked: false,
        })
    }

    async fn add_ssh_key(
        &self,
        name: &str,
        public_key: &str,
    ) -> Result<cybou_protocol::system::SshKeyRecord, ClientError> {
        Ok(cybou_protocol::system::SshKeyRecord {
            id: "mock-key-01".to_owned(),
            name: name.to_owned(),
            fingerprint: "SHA256:mockfp".to_owned(),
            key_type: "ssh-ed25519".to_owned(),
            public_key: public_key.to_owned(),
            created_at: "2026-08-28T22:00:00Z".to_owned(),
        })
    }

    async fn delete_ssh_key(&self, key_id: &str) -> Result<String, ClientError> {
        Ok(format!("Mock deleted SSH key {key_id}"))
    }

    async fn get_security_settings(
        &self,
    ) -> Result<cybou_web_contracts::SecuritySettingsProjection, ClientError> {
        Ok(cybou_web_contracts::SecuritySettingsProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            policy: cybou_protocol::system::SecurityPolicyRecord {
                landlock_enabled: true,
                bubblewrap_enabled: true,
                apparmor_enforcing: true,
                seccomp_strict: true,
                egress_firewall_strict: true,
            },
            audit_log: Vec::new(),
        })
    }

    async fn update_security_policy(
        &self,
        req: cybou_web_contracts::UpdateSecurityPolicyRequest,
    ) -> Result<cybou_protocol::system::SecurityPolicyRecord, ClientError> {
        Ok(cybou_protocol::system::SecurityPolicyRecord {
            landlock_enabled: req.landlock_enabled,
            bubblewrap_enabled: req.bubblewrap_enabled,
            apparmor_enforcing: req.apparmor_enforcing,
            seccomp_strict: req.seccomp_strict,
            egress_firewall_strict: req.egress_firewall_strict,
        })
    }

    async fn get_backup_settings(
        &self,
    ) -> Result<cybou_web_contracts::BackupSettingsProjection, ClientError> {
        Ok(cybou_web_contracts::BackupSettingsProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            repository: cybou_protocol::system::BackupRepositoryRecord {
                id: "mock-repo".to_owned(),
                name: "Mock Vault".to_owned(),
                destination: "/var/backups".to_owned(),
                encryption: "repokey".to_owned(),
                last_backup_time: None,
                total_archives: 0,
                total_size_bytes: 0,
            },
            archives: Vec::new(),
            schedule: cybou_protocol::system::BackupScheduleRecord {
                enabled: true,
                frequency: "daily".to_owned(),
                retention_daily: 7,
                retention_weekly: 4,
                retention_monthly: 12,
            },
        })
    }

    async fn trigger_backup(
        &self,
        name: Option<String>,
    ) -> Result<cybou_protocol::system::BackupArchiveRecord, ClientError> {
        Ok(cybou_protocol::system::BackupArchiveRecord {
            id: "mock-arch-01".to_owned(),
            name: name.unwrap_or_else(|| "backup".to_owned()),
            timestamp: "2026-08-28T22:00:00Z".to_owned(),
            size_bytes: 1_000_000_000,
            duration_seconds: 10,
        })
    }

    async fn restore_archive(
        &self,
        archive_id: &str,
        _target_path: Option<String>,
    ) -> Result<String, ClientError> {
        Ok(format!("Mock restored archive {archive_id}"))
    }

    async fn update_backup_schedule(
        &self,
        req: cybou_web_contracts::UpdateBackupScheduleRequest,
    ) -> Result<cybou_protocol::system::BackupScheduleRecord, ClientError> {
        Ok(cybou_protocol::system::BackupScheduleRecord {
            enabled: req.enabled,
            frequency: req.frequency,
            retention_daily: req.retention_daily,
            retention_weekly: req.retention_weekly,
            retention_monthly: req.retention_monthly,
        })
    }

    async fn get_mail(
        &self,
        account_id: Option<String>,
        folder: Option<cybou_protocol::personal::MailFolderKind>,
    ) -> Result<cybou_web_contracts::MailProjection, ClientError> {
        Ok(cybou_web_contracts::MailProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            accounts: Vec::new(),
            messages: Vec::new(),
            active_account_id: account_id.unwrap_or_default(),
            active_folder: folder.unwrap_or(cybou_protocol::personal::MailFolderKind::Inbox),
        })
    }

    async fn send_mail(
        &self,
        req: cybou_web_contracts::SendMailRequest,
    ) -> Result<cybou_protocol::personal::MailMessageRecord, ClientError> {
        Ok(cybou_protocol::personal::MailMessageRecord {
            id: "mock-msg-01".to_owned(),
            account_id: req.account_id,
            folder: cybou_protocol::personal::MailFolderKind::Sent,
            from: "mock@cybou.local".to_owned(),
            to: req.to,
            subject: req.subject,
            preview: req.body.clone(),
            body: req.body,
            timestamp: "2026-08-28T23:00:00Z".to_owned(),
            is_unread: false,
            is_starred: false,
            referenced_subject: req.referenced_subject,
        })
    }

    async fn get_calendar(&self) -> Result<cybou_web_contracts::CalendarProjection, ClientError> {
        Ok(cybou_web_contracts::CalendarProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            events: Vec::new(),
        })
    }

    async fn create_calendar_event(
        &self,
        req: cybou_web_contracts::CreateCalendarEventRequest,
    ) -> Result<cybou_protocol::personal::CalendarEventRecord, ClientError> {
        Ok(cybou_protocol::personal::CalendarEventRecord {
            id: "mock-evt-01".to_owned(),
            title: req.title,
            description: req.description,
            start_time: req.start_time,
            end_time: req.end_time,
            is_all_day: req.is_all_day,
            location: req.location,
            attendees: req.attendees,
            color_category: req.color_category,
            referenced_subject: req.referenced_subject,
        })
    }

    async fn get_notes(&self) -> Result<cybou_web_contracts::NotesProjection, ClientError> {
        Ok(cybou_web_contracts::NotesProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            notes: Vec::new(),
        })
    }

    async fn create_note(
        &self,
        req: cybou_web_contracts::CreateNoteRequest,
    ) -> Result<cybou_protocol::personal::NoteRecord, ClientError> {
        Ok(cybou_protocol::personal::NoteRecord {
            id: "mock-note-01".to_owned(),
            title: req.title,
            content_markdown: req.content_markdown,
            tags: req.tags,
            updated_at: "2026-08-28T23:00:00Z".to_owned(),
            is_pinned: req.is_pinned,
            referenced_subject: req.referenced_subject,
        })
    }

    async fn update_note(
        &self,
        req: cybou_web_contracts::UpdateNoteRequest,
    ) -> Result<cybou_protocol::personal::NoteRecord, ClientError> {
        Ok(cybou_protocol::personal::NoteRecord {
            id: req.id,
            title: req.title,
            content_markdown: req.content_markdown,
            tags: req.tags,
            updated_at: "2026-08-28T23:00:00Z".to_owned(),
            is_pinned: req.is_pinned,
            referenced_subject: None,
        })
    }

    async fn get_contacts(&self) -> Result<cybou_web_contracts::ContactsProjection, ClientError> {
        Ok(cybou_web_contracts::ContactsProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            contacts: Vec::new(),
        })
    }

    async fn create_contact(
        &self,
        req: cybou_web_contracts::CreateContactRequest,
    ) -> Result<cybou_protocol::personal::ContactRecord, ClientError> {
        Ok(cybou_protocol::personal::ContactRecord {
            id: "mock-cnt-01".to_owned(),
            name: req.name,
            email: req.email,
            role: req.role,
            organization: req.organization,
            phone: req.phone,
            tags: req.tags,
            notes: req.notes,
            referenced_subject: req.referenced_subject,
        })
    }

    async fn get_cognitive_graph(
        &self,
        focus: Option<String>,
    ) -> Result<cybou_web_contracts::CognitiveGraphProjection, ClientError> {
        Ok(cybou_web_contracts::CognitiveGraphProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            graph: cybou_protocol::cognitive::CognitiveGraphRecord {
                nodes: Vec::new(),
                edges: Vec::new(),
            },
            focus_node_id: focus,
        })
    }

    async fn query_cognitive_graph(
        &self,
        req: cybou_web_contracts::CognitiveQueryRequest,
    ) -> Result<cybou_web_contracts::CognitiveGraphProjection, ClientError> {
        Ok(cybou_web_contracts::CognitiveGraphProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            graph: cybou_protocol::cognitive::CognitiveGraphRecord {
                nodes: Vec::new(),
                edges: Vec::new(),
            },
            focus_node_id: req.focus_id,
        })
    }

    async fn get_event_journal(
        &self,
        _limit: Option<usize>,
        _offset: Option<usize>,
    ) -> Result<cybou_web_contracts::EventJournalProjection, ClientError> {
        Ok(cybou_web_contracts::EventJournalProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            entries: Vec::new(),
            total_count: 0,
        })
    }

    async fn interpret_meaning(
        &self,
        req: &cybou_web_contracts::MeaningInterpretRequest,
    ) -> Result<cybou_web_contracts::MeaningInterpretProjection, ClientError> {
        let now = time::OffsetDateTime::now_utc();
        Ok(cybou_web_contracts::MeaningInterpretProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            interpretation: cybou_protocol::meaning::MeaningInterpretation {
                utterance: req.utterance.clone(),
                primary_act: cybou_protocol::meaning::CognitiveAct {
                    act_id: uuid::Uuid::new_v4(),
                    kind: cybou_protocol::meaning::CognitiveActKind::Ask,
                    subject: req.utterance.clone(),
                    parameters: Vec::new(),
                    source: "person".to_owned(),
                    evidence: Vec::new(),
                },
                references: Vec::new(),
                confidence: 0.9,
                ambiguous: false,
                derived_at: now,
            },
            response_plan: Some(cybou_protocol::meaning::ResponsePlan {
                plan_id: uuid::Uuid::new_v4(),
                intent: "mock_response".to_owned(),
                key_points: vec![format!("Interpreted query '{}'", req.utterance)],
                referenced_evidence: Vec::new(),
                qualifications: Vec::new(),
            }),
            realization: Some(format!("Mock response to '{}'", req.utterance)),
        })
    }

    async fn get_dialogue_memory(
        &self,
    ) -> Result<cybou_web_contracts::DialogueMemoryProjection, ClientError> {
        Ok(cybou_web_contracts::DialogueMemoryProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            current_turn: 1,
            remembered_referents: vec!["system".to_owned(), "cybou-web-gateway".to_owned()],
            turns_bound: 20,
        })
    }

    async fn get_learning_candidates(
        &self,
        _layer: Option<String>,
    ) -> Result<cybou_web_contracts::LearningCandidatesProjection, ClientError> {
        let now = time::OffsetDateTime::now_utc();
        Ok(cybou_web_contracts::LearningCandidatesProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            candidates: vec![cybou_protocol::learning::LearningCandidate {
                candidate_id: uuid::Uuid::new_v4(),
                layer: cybou_protocol::learning::LearningLayer::Procedural,
                source_evidence: vec![uuid::Uuid::new_v4()],
                outcome_evidence: vec![uuid::Uuid::new_v4()],
                generalization: "Auto-reconnect D-Bus daemon on dropped connection".into(),
                scope: "service.dbus".into(),
                derivation_version: 1,
                created_at: now,
            }],
            total_count: 1,
        })
    }

    async fn propose_learning_candidate(
        &self,
        req: &cybou_web_contracts::ProposeLearningCandidateRequest,
    ) -> Result<cybou_protocol::learning::LearningCandidate, ClientError> {
        let now = time::OffsetDateTime::now_utc();
        Ok(cybou_protocol::learning::LearningCandidate {
            candidate_id: uuid::Uuid::new_v4(),
            layer: req.layer,
            source_evidence: req.source_evidence.clone(),
            outcome_evidence: req.outcome_evidence.clone(),
            generalization: req.generalization.clone(),
            scope: req.scope.clone(),
            derivation_version: 1,
            created_at: now,
        })
    }

    async fn evaluate_learning_candidate(
        &self,
        candidate_id: uuid::Uuid,
        _req: Option<&cybou_web_contracts::EvaluateCandidateRequest>,
    ) -> Result<cybou_web_contracts::CandidateEvaluationProjection, ClientError> {
        let now = time::OffsetDateTime::now_utc();
        Ok(cybou_web_contracts::CandidateEvaluationProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            candidate_id,
            promoted: Some(cybou_protocol::promotion::Promoted {
                candidate_id,
                layer: cybou_protocol::learning::LearningLayer::Procedural,
                independent_episodes: 3,
                success_rate: 1.0,
            }),
            refused: None,
            artifact: Some(cybou_protocol::learning::LearnedArtifactLineage {
                artifact_id: uuid::Uuid::new_v4(),
                layer: cybou_protocol::learning::LearningLayer::Procedural,
                status: cybou_protocol::learning::ArtifactStatus::Promoted,
                contributing_candidates: vec![candidate_id],
                source_evidence: vec![uuid::Uuid::new_v4()],
                promoted_at: Some(now),
                erasure_epoch: 1,
            }),
        })
    }

    async fn get_learned_artifacts(
        &self,
    ) -> Result<cybou_web_contracts::LearnedArtifactsProjection, ClientError> {
        let now = time::OffsetDateTime::now_utc();
        Ok(cybou_web_contracts::LearnedArtifactsProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            artifacts: vec![cybou_protocol::learning::LearnedArtifactLineage {
                artifact_id: uuid::Uuid::new_v4(),
                layer: cybou_protocol::learning::LearningLayer::Procedural,
                status: cybou_protocol::learning::ArtifactStatus::Promoted,
                contributing_candidates: vec![uuid::Uuid::new_v4()],
                source_evidence: vec![uuid::Uuid::new_v4()],
                promoted_at: Some(now),
                erasure_epoch: 1,
            }],
            total_count: 1,
        })
    }

    async fn revoke_learned_artifact(
        &self,
        _artifact_id: uuid::Uuid,
        _reason: &str,
    ) -> Result<(), ClientError> {
        Ok(())
    }

    async fn get_governance_scopes(
        &self,
    ) -> Result<cybou_web_contracts::GovernanceScopesProjection, ClientError> {
        let now = time::OffsetDateTime::now_utc();
        Ok(cybou_web_contracts::GovernanceScopesProjection {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            scopes: vec![cybou_protocol::governance::TaskScope {
                actor_id: uuid::Uuid::new_v4(),
                kind: cybou_protocol::governance::ActorKind::Agent,
                intention_id: Some(uuid::Uuid::new_v4()),
                capabilities: vec!["fs.read".into(), "terminal.exec".into()],
                tool_grants: vec!["git.status".into()],
                network_destinations: vec!["localhost".into()],
                ttl_seconds: 3600,
                max_compute_ms: 60000,
                delegation_permitted: true,
                granted_at: now,
            }],
        })
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

    #[test]
    fn inspector_source_contains_no_unbacked_operational_success_claims() {
        let source = include_str!("components/cards/inspector.rs");
        let forbidden_claims = [
            ["Active /", " Healthy"].concat(),
            ["Just now", " (Live)"].concat(),
            ["Telemetry stream", " opened"].concat(),
            ["Action proposal", " created"].concat(),
        ];

        for forbidden in forbidden_claims {
            assert!(
                !source.contains(&forbidden),
                "Inspector must not present an unbacked claim: {forbidden}"
            );
        }
    }

    #[test]
    fn editor_source_contains_no_unbacked_persistence_success_claims() {
        let source = include_str!("components/cards/editor.rs");
        let forbidden_claims = [
            ["Saved file", " successfully"].concat(),
            ["proposal submitted", " for authorization"].concat(),
        ];

        for forbidden in forbidden_claims {
            assert!(
                !source.contains(&forbidden),
                "Editor must not present an unbacked persistence claim: {forbidden}"
            );
        }
    }

    #[test]
    fn diff_source_contains_no_unbacked_commit_claims() {
        let source = include_str!("components/cards/diff.rs");
        let forbidden_claims = [
            ["queued for", " commit"].concat(),
            ["changes accepted", " and"].concat(),
        ];

        for forbidden in forbidden_claims {
            assert!(
                !source.to_ascii_lowercase().contains(&forbidden),
                "Diff Viewer must not present an unbacked commit claim: {forbidden}"
            );
        }
    }
}
