// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Where a person's desktop arrangement lives when it is not in one browser.
//!
//! The arrangement was kept in `localStorage` and nowhere else. That is per browser, per profile
//! and per machine: signing in from a second computer gave a stranger's desktop, and clearing site
//! data threw the arrangement away with the cookies. For a desktop whose whole argument is that it
//! follows you to whatever screen you are at, that is the wrong place for it.
//!
//! The layout is stored opaquely, as the string the browser wrote. The gateway has no business
//! understanding a card's geometry: it would be a second implementation of a schema the frontend
//! owns, and it would go stale the first time a card gained a field. What the gateway does own is
//! the two things that make it safe to keep — that it belongs to one authenticated seat and to no
//! other, and that it cannot grow without bound.
//!
//! It shares the private database the drafts use. Same isolation rule, same lifetime, same seat
//! scoping, and one file to keep outside the sandbox instead of two.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use cybou_web_contracts::{DesktopLayoutProjection, DesktopLayoutSaveRequest, WEB_SCHEMA_V1};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::state::{ErrorBody, GatewayState};

/// The largest arrangement that will be stored.
///
/// A desktop is a few dozen cards with a rectangle and a few flags each; a hundred kilobytes is
/// already far more than one can be. The cap is here because this is a browser-supplied document
/// and the store is durable: without it, one seat could write until the disk is the limit.
const LAYOUT_MAX_BYTES: usize = 256 * 1024;

/// Why an arrangement could not be read or written.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceStoreError {
    /// The database could not be opened, read or written.
    StorageUnavailable,
    /// The arrangement is larger than [`LAYOUT_MAX_BYTES`].
    LayoutTooLarge,
}

/// One person's desktop arrangement, kept per authenticated seat.
pub struct WorkspaceStore {
    inner: Arc<Mutex<Connection>>,
}

impl WorkspaceStore {
    /// An arrangement store that lives only as long as the process. Used by tests.
    #[must_use]
    pub fn new() -> Self {
        Self::from_connection(
            Connection::open_in_memory().expect("initialize in-memory workspace DB"),
        )
        .expect("initialize workspace schema")
    }

    /// Open or create the durable private arrangement table.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceStoreError::StorageUnavailable`] when the database cannot be opened or
    /// the schema cannot be established.
    pub fn open(path: &Path) -> Result<Self, WorkspaceStoreError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| WorkspaceStoreError::StorageUnavailable)?;
        }
        let connection =
            Connection::open(path).map_err(|_| WorkspaceStoreError::StorageUnavailable)?;
        Self::from_connection(connection)
    }

    fn from_connection(connection: Connection) -> Result<Self, WorkspaceStoreError> {
        connection
            .execute_batch(
                "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
                 CREATE TABLE IF NOT EXISTS desktop_layouts (
                   principal TEXT PRIMARY KEY, payload TEXT NOT NULL,
                   bytes INTEGER NOT NULL, updated_at INTEGER NOT NULL
                 );",
            )
            .map_err(|_| WorkspaceStoreError::StorageUnavailable)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(connection)),
        })
    }

    /// The arrangement this seat last saved, and when, or `None` if it has never saved one.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceStoreError::StorageUnavailable`] when the database cannot be read.
    pub fn load(&self, principal: &str) -> Result<Option<(String, i64)>, WorkspaceStoreError> {
        let connection = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        connection
            .query_row(
                "SELECT payload, updated_at FROM desktop_layouts WHERE principal = ?1",
                params![principal],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()
            .map_err(|_| WorkspaceStoreError::StorageUnavailable)
    }

    /// Replace this seat's arrangement.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceStoreError::LayoutTooLarge`] past the size cap, or
    /// [`WorkspaceStoreError::StorageUnavailable`] when the database cannot be written.
    pub fn save(&self, principal: &str, layout: &str) -> Result<i64, WorkspaceStoreError> {
        if layout.len() > LAYOUT_MAX_BYTES {
            return Err(WorkspaceStoreError::LayoutTooLarge);
        }
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let connection = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        connection
            .execute(
                "INSERT INTO desktop_layouts (principal, payload, bytes, updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(principal) DO UPDATE SET
                   payload = excluded.payload, bytes = excluded.bytes,
                   updated_at = excluded.updated_at",
                params![
                    principal,
                    layout,
                    i64::try_from(layout.len()).unwrap_or(i64::MAX),
                    now
                ],
            )
            .map_err(|_| WorkspaceStoreError::StorageUnavailable)?;
        Ok(now)
    }
}

impl Default for WorkspaceStore {
    fn default() -> Self {
        Self::new()
    }
}

type Refusal = (StatusCode, Json<ErrorBody>);

