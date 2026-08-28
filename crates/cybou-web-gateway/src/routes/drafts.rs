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
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use time::Duration;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::state::{ErrorBody, GatewayState};

/// Resolve the private operational draft database path for this deployment.
#[must_use]
pub fn draft_database_path(sandbox_path: &Path) -> std::path::PathBuf {
    if let Ok(configured) = std::env::var("CYBOU_DRAFT_DB") {
        return configured.into();
    }
    #[cfg(test)]
    return sandbox_path.join(".cybou-private/drafts.sqlite3");
    #[cfg(not(test))]
    {
        let _ = sandbox_path;
        let state_root = std::env::var("XDG_STATE_HOME").map_or_else(
            |_| {
                std::env::var("HOME").map_or_else(
                    |_| std::path::PathBuf::from("/var/lib/cybou/state"),
                    |home| std::path::PathBuf::from(home).join(".local/state"),
                )
            },
            std::path::PathBuf::from,
        );
        state_root.join("cybou/drafts.sqlite3")
    }
}

/// Maximum UTF-8 bytes accepted for one draft identifier.
pub const DRAFT_MAX_ID_BYTES: usize = 128;
/// Maximum UTF-8 bytes accepted for one draft title.
pub const DRAFT_MAX_TITLE_BYTES: usize = 256;
/// Maximum content bytes accepted for one draft.
pub const DRAFT_MAX_CONTENT_BYTES: usize = 1024 * 1024;
/// Maximum serialized display-location bytes retained with a draft.
pub const DRAFT_MAX_LOCATION_BYTES: usize = 4096;
/// Maximum drafts retained for one authenticated seat.
pub const DRAFT_MAX_PER_SEAT: usize = 32;
/// Maximum aggregate draft bytes retained for one authenticated seat.
pub const DRAFT_MAX_BYTES_PER_SEAT: usize = 16 * 1024 * 1024;
/// Emergency maximum aggregate draft bytes retained by the gateway process.
pub const DRAFT_MAX_GLOBAL_BYTES: usize = 128 * 1024 * 1024;
/// Emergency maximum number of drafts retained by the gateway process.
pub const DRAFT_MAX_GLOBAL_COUNT: usize = 4096;
const DRAFT_TTL_DAYS: i64 = 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Reason a draft was refused by the bounded persistent store.
pub enum DraftStoreError {
    /// One request field exceeds its fixed input limit.
    FieldTooLarge,
    /// The seat already retains the maximum number of drafts.
    SeatCountLimit,
    /// Saving would exceed the seat's aggregate byte budget.
    SeatByteLimit,
    /// Saving would exceed the process emergency budget.
    GlobalLimit,
    /// The private operational database could not complete the request.
    StorageUnavailable,
}

/// Thread-safe bounded `SQLite` draft store partitioned by authenticated principal.
#[derive(Clone)]
pub struct UserDraftStore {
    inner: Arc<Mutex<Connection>>,
    ttl: Duration,
}

impl Default for UserDraftStore {
    fn default() -> Self {
        Self::new()
    }
}

impl UserDraftStore {
    /// Create an isolated in-memory store, primarily for tests.
    ///
    /// # Panics
    ///
    /// Panics only if `SQLite` cannot initialize an in-memory database.
    #[must_use]
    pub fn new() -> Self {
        Self::from_connection(Connection::open_in_memory().expect("initialize in-memory draft DB"))
            .expect("initialize draft schema")
    }

    #[cfg(test)]
    fn with_ttl(ttl: Duration) -> Self {
        let mut store = Self::new();
        store.ttl = ttl;
        store
    }

