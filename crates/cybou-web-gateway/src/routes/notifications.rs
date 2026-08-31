// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! HTTP endpoints for querying and managing desktop notifications.

use axum::{Json, extract::State, http::StatusCode};
use cybou_web_contracts::{
    NotificationActionRequest, NotificationDismissRequest, NotificationsListProjection,
};

use crate::state::{GatewayError, GatewayState};

/// GET `/api/v1/notifications`
pub async fn list_notifications(
    State(state): State<GatewayState>,
) -> Result<Json<NotificationsListProjection>, GatewayError> {
    Ok(Json(state.notifications.list()))
}

/// POST `/api/v1/notifications/dismiss`
pub async fn dismiss_notifications(
    State(state): State<GatewayState>,
    Json(request): Json<NotificationDismissRequest>,
) -> Result<StatusCode, GatewayError> {
    state
        .notifications
        .dismiss(request.notification_id, request.dismiss_all);
    Ok(StatusCode::OK)
}

/// POST `/api/v1/notifications/action`
pub async fn execute_notification_action(
    State(state): State<GatewayState>,
    Json(request): Json<NotificationActionRequest>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let outcome = state
        .notifications
        .execute_action(request.notification_id, &request.action_id)?;
    Ok(Json(serde_json::json!({ "outcome": outcome })))
}

#[cfg(test)]
mod tests {
    use crate::notifications_hub::NotificationsHub;
    use cybou_protocol::notification::{
        NotificationAction, NotificationActionKind, NotificationCategory, NotificationItem,
        NotificationSeverity,
    };
    use time::OffsetDateTime;
    use uuid::Uuid;

    #[test]
    fn notifications_hub_filters_and_dismisses() {
        let hub = NotificationsHub::new();
        let list = hub.list();
        assert_eq!(list.notifications.len(), 0);

        let notif_id = Uuid::new_v4();
        hub.push(NotificationItem {
            id: notif_id,
            category: NotificationCategory::Attention,
            severity: NotificationSeverity::Warning,
            title: "Test Warning".to_owned(),
            body: "Attention needed".to_owned(),
            subject: None,
            created_at: OffsetDateTime::now_utc(),
            read: false,
            dismissed: false,
            actions: vec![NotificationAction {
                id: "dismiss".to_owned(),
                label: "Dismiss".to_owned(),
                kind: NotificationActionKind::Dismiss,
                primary: true,
            }],
        });

        let after = hub.list();
        assert_eq!(after.attention_count, 1);

        let result = hub.execute_action(notif_id, "dismiss");
        assert!(result.is_ok());

        let final_list = hub.list();
        let updated = final_list
            .notifications
            .iter()
            .find(|n| n.id == notif_id)
            .unwrap();
        assert!(updated.dismissed);
    }
}