fn no_seat() -> Refusal {
    // An arrangement belongs to an account rather than to a browser, which is the whole reason it
    // is stored here, so a request without a seat has nothing to be given.
    (
        StatusCode::FORBIDDEN,
        Json(ErrorBody {
            schema_version: WEB_SCHEMA_V1,
            error: "workspaceRequiresSeat",
            retryable: false,
        }),
    )
}

fn store_refusal(error: WorkspaceStoreError) -> Refusal {
    let (status, code, retryable) = match error {
        WorkspaceStoreError::LayoutTooLarge => (
            StatusCode::PAYLOAD_TOO_LARGE,
            "workspaceLayoutTooLarge",
            false,
        ),
        WorkspaceStoreError::StorageUnavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "workspaceStorageUnavailable",
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

fn as_rfc3339(seconds: i64) -> String {
    OffsetDateTime::from_unix_timestamp(seconds)
        .ok()
        .and_then(|instant| instant.format(&Rfc3339).ok())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_owned())
}

/// Read this seat's saved desktop arrangement.
///
/// # Errors
///
/// Returns a governed refusal when the request has no authenticated seat, or when the store cannot
/// be read. A seat that has never saved one is not an error: it is an empty arrangement, and the
/// browser is expected to keep whatever it already had.
pub async fn get_layout_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<DesktopLayoutProjection>, Refusal> {
    let principal = state
        .authenticated_principal(&headers)
        .ok_or_else(no_seat)?;
    let workspace = state.workspace.clone();
    let stored = tokio::task::spawn_blocking(move || workspace.load(&principal))
        .await
        .map_err(|_| store_refusal(WorkspaceStoreError::StorageUnavailable))?
        .map_err(store_refusal)?;
    let (layout, updated_at_utc) = match stored {
        Some((payload, updated_at)) => (Some(payload), Some(as_rfc3339(updated_at))),
        None => (None, None),
    };
    Ok(Json(DesktopLayoutProjection {
        schema_version: WEB_SCHEMA_V1,
        layout,
        updated_at_utc,
    }))
}

/// Replace this seat's saved desktop arrangement.
///
/// # Errors
///
/// Returns a governed refusal when the request has no authenticated seat, when the arrangement is
/// past the size cap, or when the store cannot be written.
pub async fn save_layout_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<DesktopLayoutSaveRequest>,
) -> Result<Json<DesktopLayoutProjection>, Refusal> {
    let principal = state
        .authenticated_principal(&headers)
        .ok_or_else(no_seat)?;
    let workspace = state.workspace.clone();
    let layout = payload.layout;
    let stored = layout.clone();
    let updated_at = tokio::task::spawn_blocking(move || workspace.save(&principal, &stored))
        .await
        .map_err(|_| store_refusal(WorkspaceStoreError::StorageUnavailable))?
        .map_err(store_refusal)?;
    Ok(Json(DesktopLayoutProjection {
        schema_version: WEB_SCHEMA_V1,
        layout: Some(layout),
        updated_at_utc: Some(as_rfc3339(updated_at)),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_arrangement_comes_back_to_the_seat_that_saved_it_and_to_no_other() {
        let store = WorkspaceStore::new();
        store.save("alice", "{\"cards\":[]}").expect("save");

        assert_eq!(
            store
                .load("alice")
                .expect("load")
                .map(|(payload, _)| payload),
            Some("{\"cards\":[]}".to_owned())
        );
        // The point of storing this per seat rather than per browser is that it is per seat.
        assert_eq!(store.load("bob").expect("load"), None);
    }

    #[test]
    fn saving_again_replaces_rather_than_accumulates() {
        // A desktop is saved on every drag. Rows per drag would be a disk filling up in the shape
        // of somebody tidying their workspace.
        let store = WorkspaceStore::new();
        store.save("alice", "first").expect("save");
        store.save("alice", "second").expect("save");
        assert_eq!(
            store
                .load("alice")
                .expect("load")
                .map(|(payload, _)| payload),
            Some("second".to_owned())
        );
    }

    #[test]
    fn an_arrangement_past_the_cap_is_refused_rather_than_stored() {
        let store = WorkspaceStore::new();
        let oversized = "x".repeat(LAYOUT_MAX_BYTES + 1);
        assert_eq!(
            store.save("alice", &oversized),
            Err(WorkspaceStoreError::LayoutTooLarge)
        );
        assert_eq!(store.load("alice").expect("load"), None);
    }

    #[test]
    fn a_seat_that_never_saved_has_no_arrangement_rather_than_an_empty_one() {
        // The browser keeps what it already had in that case. An empty string here would arrive as
        // a desktop with no cards on it, which is not what "I have not saved yet" means.
        let store = WorkspaceStore::new();
        assert_eq!(store.load("nobody").expect("load"), None);
    }
}