    /// Open or create a durable private draft database.
    ///
    /// # Errors
    ///
    /// Returns [`DraftStoreError::StorageUnavailable`] if its directory, database, permissions,
    /// or schema cannot be established.
    pub fn open(path: &Path) -> Result<Self, DraftStoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| DraftStoreError::StorageUnavailable)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
                    .map_err(|_| DraftStoreError::StorageUnavailable)?;
            }
        }
        let connection = Connection::open(path).map_err(|_| DraftStoreError::StorageUnavailable)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .map_err(|_| DraftStoreError::StorageUnavailable)?;
        }
        Self::from_connection(connection)
    }

    fn from_connection(connection: Connection) -> Result<Self, DraftStoreError> {
        connection
            .execute_batch(
                "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
                 CREATE TABLE IF NOT EXISTS drafts (
                   principal TEXT NOT NULL, draft_id TEXT NOT NULL, payload TEXT NOT NULL,
                   bytes INTEGER NOT NULL, expires_at INTEGER NOT NULL,
                   PRIMARY KEY (principal, draft_id)
                 );
                 CREATE INDEX IF NOT EXISTS drafts_expiry ON drafts(expires_at);",
            )
            .map_err(|_| DraftStoreError::StorageUnavailable)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(connection)),
            ttl: Duration::days(DRAFT_TTL_DAYS),
        })
    }

    fn draft_bytes(draft: &UserDraftProjection) -> Result<usize, DraftStoreError> {
        let location_bytes = draft
            .base_location
            .as_ref()
            .map_or(0, |location| location.display_path().len());
        if draft.draft_id.is_empty()
            || draft.draft_id.len() > DRAFT_MAX_ID_BYTES
            || draft.title.len() > DRAFT_MAX_TITLE_BYTES
            || draft.content.len() > DRAFT_MAX_CONTENT_BYTES
            || location_bytes > DRAFT_MAX_LOCATION_BYTES
            || draft
                .base_sha256
                .as_ref()
                .is_some_and(|hash| hash.len() > 64)
        {
            return Err(DraftStoreError::FieldTooLarge);
        }
        Ok(draft.draft_id.len()
            + draft.title.len()
            + draft.content.len()
            + location_bytes
            + draft.base_sha256.as_ref().map_or(0, String::len)
            + draft.updated_at_utc.len())
    }

    /// List all drafts belonging to a specific authenticated seat key.
    ///
    /// # Errors
    ///
    /// Returns [`DraftStoreError::StorageUnavailable`] on database or decoding failure.
    pub fn list(&self, principal: &str) -> Result<Vec<UserDraftProjection>, DraftStoreError> {
        let guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let now = OffsetDateTime::now_utc().unix_timestamp();
        guard
            .execute("DELETE FROM drafts WHERE expires_at <= ?1", [now])
            .map_err(|_| DraftStoreError::StorageUnavailable)?;
        let mut statement = guard
            .prepare("SELECT payload FROM drafts WHERE principal = ?1 ORDER BY draft_id")
            .map_err(|_| DraftStoreError::StorageUnavailable)?;
        let payloads = statement
            .query_map([principal], |row| row.get::<_, String>(0))
            .map_err(|_| DraftStoreError::StorageUnavailable)?;
        payloads
            .map(|payload| {
                payload
                    .map_err(|_| DraftStoreError::StorageUnavailable)
                    .and_then(|payload| {
                        serde_json::from_str(&payload)
                            .map_err(|_| DraftStoreError::StorageUnavailable)
                    })
            })
            .collect()
    }

    /// Save or overwrite a draft for a specific authenticated seat key.
    ///
    /// # Errors
    ///
    /// Returns [`DraftStoreError`] when a field, seat, or global resource limit is exceeded.
    pub fn save(
        &self,
        principal: &str,
        draft: &UserDraftProjection,
    ) -> Result<(), DraftStoreError> {
        let bytes = Self::draft_bytes(draft)?;
        let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let now = OffsetDateTime::now_utc();
        let tx = guard
            .transaction()
            .map_err(|_| DraftStoreError::StorageUnavailable)?;
        tx.execute(
            "DELETE FROM drafts WHERE expires_at <= ?1",
            [now.unix_timestamp()],
        )
        .map_err(|_| DraftStoreError::StorageUnavailable)?;
        let previous_bytes: Option<i64> = tx
            .query_row(
                "SELECT bytes FROM drafts WHERE principal=?1 AND draft_id=?2",
                params![principal, draft.draft_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| DraftStoreError::StorageUnavailable)?;
        let (seat_count, seat_bytes): (i64, i64) = tx
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(bytes),0) FROM drafts WHERE principal=?1",
                [principal],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| DraftStoreError::StorageUnavailable)?;
        let (global_count, global_bytes): (i64, i64) = tx
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(bytes),0) FROM drafts",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| DraftStoreError::StorageUnavailable)?;
        let is_new = previous_bytes.is_none();
        let seat_count_limit =
            i64::try_from(DRAFT_MAX_PER_SEAT).map_err(|_| DraftStoreError::StorageUnavailable)?;
        let seat_byte_limit = i64::try_from(DRAFT_MAX_BYTES_PER_SEAT)
            .map_err(|_| DraftStoreError::StorageUnavailable)?;
        let global_count_limit = i64::try_from(DRAFT_MAX_GLOBAL_COUNT)
            .map_err(|_| DraftStoreError::StorageUnavailable)?;
        let global_byte_limit = i64::try_from(DRAFT_MAX_GLOBAL_BYTES)
            .map_err(|_| DraftStoreError::StorageUnavailable)?;
        if is_new && seat_count >= seat_count_limit {
            return Err(DraftStoreError::SeatCountLimit);
        }
        let previous_bytes = previous_bytes.unwrap_or(0);
        let bytes = i64::try_from(bytes).map_err(|_| DraftStoreError::FieldTooLarge)?;
        if seat_bytes - previous_bytes + bytes > seat_byte_limit {
            return Err(DraftStoreError::SeatByteLimit);
        }
        if (is_new && global_count >= global_count_limit)
            || global_bytes - previous_bytes + bytes > global_byte_limit
        {
            return Err(DraftStoreError::GlobalLimit);
        }
        let payload =
            serde_json::to_string(&draft).map_err(|_| DraftStoreError::StorageUnavailable)?;
        tx.execute(
            "INSERT INTO drafts(principal,draft_id,payload,bytes,expires_at) VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(principal,draft_id) DO UPDATE SET payload=excluded.payload, bytes=excluded.bytes, expires_at=excluded.expires_at",
            params![principal, draft.draft_id, payload, bytes, (now + self.ttl).unix_timestamp()],
        ).map_err(|_| DraftStoreError::StorageUnavailable)?;
        tx.commit().map_err(|_| DraftStoreError::StorageUnavailable)
    }

    /// Remove a draft for a specific authenticated seat key by draft ID.
    ///
    /// # Errors
    ///
    /// Returns [`DraftStoreError::StorageUnavailable`] on database failure.
    pub fn delete(&self, principal: &str, draft_id: &str) -> Result<bool, DraftStoreError> {
        let guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        guard
            .execute(
                "DELETE FROM drafts WHERE principal=?1 AND draft_id=?2",
                params![principal, draft_id],
            )
            .map(|changed| changed != 0)
            .map_err(|_| DraftStoreError::StorageUnavailable)
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

fn draft_refusal(error: DraftStoreError) -> Refusal {
    let (status, code, retryable) = match error {
        DraftStoreError::FieldTooLarge | DraftStoreError::SeatByteLimit => {
            (StatusCode::PAYLOAD_TOO_LARGE, "draftTooLarge", false)
        }
        DraftStoreError::SeatCountLimit => (StatusCode::CONFLICT, "draftLimitReached", false),
        DraftStoreError::GlobalLimit => (
            StatusCode::SERVICE_UNAVAILABLE,
            "draftCapacityReached",
            true,
        ),
        DraftStoreError::StorageUnavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "draftStorageUnavailable",
            true,
        ),
    };
    (
        status,
        Json(ErrorBody {
            schema_version: WEB_SCHEMA_V1,
            error: code,
            retryable,
        }),
    )
}

