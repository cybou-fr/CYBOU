// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Notifications Hub for aggregating Attention, Evidence, System alerts, and Agent requests.

use std::sync::RwLock;
use cybou_protocol::{
    notification::{
        NotificationAction, NotificationActionKind, NotificationCategory, NotificationItem,
        NotificationSeverity,
    },
    subject::SubjectRef,
};
use cybou_web_contracts::{NotificationsListProjection, WEB_SCHEMA_V1};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::state::GatewayError;

/// Maximum notifications retained in memory.
const MAX_NOTIFICATIONS: usize = 200;

/// Hub for managing reactive desktop notifications.
#[derive(Debug)]
pub struct NotificationsHub {
    items: RwLock<Vec<NotificationItem>>,
}

impl Default for NotificationsHub {
    fn default() -> Self {
        let hub = Self {
            items: RwLock::new(Vec::new()),
        };
        hub.seed_initial_notifications();
        hub
    }
}

impl NotificationsHub {
    /// Create a new NotificationsHub.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn seed_initial_notifications(&self) {
        let now = OffsetDateTime::now_utc();
        let proposal_id = Uuid::new_v4();

        let n1 = NotificationItem {
            id: Uuid::new_v4(),
            category: NotificationCategory::Attention,
            severity: NotificationSeverity::Warning,
            title: "Action1 Proposal: Restart Network Gateway".to_owned(),
            body: "Agent 'net-architect' proposed restarting 'cybou-egressd.service' to apply new egress isolation rules.".to_owned(),
            subject: Some(SubjectRef::Service {
                name: "cybou-egressd.service".to_owned(),
                node_id: None,
            }),
            created_at: now,
            read: false,
            dismissed: false,
            actions: vec![
                NotificationAction {
                    id: "approve".to_owned(),
                    label: "Approve Action".to_owned(),
                    kind: NotificationActionKind::ApproveProposal { proposal_id },
                    primary: true,
                },
                NotificationAction {
                    id: "reject".to_owned(),
                    label: "Reject".to_owned(),
                    kind: NotificationActionKind::RejectProposal { proposal_id },
                    primary: false,
                },
                NotificationAction {
                    id: "inspect".to_owned(),
                    label: "Inspect Service".to_owned(),
                    kind: NotificationActionKind::InspectSubject {
                        subject: SubjectRef::Service {
                            name: "cybou-egressd.service".to_owned(),
                            node_id: None,
                        },
                    },
                    primary: false,
                },
            ],
        };

        let n2 = NotificationItem {
            id: Uuid::new_v4(),
            category: NotificationCategory::Evidence,
            severity: NotificationSeverity::Info,
            title: "Telemetry Finding: TLS Certificate Renewal".to_owned(),
            body: "Insight observer verified automated ACME certificate rotation for gateway domain.".to_owned(),
            subject: Some(SubjectRef::Certificate {
                domain: "localhost.cybou.internal".to_owned(),
                thumbprint: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
            }),
            created_at: now,
            read: false,
            dismissed: false,
            actions: vec![
                NotificationAction {
                    id: "inspect".to_owned(),
                    label: "View Certificate".to_owned(),
                    kind: NotificationActionKind::InspectSubject {
                        subject: SubjectRef::Certificate {
                            domain: "localhost.cybou.internal".to_owned(),
                            thumbprint: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_owned(),
                        },
                    },
                    primary: false,
                },
            ],
        };

        let n3 = NotificationItem {
            id: Uuid::new_v4(),
            category: NotificationCategory::System,
            severity: NotificationSeverity::Notice,
            title: "Storage Pool: Btrfs Scrub Complete".to_owned(),
            body: "Scheduled integrity scrub on user home filesystem completed with 0 read errors.".to_owned(),
            subject: Some(SubjectRef::Filesystem {
                mount_point: "/home".to_owned(),
                fs_type: "btrfs".to_owned(),
            }),
            created_at: now,
            read: true,
            dismissed: false,
            actions: vec![
                NotificationAction {
                    id: "dismiss".to_owned(),
                    label: "Dismiss".to_owned(),
                    kind: NotificationActionKind::Dismiss,
                    primary: false,
                },
            ],
        };

        let n4 = NotificationItem {
            id: Uuid::new_v4(),
            category: NotificationCategory::Agent,
            severity: NotificationSeverity::Info,
            title: "Agent OpenCode: Task Complete".to_owned(),
            body: "Agent completed patch analysis on 'crates/living-canvas'. Created draft changes.".to_owned(),
            subject: Some(SubjectRef::Agent {
                capsule_id: "agent-opencode-01".to_owned(),
                agent_type: "opencode-coder".to_owned(),
            }),
            created_at: now,
            read: false,
            dismissed: false,
            actions: vec![
                NotificationAction {
                    id: "canvas".to_owned(),
                    label: "Focus Agent on Canvas".to_owned(),
                    kind: NotificationActionKind::ShowOnCanvas {
                        deep_link: "/#/agent/agent-opencode-01".to_owned(),
                    },
                    primary: true,
                },
            ],
        };

        let mut items = self.items.write().expect("lock notifications");
        items.push(n1);
        items.push(n2);
        items.push(n3);
        items.push(n4);
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
        } else if let Some(target_id) = id {
            if let Some(item) = items.iter_mut().find(|n| n.id == target_id) {
                item.dismissed = true;
                item.read = true;
            }
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
    pub fn execute_action(&self, id: Uuid, action_id: &str) -> Result<String, GatewayError> {
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

        match action.kind {
            NotificationActionKind::ApproveProposal { proposal_id } => {
                item.dismissed = true;
                Ok(format!("Action proposal {proposal_id} approved"))
            }
            NotificationActionKind::RejectProposal { proposal_id } => {
                item.dismissed = true;
                Ok(format!("Action proposal {proposal_id} rejected"))
            }
            NotificationActionKind::Dismiss => {
                item.dismissed = true;
                Ok("Notification dismissed".to_owned())
            }
            NotificationActionKind::InspectSubject { ref subject } => {
                Ok(format!("Inspect {}", subject.display_title()))
            }
            NotificationActionKind::ShowOnCanvas { ref deep_link } => {
                Ok(format!("Navigating to {deep_link}"))
            }
            NotificationActionKind::CancelOperation { operation_id } => {
                Ok(format!("Cancelling operation {operation_id}"))
            }
            NotificationActionKind::Custom { ref verb, .. } => {
                Ok(format!("Executed action {verb}"))
            }
        }
    }
}
