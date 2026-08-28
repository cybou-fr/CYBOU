// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! HTTP endpoints for querying and managing desktop notifications.

use axum::{
    Json,
    extract::State,
    http::StatusCode,
};
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

    #[test]
    fn notifications_hub_filters_and_dismisses() {
        let hub = NotificationsHub::new();
        let list = hub.list();
        assert!(list.attention_count >= 1);
        let first_notif = &list.notifications[0];
        let notif_id = first_notif.id;

        assert!(!first_notif.actions.is_empty());
        let action_id = &first_notif.actions[0].id;

        let result = hub.execute_action(notif_id, action_id);
        assert!(result.is_ok());

        let after = hub.list();
        let updated = after.notifications.iter().find(|n| n.id == notif_id).unwrap();
        assert!(updated.read);
    }
}