fn authenticated_principal(state: &GatewayState, headers: &HeaderMap) -> Option<String> {
    if let Some(session) = state.session_for(headers) {
        return Some(format!("linux-account:{}", session.username));
    }
    if matches!(
        state.shell_seat(headers),
        Some(crate::shells::ShellOwner::LocalDesktop { .. })
    ) {
        return Some("local-desktop".to_string());
    }
    None
}

/// List all drafts for the authenticated seat.
///
/// # Errors
///
/// Returns a governed refusal when the request has no authenticated private seat.
pub async fn list_drafts_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<UserDraftListProjection>, Refusal> {
    let principal = authenticated_principal(&state, &headers).ok_or_else(no_seat)?;
    let drafts = state.drafts.list(&principal).map_err(draft_refusal)?;
    Ok(Json(UserDraftListProjection {
        schema_version: WEB_SCHEMA_V1,
        drafts,
    }))
}

/// Save or update a draft for the authenticated seat.
///
/// # Errors
///
/// Returns a governed refusal when the request has no seat or exceeds a draft-store limit.
pub async fn save_draft_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<UserDraftSaveRequest>,
) -> Result<Json<UserDraftProjection>, Refusal> {
    let principal = authenticated_principal(&state, &headers).ok_or_else(no_seat)?;
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

    state
        .drafts
        .save(&principal, &draft)
        .map_err(draft_refusal)?;
    Ok(Json(draft))
}

