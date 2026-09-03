// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Authenticated host-user filesystem boundary.
//!
//! The gateway deliberately owns no implementation: it runs as the `cybou` service user and may
//! not treat successful PAM authentication as permission to perform I/O under that service UID.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use cybou_protocol::LocationRef;
use cybou_web_contracts::{
    FileContentProjection, FilePathRequest, HostDirectoryListingProjection, WEB_SCHEMA_V1,
};

use crate::state::{ErrorBody, GatewayState};

type Refusal = (StatusCode, Json<ErrorBody>);

fn unavailable() -> Refusal {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorBody {
            schema_version: WEB_SCHEMA_V1,
            error: "hostUserFilesystemUnavailable",
            retryable: true,
            detail: None,
        }),
    )
}

fn invalid_owner_projection() -> Refusal {
    (
        StatusCode::BAD_GATEWAY,
        Json(ErrorBody {
            schema_version: WEB_SCHEMA_V1,
            error: "invalidHostUserFilesystemProjection",
            retryable: false,
            detail: None,
        }),
    )
}

fn is_matching_host_location(location: &LocationRef, requested_path: &str) -> bool {
    matches!(location, LocationRef::HostUserPath(path) if path == requested_path)
}

fn is_inside_home(path: &str, home: &str) -> bool {
    use std::path::Component;

    let path = std::path::Path::new(path);
    let home = std::path::Path::new(home);
    let is_clean = |candidate: &std::path::Path| {
        candidate
            .components()
            .all(|component| !matches!(component, Component::ParentDir | Component::CurDir))
    };
    path.is_absolute()
        && home.is_absolute()
        && is_clean(path)
        && is_clean(home)
        && path.starts_with(home)
}

/// List a directory through the per-user filesystem owner.
///
/// # Errors
///
/// Returns a typed refusal when authentication, confinement, transport, or projection validation
/// fails.
pub async fn list_host_directory_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<FilePathRequest>,
) -> Result<Json<HostDirectoryListingProjection>, Refusal> {
    let session = state
        .session_for(&headers)
        .ok_or_else(GatewayState::sign_in_required)?;
    let (Some(uid), Some(home)) = (session.uid, session.home.as_deref()) else {
        return Err(unavailable());
    };
    if !is_inside_home(&payload.path, home) {
        return Err(invalid_owner_projection());
    }
    let source = state.host_user_files.as_ref().ok_or_else(unavailable)?;
    let projection = source
        .list_directory(uid, home, &payload.path)
        .await
        .map_err(|_| unavailable())?;
    if !is_matching_host_location(&projection.location, &payload.path) {
        return Err(invalid_owner_projection());
    }
    Ok(Json(projection))
}

/// Read a file through the per-user filesystem owner and verify its authority-domain claim.
///
/// # Errors
///
/// Returns a typed refusal when authentication, confinement, transport, or projection validation
/// fails.
pub async fn read_host_file_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<FilePathRequest>,
) -> Result<Json<FileContentProjection>, Refusal> {
    let session = state
        .session_for(&headers)
        .ok_or_else(GatewayState::sign_in_required)?;
    let (Some(uid), Some(home)) = (session.uid, session.home.as_deref()) else {
        return Err(unavailable());
    };
    if !is_inside_home(&payload.path, home) {
        return Err(invalid_owner_projection());
    }
    let source = state.host_user_files.as_ref().ok_or_else(unavailable)?;
    let projection = source
        .read_file(uid, home, &payload.path)
        .await
        .map_err(|_| unavailable())?;
    if !is_matching_host_location(&projection.location, &payload.path) {
        return Err(invalid_owner_projection());
    }
    Ok(Json(projection))
}

/// Conditionally write an existing host file.
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn write_host_file_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<cybou_web_contracts::HostFileWriteRequest>,
) -> Result<Json<cybou_web_contracts::FileWriteProjection>, Refusal> {
    let session = state
        .session_for(&headers)
        .ok_or_else(GatewayState::sign_in_required)?;
    let (Some(uid), Some(home)) = (session.uid, session.home.as_deref()) else {
        return Err(unavailable());
    };
    if !is_inside_home(&payload.path, home) {
        return Err(invalid_owner_projection());
    }
    let source = state.host_user_files.as_ref().ok_or_else(unavailable)?;
    let projection = source
        .write_file(
            uid,
            home,
            &payload.path,
            payload.expected_sha256,
            &payload.text,
        )
        .await
        .map_err(|_| unavailable())?;
    if !is_matching_host_location(&projection.location, &payload.path) {
        return Err(invalid_owner_projection());
    }
    Ok(Json(projection))
}

/// Create a new file in the user's home authority domain.
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn create_host_file_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<cybou_web_contracts::HostFileCreateRequest>,
) -> Result<Json<cybou_web_contracts::FileWriteProjection>, Refusal> {
    let session = state
        .session_for(&headers)
        .ok_or_else(GatewayState::sign_in_required)?;
    let (Some(uid), Some(home)) = (session.uid, session.home.as_deref()) else {
        return Err(unavailable());
    };
    if !is_inside_home(&payload.path, home) {
        return Err(invalid_owner_projection());
    }
    let source = state.host_user_files.as_ref().ok_or_else(unavailable)?;
    let projection = source
        .create_file(uid, home, &payload.path, &payload.text, payload.exclusive)
        .await
        .map_err(|_| unavailable())?;
    if !is_matching_host_location(&projection.location, &payload.path) {
        return Err(invalid_owner_projection());
    }
    Ok(Json(projection))
}

