// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Isolated shell command execution route.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use cybou_web_contracts::{
    SessionMode, ShellExecRequest, ShellExecResponse, WEB_SCHEMA_V1,
};

use crate::state::{ErrorBody, GatewayState};

/// Execute a sandboxed shell command.
///
/// # Errors
///
/// Returns `StatusCode::FORBIDDEN` if executed outside authenticated or local desktop context.
pub async fn shell_exec_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<ShellExecRequest>,
) -> Result<Json<ShellExecResponse>, (StatusCode, Json<ErrorBody>)> {
    let is_authenticated =
        state.session_for(&headers).is_some() || state.session.mode == SessionMode::LocalDesktop;
    if !is_authenticated {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorBody {
                schema_version: WEB_SCHEMA_V1,
                error: "shellExecutionForbiddenInPublicPreview",
                retryable: false,
            }),
        ));
    }

    let mut engine = state.shell.lock().await;
    let output = engine.execute(&payload.command);

    Ok(Json(ShellExecResponse {
        schema_version: WEB_SCHEMA_V1,
        exit_code: output.exit_code,
        stdout: output.stdout,
        stderr: output.stderr,
        cwd: output.cwd,
    }))
}