/// Delete a draft for the authenticated seat.
///
/// # Errors
///
/// Returns a governed refusal when the request has no authenticated private seat.
pub async fn delete_draft_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<UserDraftDeleteRequest>,
) -> Result<StatusCode, Refusal> {
    let principal = authenticated_principal(&state, &headers).ok_or_else(no_seat)?;
    state
        .drafts
        .delete(&principal, &payload.draft_id)
        .map_err(draft_refusal)?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(id: impl Into<String>, content: impl Into<String>) -> UserDraftProjection {
        UserDraftProjection {
            draft_id: id.into(),
            title: "Draft".into(),
            content: content.into(),
            base_location: None,
            base_sha256: None,
            updated_at_utc: "2026-08-28T00:00:00Z".into(),
        }
    }

    #[test]
    fn oversized_input_is_rejected_without_creating_a_seat_partition() {
        let store = UserDraftStore::new();
        let oversized = "x".repeat(DRAFT_MAX_CONTENT_BYTES + 1);

        assert_eq!(
            store.save("seat", &draft("one", oversized)),
            Err(DraftStoreError::FieldTooLarge)
        );
        assert!(store.list("seat").expect("list drafts").is_empty());
    }

    #[test]
    fn seat_count_is_bounded_but_existing_drafts_can_be_overwritten() {
        let store = UserDraftStore::new();
        for index in 0..DRAFT_MAX_PER_SEAT {
            store
                .save("seat", &draft(format!("draft-{index}"), "content"))
                .expect("draft within count limit");
        }

        assert_eq!(
            store.save("seat", &draft("one-too-many", "content")),
            Err(DraftStoreError::SeatCountLimit)
        );
        store
            .save("seat", &draft("draft-0", "updated"))
            .expect("overwrite does not consume another slot");
        assert_eq!(
            store.list("seat").expect("list drafts").len(),
            DRAFT_MAX_PER_SEAT
        );
    }

    #[test]
    fn expired_drafts_are_pruned_from_database() {
        let store = UserDraftStore::with_ttl(Duration::seconds(-1));
        store
            .save("seat", &draft("expired", "content"))
            .expect("save before expiry pruning");

        assert!(store.list("seat").expect("list drafts").is_empty());
    }

    #[test]
    fn drafts_survive_store_restart() {
        let directory = tempfile::tempdir().expect("private draft directory");
        let database = directory.path().join("drafts.sqlite3");
        {
            let store = UserDraftStore::open(&database).expect("open first store");
            store
                .save("linux-account:alice", &draft("recovery", "durable"))
                .expect("persist draft");
        }

        let reopened = UserDraftStore::open(&database).expect("reopen store");
        let drafts = reopened
            .list("linux-account:alice")
            .expect("read persisted drafts");
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].content, "durable");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&database)
                    .expect("database metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(
                std::fs::metadata(directory.path())
                    .expect("directory metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
        }
    }

    #[test]
    fn store_errors_map_to_stable_http_refusals() {
        let (status, Json(body)) = draft_refusal(DraftStoreError::GlobalLimit);
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body.error, "draftCapacityReached");
        assert!(body.retryable);

        let (status, Json(body)) = draft_refusal(DraftStoreError::SeatCountLimit);
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.error, "draftLimitReached");
        assert!(!body.retryable);
    }
}
