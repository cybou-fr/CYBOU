// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Server-owned asynchronous operations and long-running task substrate.
//!
//! Separates asynchronous server tasks (package updates, backups, agent tasks, service restarts)
//! from transient browser states, providing persistent progress tracking, log streaming,
//! and cancellation tokens.

use crate::action::Proposer;
use crate::subject::SubjectRef;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Category of an asynchronous server operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationKind {
    /// Installing or upgrading system package.
    PackageInstall,
    /// Removing system package.
    PackageRemove,
    /// Restarting system service daemon.
    ServiceRestart,
    /// Stopping system service daemon.
    ServiceStop,
    /// Creating filesystem or database backup snapshot.
    BackupCreate,
    /// Restoring from backup snapshot.
    BackupRestore,
    /// File transfer, upload, download, or copy.
    FileTransfer,
    /// Long-running agent task execution in a capsule.
    AgentTask,
    /// Governed OS / system configuration update.
    SystemUpdate,
    /// Workspace semantic search / symbol indexing.
    IndexWorkspace,
    /// Custom domain operation.
    Custom(String),
}

impl OperationKind {
    /// Human-readable category label.
    #[must_use]
    pub fn display_label(&self) -> String {
        match self {
            Self::PackageInstall => "Package Installation".to_owned(),
            Self::PackageRemove => "Package Removal".to_owned(),
            Self::ServiceRestart => "Service Restart".to_owned(),
            Self::ServiceStop => "Service Stop".to_owned(),
            Self::BackupCreate => "Backup Creation".to_owned(),
            Self::BackupRestore => "Backup Restoration".to_owned(),
            Self::FileTransfer => "File Transfer".to_owned(),
            Self::AgentTask => "Agent Task".to_owned(),
            Self::SystemUpdate => "System Update".to_owned(),
            Self::IndexWorkspace => "Workspace Indexing".to_owned(),
            Self::Custom(label) => label.clone(),
        }
    }
}

/// Lifecycle state of a server operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "status")]
pub enum OperationState {
    /// Operation is queued awaiting execution capacity or approval.
    Queued,
    /// Operation is currently executing.
    Running,
    /// Operation completed successfully.
    Completed,
    /// Operation failed with an error message.
    Failed {
        /// Failure reason.
        error: String,
    },
    /// Operation was cancelled before completion.
    Cancelled,
}

impl OperationState {
    /// Whether the operation has reached a terminal state.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed { .. } | Self::Cancelled
        )
    }

    /// Status badge label.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Running => "Running",
            Self::Completed => "Completed",
            Self::Failed { .. } => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }
}

/// How well the owner can still observe the real work behind an operation.
///
/// Lifecycle state answers "what did the work do"; observation answers "can the owner still see
/// it". Keeping them apart stops a restored `Running` record from claiming, forever, that a
/// vanished worker is still executing.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationState {
    /// The executing authority still establishes this operation.
    #[default]
    Known,
    /// The operation was observed recently but the last reconciliation did not confirm it.
    Stale,
    /// The executing authority no longer establishes this operation; its outcome is unknown.
    Detached,
    /// The executing authority itself cannot be read right now.
    Unavailable,
}

impl ObservationState {
    /// Badge label.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Known => "Known",
            Self::Stale => "Stale",
            Self::Detached => "Detached",
            Self::Unavailable => "Unavailable",
        }
    }

    /// Whether the owner can still vouch for the reported lifecycle state.
    #[must_use]
    pub const fn is_established(&self) -> bool {
        matches!(self, Self::Known)
    }
}

/// Outcome of a cancel request, distinguishing an accepted request from a confirmed teardown.
///
/// `CancellationAccepted` means the request was recorded and signalled; only the worker may later
/// publish a terminal state. `CancellationConfirmed` means the executing authority already tore the
/// work down and the terminal state is published.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CancelOutcome {
    /// Request durably recorded and signalled; the operation is still running.
    CancellationAccepted,
    /// Teardown confirmed by the executing authority; the operation is terminal.
    CancellationConfirmed,
    /// No such operation.
    NotFound,
    /// The operation is already terminal or the request could not be recorded.
    Conflict,
    /// The operation does not accept cancellation.
    Refused,
}

/// Step-by-step progress tracking for an operation.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationProgress {
    /// Execution percentage (0.0 to 100.0) if determinable.
    pub percent: Option<f32>,
    /// Name of the current active step (e.g. "Downloading packages", "Writing disk image").
    pub step: String,
    /// Total expected steps.
    pub total_steps: Option<u32>,
    /// Current step index (1-based).
    pub current_step: Option<u32>,
    /// Optional detailed progress description or speed metric.
    pub detail: Option<String>,
}

/// One line in the operation's execution log stream.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationLogEntry {
    /// RFC 3339 timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
    /// Output stream ("stdout", "stderr", "system").
    pub stream: String,
    /// Log line text content.
    pub text: String,
}

/// Durable or in-flight record of a server-owned operation.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationRecord {
    /// Unique operation identifier.
    pub id: Uuid,
    /// Operation category.
    pub kind: OperationKind,
    /// Current lifecycle state.
    pub state: OperationState,
    /// Human-readable title (e.g. "Updating Linux kernel", "Creating daily backup").
    pub label: String,
    /// Who initiated the operation.
    pub initiator: Proposer,
    /// Target subject being mutated or inspected.
    pub subject: Option<SubjectRef>,
    /// Live execution progress.
    pub progress: OperationProgress,
    /// Whether this operation accepts an explicit cancel request.
    pub cancellable: bool,
    /// Whether a cancel request is recorded and still awaiting a worker-published terminal state.
    #[serde(default)]
    pub cancellation_requested: bool,
    /// How well the owner can still observe the real work.
    #[serde(default)]
    pub observation: ObservationState,
    /// When the executing authority last established this operation.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub last_observed_at: Option<OffsetDateTime>,
    /// When the operation started.
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
    /// When the progress or status was last updated.
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    /// When the operation reached a terminal state.
    #[serde(default, with = "time::serde::rfc3339::option")]
    pub finished_at: Option<OffsetDateTime>,
}
