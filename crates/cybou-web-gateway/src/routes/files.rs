// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Reading the sandbox as structure rather than as terminal output.
//!
//! The File Manager used to ask the Shell for `ls -la` and parse the columns back into names,
//! kinds and sizes. That is a typed filesystem turned into text and then guessed at, and it failed
//! the way such loops always fail: the parser expected nine whitespace-separated fields, the
//! engine's own long format produced six, so every entry fell through both branches and the panel
//! showed an empty directory. Nothing reported an error, because from the parser's point of view
//! there was simply nothing there.
//!
//! These routes hand back what `cybou-jailfs` already established. They share the sandbox and the
//! entitlement boundary with the Shell, and they do not go through it.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use cybou_jailfs::JailError;
use cybou_web_contracts::{
    DirectoryEntryProjection, DirectoryListingProjection, FILE_LISTING_MAX_ENTRIES,
    FILE_READ_MAX_BYTES, FileContentProjection, FilePathRequest, WEB_SCHEMA_V1,
};

use crate::state::{ErrorBody, GatewayState};

/// One typed refusal.
type Refusal = (StatusCode, Json<ErrorBody>);

/// The refusal a caller who holds no seat receives.
///
/// The same boundary as Shell execution, and the same answer: reading the sandbox is a Body
/// capability, and a public preview holds none.
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

/// What a sandbox failure looks like at the boundary.
///
/// A path that left the sandbox is answered exactly as a path that does not exist. Telling a caller
/// which of its guesses escaped is telling it where the sandbox ends, and a caller entitled to read
/// inside it is not thereby entitled to map its edge.
fn boundary(error: &JailError) -> Refusal {
    let (status, code, retryable) = match error {
        JailError::NotFound(_) | JailError::TraversalAttempt(_) | JailError::InvalidPath(_) => {
            (StatusCode::NOT_FOUND, "pathNotReadable", false)
        }
        JailError::SizeLimitExceeded { .. } => {
            (StatusCode::PAYLOAD_TOO_LARGE, "fileTooLarge", false)
        }
        JailError::Io(_) => (StatusCode::BAD_GATEWAY, "sandboxUnreadable", true),
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

/// List one directory inside the sandbox.
///
/// # Errors
///
/// Refuses if the caller holds no seat, or the path names nothing readable.
pub async fn list_directory_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<FilePathRequest>,
) -> Result<Json<DirectoryListingProjection>, Refusal> {
    if state.shell_seat(&headers).is_none() {
        return Err(no_seat());
    }

    let entries = state
        .files
        .list_dir(&payload.path)
        .map_err(|error| boundary(&error))?;

    let total = u32::try_from(entries.len()).unwrap_or(u32::MAX);
    let truncated = entries.len() > FILE_LISTING_MAX_ENTRIES;

    Ok(Json(DirectoryListingProjection {
        schema_version: WEB_SCHEMA_V1,
        path: payload.path,
        entries: entries
            .into_iter()
            .take(FILE_LISTING_MAX_ENTRIES)
            .map(|entry| DirectoryEntryProjection {
                name: entry.name,
                is_dir: entry.is_dir,
                size_bytes: entry.size_bytes,
            })
            .collect(),
        total_entries: total,
        truncated,
    }))
}

/// Read one text file inside the sandbox.
///
/// # Errors
///
/// Refuses if the caller holds no seat, or the path names nothing readable within the byte budget.
pub async fn read_file_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<FilePathRequest>,
) -> Result<Json<FileContentProjection>, Refusal> {
    if state.shell_seat(&headers).is_none() {
        return Err(no_seat());
    }

    // The size is read before the bytes so a file too large to serve can still say how large it is.
    // Answering "cannot show this" without saying what was too big is a refusal a person cannot act
    // on.
    let size_bytes = state
        .files
        .resolve(&payload.path)
        .ok()
        .and_then(|resolved| std::fs::metadata(resolved).ok())
        .map_or(0, |metadata| metadata.len());

    let text = state
        .files
        .read_to_string(&payload.path, FILE_READ_MAX_BYTES)
        .map_err(|error| boundary(&error))?;

    Ok(Json(FileContentProjection {
        schema_version: WEB_SCHEMA_V1,
        path: payload.path,
        text,
        size_bytes,
    }))
}
