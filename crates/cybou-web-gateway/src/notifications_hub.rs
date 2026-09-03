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

    /// List active notifications.
    #[must_use]
    pub fn list(&self) -> NotificationsListProjection {
        let items = self.items.read().expect("read notifications").clone();
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
    pub fn push(&self, item: NotificationItem) {
        let mut items = self.items.write().expect("write notifications");
        items.insert(0, item);
        if items.len() > MAX_NOTIFICATIONS {
            items.truncate(MAX_NOTIFICATIONS);
        }
    }

    /// Dismiss one or all notifications.
    pub fn dismiss(&self, id: Option<Uuid>, dismiss_all: bool) {
        let mut items = self.items.write().expect("write notifications");
        if dismiss_all {
            for item in items.iter_mut() {
                item.dismissed = true;
                item.read = true;
            }
        } else if let Some(target_id) = id
            && let Some(item) = items.iter_mut().find(|n| n.id == target_id)
        {
            item.dismissed = true;
            item.read = true;
        }
    }

    /// Mark a notification as read.
    pub fn mark_read(&self, id: Uuid) {
        let mut items = self.items.write().expect("write notifications");
        if let Some(item) = items.iter_mut().find(|n| n.id == id) {
            item.read = true;
        }
    }

    /// Execute a notification action.
    pub async fn execute_action(
        &self,
        operations: &OperationsHub,
        id: Uuid,
        action_id: &str,
    ) -> Result<String, GatewayError> {
        let action = {
            let mut items = self.items.write().expect("write notifications");
            let item = items
                .iter_mut()
                .find(|n| n.id == id)
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
            NotificationActionKind::ApproveProposal { .. } => Err(GatewayError::Refused),
            NotificationActionKind::RejectProposal { .. } => Err(GatewayError::Refused),
            NotificationActionKind::Dismiss => {
                self.dismiss(Some(id), false);
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
