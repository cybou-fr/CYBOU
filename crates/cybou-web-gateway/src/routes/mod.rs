// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! HTTP route definitions and handlers for CYBOU Web Gateway.

pub mod auth;
pub mod disclosure;
pub mod events;
pub mod files;
pub mod shell;
pub mod snapshot;

use axum::http::StatusCode;

pub use auth::{login_handler, logout_handler, session_handler};
pub use disclosure::disclosure_handler;
pub use events::events_handler;
pub use files::{list_directory_handler, read_file_handler};
pub use shell::{shell_close_handler, shell_exec_handler};
pub use snapshot::{mind_handler, snapshot_handler};

/// 404 handler for unmatched API routes preventing SPA fallback on API endpoints.
pub async fn api_not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}
