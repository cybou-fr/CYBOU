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
        matches!(self, Self::Completed | Self::Failed { .. } | Self::Cancelled)
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
