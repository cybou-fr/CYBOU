// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! HTTP endpoints for querying and managing desktop notifications.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use cybou_web_contracts::{
    NotificationActionRequest, NotificationDismissRequest, NotificationsListProjection,
};

use crate::state::{GatewayError, GatewayState};

/// GET `/api/v1/notifications`
///
/// # Errors
///
/// Refuses when the request holds no authenticated seat.
pub async fn list_notifications(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<NotificationsListProjection>, GatewayError> {
    let principal = state
        .authenticated_principal(&headers)
        .ok_or(GatewayError::Refused)?;
    Ok(Json(state.notifications.list(&principal)))
}

/// POST `/api/v1/notifications/dismiss`
///
/// # Errors
///
/// Refuses when the request holds no authenticated seat.
pub async fn dismiss_notifications(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<NotificationDismissRequest>,
) -> Result<StatusCode, GatewayError> {
    let principal = state
        .authenticated_principal(&headers)
        .ok_or(GatewayError::Refused)?;
    state
        .notifications
        .dismiss(&principal, request.notification_id, request.dismiss_all);
    Ok(StatusCode::OK)
}

/// POST `/api/v1/notifications/action`
///
/// # Errors
///
/// Refuses when the request holds no authenticated seat.
pub async fn execute_notification_action(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<NotificationActionRequest>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let principal = state
        .authenticated_principal(&headers)
        .ok_or(GatewayError::Refused)?;
    let outcome = state
        .notifications
        .execute_action(
            &state.operations,
            &principal,
            request.notification_id,
            &request.action_id,
        )
        .await?;
    Ok(Json(serde_json::json!({ "outcome": outcome })))
}

#[cfg(test)]
mod tests {
    use crate::notifications_hub::NotificationsHub;
    use cybou_protocol::notification::{
        NotificationAction, NotificationActionKind, NotificationAudience, NotificationCategory,
        NotificationItem, NotificationSeverity,
    };
    use time::OffsetDateTime;
    use uuid::Uuid;

    fn item(id: Uuid, audience: NotificationAudience) -> NotificationItem {
        NotificationItem {
            id,
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
            audience,
        }
    }

    fn alice() -> NotificationAudience {
        NotificationAudience::Principal {
            principal: "linux-account:alice".to_owned(),
        }
    }

    #[tokio::test]
    async fn notifications_hub_filters_and_dismisses() {
        let hub = NotificationsHub::new();
        let operations = crate::operations_hub::OperationsHub::new();
        assert_eq!(hub.list("linux-account:alice").notifications.len(), 0);

        let notif_id = Uuid::new_v4();
        hub.push(item(notif_id, alice()));

        let after = hub.list("linux-account:alice");
        assert_eq!(after.attention_count, 1);

        let result = hub
            .execute_action(&operations, "linux-account:alice", notif_id, "dismiss")
            .await;
        assert!(result.is_ok());

        let final_list = hub.list("linux-account:alice");
        let updated = final_list
            .notifications
            .iter()
            .find(|n| n.id == notif_id)
            .expect("own notification is listed");
        assert!(updated.dismissed);
    }

    #[tokio::test]
    async fn one_principal_can_neither_read_nor_dismiss_another_principals_notification() {
        let hub = NotificationsHub::new();
        let operations = crate::operations_hub::OperationsHub::new();
        let hers = Uuid::new_v4();
        let host = Uuid::new_v4();
        hub.push(item(hers, alice()));
        hub.push(item(host, NotificationAudience::Operator));

        // Bob sees the host notice and nothing of Alice's.
        let bob = hub.list("linux-account:bob");
        let visible: Vec<Uuid> = bob.notifications.iter().map(|item| item.id).collect();
        assert_eq!(visible, vec![host]);

        // A dismiss-all from Bob is all of Bob's, and reaches nothing of Alice's.
        hub.dismiss("linux-account:bob", None, true);
        // Naming her notification directly changes nothing either.
        hub.dismiss("linux-account:bob", Some(hers), false);
        assert!(
            hub.execute_action(&operations, "linux-account:bob", hers, "dismiss")
                .await
                .is_err()
        );

        let alice_view = hub.list("linux-account:alice");
        let still_hers = alice_view
            .notifications
            .iter()
            .find(|item| item.id == hers)
            .expect("her notification survives");
        assert!(!still_hers.dismissed);
        assert!(!still_hers.read);
    }
}
