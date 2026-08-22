// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Isolated shell command execution route.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use cybou_web_contracts::{ShellExecRequest, ShellExecResponse, WEB_SCHEMA_V1};
use time::OffsetDateTime;

use crate::state::{ErrorBody, GatewayState};

/// Execute a sandboxed shell command in the caller's own shell.
///
/// The shell a request drives is the one belonging to whoever sent it — a live session, or the
/// desktop seat. Entitlement and identity are the same question here, so it is asked once: a
/// caller who owns no shell is refused rather than falling back onto someone else's.
///
/// # Errors
///
/// Returns `StatusCode::FORBIDDEN` if executed outside authenticated or local desktop context.
pub async fn shell_exec_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<ShellExecRequest>,
) -> Result<Json<ShellExecResponse>, (StatusCode, Json<ErrorBody>)> {
    let Some(owner) = state.shell_owner(&headers) else {
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
