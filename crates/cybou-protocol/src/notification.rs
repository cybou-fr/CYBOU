// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Typed notification and attention substrate for the CYBOU Desktop.
//!
//! Separates Attention (Action1 proposals), Evidence (telemetry findings),
//! System events, Agent requests, and Operations milestones.

use crate::subject::SubjectRef;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Notification category establishing cognitive urgency and domain authority.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationCategory {
    /// Human approval required for governed Action1 proposals or security decisions.
    Attention,
    /// Automated diagnosis or system finding backed by cited sensor readings.
    Evidence,
    /// Operating system, hardware, storage, service, or network events.
    System,
    /// Autonomous Agent capsule request, completion, or budget notice.
    Agent,
    /// Background operation progress, completion, or failure notice.
    Operation,
}

impl NotificationCategory {
    /// Category display title.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Attention => "Attention",
            Self::Evidence => "Evidence",
            Self::System => "System",
            Self::Agent => "Agent",
            Self::Operation => "Operations",
        }
    }
}

/// Urgency / importance level of a notification.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationSeverity {
    /// Informational background event.
    Info,
    /// Notice of a state change or completed task.
    Notice,
    /// Warning requiring user awareness.
    Warning,
    /// Critical alert or pending required action.
    Critical,
}

/// Action to be taken directly from the notification card or toast.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "actionKind")]
pub enum NotificationActionKind {
    /// Approve an Action1 proposal directly.
    ApproveProposal {
        /// Target proposal identifier.
        proposal_id: Uuid,
    },
    /// Reject an Action1 proposal.
    RejectProposal {
        /// Target proposal identifier.
        proposal_id: Uuid,
    },
    /// Open the Universal Inspector for the referenced subject.
    InspectSubject {
        /// Target subject entity.
        subject: SubjectRef,
    },
    /// Fly camera or focus panel on canvas.
    ShowOnCanvas {
        /// Deep link fragment (e.g. "/#/service/nginx.service").
        deep_link: String,
    },
    /// Cancel an active operation.
    CancelOperation {
        /// Operation ID to cancel.
        operation_id: Uuid,
    },
    /// Dismiss the notification.
    Dismiss,
    /// Custom domain action payload.
    Custom {
        /// Action verb.
        verb: String,
        /// Optional argument.
        payload: Option<String>,
    },
}

/// Button or interactive trigger attached to a notification.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationAction {
    /// Action identifier.
    pub id: String,
    /// Button label.
    pub label: String,
    /// Typed action outcome.
    pub kind: NotificationActionKind,
    /// Whether this button is styled as the primary/emphasized action.
    pub primary: bool,
}

/// Who a notification belongs to.
///
/// Notifications about the host as a whole are for whoever is operating it; notifications drawn
/// from one account's mail, calendar, agents or personal work belong to that account alone. Mixing
/// them would let one authenticated person read, or dismiss, another's.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "scope")]
pub enum NotificationAudience {
    /// About the host itself; readable by any authenticated seat.
    Operator,
    /// About one principal's own work; readable only by that principal.
    Principal {
        /// The server-established principal, as the authenticating boundary named it.
        principal: String,
    },
}

impl NotificationAudience {
    /// Whether the given authenticated principal may see and act on this notification.
    #[must_use]
    pub fn admits(&self, principal: &str) -> bool {
        match self {
            Self::Operator => true,
            Self::Principal { principal: owner } => owner == principal,
        }
    }
}

/// A structured notification record displayed in the Notification Center and toasts.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationItem {
    /// Unique notification identifier.
    pub id: Uuid,
    /// Cognitive category.
    pub category: NotificationCategory,
    /// Severity level.
    pub severity: NotificationSeverity,
    /// Short notification title.
    pub title: String,
    /// Detailed description or rationale.
    pub body: String,
    /// Referenced subject if applicable.
    pub subject: Option<SubjectRef>,
    /// Creation timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// Whether the user has marked this notification as read.
    pub read: bool,
    /// Whether this notification was dismissed.
    pub dismissed: bool,
    /// Interactive action triggers.
    pub actions: Vec<NotificationAction>,
    /// Who this notification belongs to.
    pub audience: NotificationAudience,
}
