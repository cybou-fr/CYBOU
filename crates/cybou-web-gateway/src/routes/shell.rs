// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Isolated shell command execution route.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use cybou_web_contracts::{ShellCloseRequest, ShellExecRequest, ShellExecResponse, WEB_SCHEMA_V1};
use time::OffsetDateTime;

use crate::state::{ErrorBody, GatewayState};

/// Execute a sandboxed shell command in the caller's own shell.
///
/// The shell a request drives is the one the caller named, within the seat they hold — a live
/// session, or the desktop. Entitlement and identity are the same question here, so it is asked
/// once: a caller who owns no shell is refused rather than falling back onto someone else's.
///
/// The instance is part of the identity. Two Shell cards in one session are two places a person is
/// standing, and a `cd` in one must not move the other; the card sends which one it is, because the
/// gateway has no other way to tell them apart.
///
/// # Errors
///
/// Returns `StatusCode::FORBIDDEN` if executed outside authenticated or local desktop context.
pub async fn shell_exec_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<ShellExecRequest>,
) -> Result<Json<ShellExecResponse>, (StatusCode, Json<ErrorBody>)> {
    let Some(owner) = state.shell_owner(&headers, payload.instance) else {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorBody {
                schema_version: WEB_SCHEMA_V1,
                error: "shellExecutionForbiddenInPublicPreview",
                retryable: false,
            }),
        ));
    };

    let shell = state.shells.for_owner(&owner, OffsetDateTime::now_utc());
    let mut engine = shell.lock().await;
    let output = engine.execute(&payload.command);

    Ok(Json(ShellExecResponse {
        schema_version: WEB_SCHEMA_V1,
        exit_code: output.exit_code,
        stdout: output.stdout,
        stderr: output.stderr,
        cwd: output.cwd,
    }))
}

/// End one of the caller's shells, because the card standing in it was closed.
///
/// Answers `OK` whether or not a shell was held: the caller is saying it is finished, and being
/// told it had already finished is not information it can act on. A caller who owns no shell at all
/// is refused, for the same reason execution is.
///
/// # Errors
///
/// Returns `StatusCode::FORBIDDEN` if the caller holds no seat.
pub async fn shell_close_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<ShellCloseRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let Some(owner) = state.shell_owner(&headers, payload.instance) else {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorBody {
                schema_version: WEB_SCHEMA_V1,
                error: "shellExecutionForbiddenInPublicPreview",
                retryable: false,
            }),
        ));
    };
    state.shells.end_one(&owner);
    Ok(StatusCode::OK)
}
