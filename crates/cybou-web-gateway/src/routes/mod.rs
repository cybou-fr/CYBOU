// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! HTTP route definitions and handlers for CYBOU Web Gateway.

pub mod actions;
pub mod agents;
pub mod auth;
pub mod disclosure;
pub mod drafts;
pub mod events;
pub mod files;
pub mod host_files;
pub mod insight;
pub mod shell;
pub mod snapshot;

use axum::http::StatusCode;

pub use actions::{actions_handler, recent_actions_handler};
pub use agents::{agent_offers_handler, agents_handler, launch_agent_handler, stop_agent_handler};
pub use auth::{login_handler, logout_handler, session_handler};
pub use disclosure::disclosure_handler;
pub use drafts::{
    UserDraftStore, delete_draft_handler, draft_database_path, list_drafts_handler,
    save_draft_handler, validate_draft_database_isolation,
};
pub use events::events_handler;
pub use files::{
    create_file_handler, list_directory_handler, read_file_handler, write_file_handler,
};
pub use host_files::{list_host_directory_handler, read_host_file_handler};
pub use insight::insight_handler;
pub use shell::{shell_close_handler, shell_exec_handler};
pub use snapshot::{mind_handler, snapshot_handler};

/// 404 handler for unmatched API routes preventing SPA fallback on API endpoints.
pub async fn api_not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}
