// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Notifications Hub for aggregating Attention, Evidence, System alerts, and Agent requests.

use cybou_protocol::notification::{
    NotificationActionKind, NotificationCategory, NotificationItem,
};
use cybou_web_contracts::{NotificationsListProjection, WEB_SCHEMA_V1};
use std::sync::RwLock;
use uuid::Uuid;

use crate::{operations_hub::OperationsHub, state::GatewayError};

/// Maximum notifications retained in memory.
const MAX_NOTIFICATIONS: usize = 200;

/// Hub for managing reactive desktop notifications.
#[derive(Debug)]
pub struct NotificationsHub {
    items: RwLock<Vec<NotificationItem>>,
}

impl Default for NotificationsHub {
    fn default() -> Self {
        Self {
            items: RwLock::new(Vec::new()),
        }
    }
}

impl NotificationsHub {
    /// Create a new `NotificationsHub`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// List the notifications this principal is entitled to see.
    ///
    /// Operator notices are about the host and reach every authenticated seat; everything else
    /// belongs to one principal and is invisible to the others.
    ///
    /// # Panics
    ///
    /// Panics when the in-process notification lock is poisoned, which means another thread failed while holding it.
    #[must_use]
    pub fn list(&self, principal: &str) -> NotificationsListProjection {
        let items: Vec<NotificationItem> = self
            .items
            .read()
            .expect("read notifications")
            .iter()
            .filter(|item| item.audience.admits(principal))
            .cloned()
            .collect();
        let unread_count = items.iter().filter(|n| !n.read && !n.dismissed).count();
        let attention_count = items
            .iter()
            .filter(|n| n.category == NotificationCategory::Attention && !n.dismissed)
            .count();
        NotificationsListProjection {
            schema_version: WEB_SCHEMA_V1,
            unread_count,
            attention_count,
            notifications: items,
        }
    }

    /// Push a new notification item.
    ///
    /// # Panics
    ///
    /// Panics when the in-process notification lock is poisoned, which means another thread failed while holding it.
    pub fn push(&self, item: NotificationItem) {
        let mut items = self.items.write().expect("write notifications");
        items.insert(0, item);
        if items.len() > MAX_NOTIFICATIONS {
            items.truncate(MAX_NOTIFICATIONS);
        }
    }

    /// Dismiss one, or all, of this principal's own notifications.
    ///
    /// "All" means all of theirs. Dismissing never reaches another principal's notification, and
    /// naming one that is not theirs changes nothing.
    ///
    /// # Panics
    ///
    /// Panics when the in-process notification lock is poisoned, which means another thread failed while holding it.
    pub fn dismiss(&self, principal: &str, id: Option<Uuid>, dismiss_all: bool) {
        let mut items = self.items.write().expect("write notifications");
        if dismiss_all {
            for item in items
                .iter_mut()
                .filter(|item| item.audience.admits(principal))
            {
                item.dismissed = true;
                item.read = true;
            }
        } else if let Some(target_id) = id
            && let Some(item) = items
                .iter_mut()
                .find(|n| n.id == target_id && n.audience.admits(principal))
        {
            item.dismissed = true;
            item.read = true;
        }
    }

    /// Mark one of this principal's own notifications as read.
    ///
    /// # Panics
    ///
    /// Panics when the in-process notification lock is poisoned, which means another thread failed while holding it.
    pub fn mark_read(&self, principal: &str, id: Uuid) {
        let mut items = self.items.write().expect("write notifications");
        if let Some(item) = items
            .iter_mut()
            .find(|n| n.id == id && n.audience.admits(principal))
        {
            item.read = true;
        }
    }

    /// Execute a notification action.
    ///
    /// # Panics
    ///
    /// Panics when the in-process notification lock is poisoned, which means another thread failed while holding it.
    ///
    /// # Errors
    ///
    /// Reports not found when the named record does not exist.
    pub async fn execute_action(
        &self,
        operations: &OperationsHub,
        principal: &str,
        id: Uuid,
        action_id: &str,
    ) -> Result<String, GatewayError> {
        let action = {
            let mut items = self.items.write().expect("write notifications");
            // A notification this principal may not see does not exist for them, so acting on it
            // is not found rather than refused: the refusal would itself disclose that it exists.
            let item = items
                .iter_mut()
                .find(|n| n.id == id && n.audience.admits(principal))
                .ok_or(GatewayError::NotFound)?;
            let action = item
                .actions
                .iter()
                .find(|a| a.id == action_id)
                .ok_or(GatewayError::NotFound)?
                .clone();
            item.read = true;
            action
        };

        match action.kind {
            // Answering a proposal is Action1's, and it needs the seat and the decision the
            // person was shown; a notification carries neither.
            NotificationActionKind::ApproveProposal { .. }
            | NotificationActionKind::RejectProposal { .. } => Err(GatewayError::Refused),
            NotificationActionKind::Dismiss => {
                self.dismiss(principal, Some(id), false);
                Ok("Notification dismissed".to_owned())
            }
            NotificationActionKind::InspectSubject { ref subject } => {
                Ok(format!("Inspect {}", subject.display_title()))
            }
            NotificationActionKind::ShowOnCanvas { ref deep_link } => {
                Ok(format!("Navigating to {deep_link}"))
            }
            NotificationActionKind::CancelOperation { operation_id } => {
                match operations.cancel(operation_id).await? {
                    cybou_protocol::operation::CancelOutcome::CancellationConfirmed => {
                        Ok(format!("Operation {operation_id} cancelled"))
                    }
                    _ => Ok(format!(
                        "Cancellation requested for operation {operation_id}"
                    )),
                }
            }
            NotificationActionKind::Custom { ref verb, .. } => {
                let _ = verb;
                Err(GatewayError::Refused)
            }
        }
    }
}