/// Create a directory in the user's home authority domain.
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn create_host_directory_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<cybou_web_contracts::HostDirectoryCreateRequest>,
) -> Result<StatusCode, Refusal> {
    let session = state
        .session_for(&headers)
        .ok_or_else(GatewayState::sign_in_required)?;
    let (Some(uid), Some(home)) = (session.uid, session.home.as_deref()) else {
        return Err(unavailable());
    };
    if !is_inside_home(&payload.path, home) {
        return Err(invalid_owner_projection());
    }
    let source = state.host_user_files.as_ref().ok_or_else(unavailable)?;
    source
        .create_directory(uid, home, &payload.path, payload.recursive)
        .await
        .map_err(|_| unavailable())?;
    Ok(StatusCode::OK)
}

/// Rename or move a path in the user's home authority domain.
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn rename_host_path_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<cybou_web_contracts::HostPathRenameRequest>,
) -> Result<StatusCode, Refusal> {
    let session = state
        .session_for(&headers)
        .ok_or_else(GatewayState::sign_in_required)?;
    let (Some(uid), Some(home)) = (session.uid, session.home.as_deref()) else {
        return Err(unavailable());
    };
    if !is_inside_home(&payload.from_path, home) || !is_inside_home(&payload.to_path, home) {
        return Err(invalid_owner_projection());
    }
    let source = state.host_user_files.as_ref().ok_or_else(unavailable)?;
    source
        .rename_path(uid, home, &payload.from_path, &payload.to_path)
        .await
        .map_err(|_| unavailable())?;
    Ok(StatusCode::OK)
}

/// Delete a path in the user's home authority domain.
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn delete_host_path_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<cybou_web_contracts::HostPathDeleteRequest>,
) -> Result<StatusCode, Refusal> {
    let session = state
        .session_for(&headers)
        .ok_or_else(GatewayState::sign_in_required)?;
    let (Some(uid), Some(home)) = (session.uid, session.home.as_deref()) else {
        return Err(unavailable());
    };
    if !is_inside_home(&payload.path, home) {
        return Err(invalid_owner_projection());
    }
    let source = state.host_user_files.as_ref().ok_or_else(unavailable)?;
    source
        .delete_path(uid, home, &payload.path, payload.recursive)
        .await
        .map_err(|_| unavailable())?;
    Ok(StatusCode::OK)
}

/// Copy a path in the user's home authority domain.
///
/// # Errors
///
/// Reports the owner unavailable when it cannot be read.
pub async fn copy_host_path_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<cybou_web_contracts::HostPathCopyRequest>,
) -> Result<StatusCode, Refusal> {
    let session = state
        .session_for(&headers)
        .ok_or_else(GatewayState::sign_in_required)?;
    let (Some(uid), Some(home)) = (session.uid, session.home.as_deref()) else {
        return Err(unavailable());
    };
    if !is_inside_home(&payload.from_path, home) || !is_inside_home(&payload.to_path, home) {
        return Err(invalid_owner_projection());
    }
    let source = state.host_user_files.as_ref().ok_or_else(unavailable)?;
    source
        .copy_path(uid, home, &payload.from_path, &payload.to_path)
        .await
        .map_err(|_| unavailable())?;
    Ok(StatusCode::OK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_absent_user_owner_is_explicitly_unavailable() {
        let (status, Json(body)) = unavailable();
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body.error, "hostUserFilesystemUnavailable");
        assert!(body.retryable);
    }

    #[test]
    fn an_owner_cannot_change_the_domain_or_requested_identity() {
        assert!(is_matching_host_location(
            &LocationRef::HostUserPath("/home/alice/notes.txt".into()),
            "/home/alice/notes.txt"
        ));
        assert!(!is_matching_host_location(
            &LocationRef::HostUserPath("/home/alice/other.txt".into()),
            "/home/alice/notes.txt"
        ));
        assert!(!is_matching_host_location(
            &LocationRef::SafeShellJail {
                session_id: "seat".into(),
                path: "/home/alice/notes.txt".into(),
            },
            "/home/alice/notes.txt"
        ));
    }

    #[test]
    fn host_paths_are_confined_to_the_authenticated_home() {
        assert!(is_inside_home("/home/alice", "/home/alice"));
        assert!(is_inside_home("/home/alice/notes.txt", "/home/alice"));
        assert!(!is_inside_home("/home/alice2/notes.txt", "/home/alice"));
        assert!(!is_inside_home(
            "/home/alice/../bob/notes.txt",
            "/home/alice"
        ));
        assert!(!is_inside_home("/etc/passwd", "/home/alice"));
        assert!(!is_inside_home("notes.txt", "/home/alice"));
    }
}
