// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! User-scoped draft storage for safe recovery (ADR-0045, ADR-0046).
//!
//! Drafts remain strictly isolated to the authenticated seat and do not leak to client-side localStorage.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use cybou_web_contracts::{
    UserDraftDeleteRequest, UserDraftListProjection, UserDraftProjection, UserDraftSaveRequest,
    WEB_SCHEMA_V1,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::state::{ErrorBody, GatewayState};

/// Thread-safe in-memory draft store partitioned by seat owner identifier.
#[derive(Clone, Debug, Default)]
pub struct UserDraftStore {
    inner: Arc<Mutex<HashMap<String, HashMap<String, UserDraftProjection>>>>,
}

impl UserDraftStore {
    /// Create a new empty in-memory draft store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// List all drafts belonging to a specific authenticated seat key.
    #[must_use]
    pub fn list(&self, seat_key: &str) -> Vec<UserDraftProjection> {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard
            .get(seat_key)
            .map(|drafts| drafts.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Save or overwrite a draft for a specific authenticated seat key.
    pub fn save(&self, seat_key: &str, draft: UserDraftProjection) {
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let drafts = guard.entry(seat_key.to_string()).or_default();
        drafts.insert(draft.draft_id.clone(), draft);
    }

    /// Remove a draft for a specific authenticated seat key by draft ID.
    pub fn delete(&self, seat_key: &str, draft_id: &str) -> bool {
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(drafts) = guard.get_mut(seat_key) {
            drafts.remove(draft_id).is_some()
        } else {
            false
        }
    }
}

type Refusal = (StatusCode, Json<ErrorBody>);

fn no_seat() -> Refusal {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorBody {
            schema_version: WEB_SCHEMA_V1,
            error: "shellExecutionForbiddenInPublicPreview",
            retryable: false,
        }),
    )
}

fn owner_seat_key(owner: &crate::shells::ShellOwner) -> String {
    match owner {
        crate::shells::ShellOwner::LocalDesktop { .. } => "local-desktop".to_string(),
        crate::shells::ShellOwner::Session { session, .. } => {
            let mut id = String::from("session-");
            for byte in &session[..8] {
                use std::fmt::Write as _;
                let _ = write!(id, "{byte:02x}");
            }
            id
        }
    }
}

/// List all drafts for the authenticated seat.
pub async fn list_drafts_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<UserDraftListProjection>, Refusal> {
    let owner = state.shell_seat(&headers).ok_or_else(no_seat)?;
    let seat_key = owner_seat_key(&owner);
    let drafts = state.drafts.list(&seat_key);
    Ok(Json(UserDraftListProjection {
        schema_version: WEB_SCHEMA_V1,
        drafts,
    }))
}

/// Save or update a draft for the authenticated seat.
pub async fn save_draft_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<UserDraftSaveRequest>,
) -> Result<Json<UserDraftProjection>, Refusal> {
    let owner = state.shell_seat(&headers).ok_or_else(no_seat)?;
    let seat_key = owner_seat_key(&owner);
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());

    let draft = UserDraftProjection {
        draft_id: payload.draft_id,
        title: payload.title,
        content: payload.content,
        base_location: payload.base_location,
        base_sha256: payload.base_sha256,
        updated_at_utc: now,
    };

    state.drafts.save(&seat_key, draft.clone());
    Ok(Json(draft))
}

/// Delete a draft for the authenticated seat.
pub async fn delete_draft_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<UserDraftDeleteRequest>,
) -> Result<StatusCode, Refusal> {
    let owner = state.shell_seat(&headers).ok_or_else(no_seat)?;
    let seat_key = owner_seat_key(&owner);
    state.drafts.delete(&seat_key, &payload.draft_id);
    Ok(StatusCode::NO_CONTENT)
}
