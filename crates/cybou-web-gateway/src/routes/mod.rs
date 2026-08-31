// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! HTTP route definitions and handlers for CYBOU Web Gateway.

pub mod actions;
pub mod agents;
pub mod auth;
pub mod cognitive;
pub mod disclosure;
pub mod drafts;
pub mod events;
pub mod files;
pub mod host_files;
pub mod insight;
pub mod learning;
pub mod meaning;
pub mod notifications;
pub mod operations;
pub mod personal;
pub mod snapshot;
pub mod system;
pub mod terminal;
pub mod workspace;

use axum::http::StatusCode;

pub use actions::{actions_handler, confirm_action_handler, recent_actions_handler};
pub use agents::{
    agent_offers_handler, agents_handler, capsule_action_handler, capsule_telemetry_handler,
    launch_agent_handler, stop_agent_handler,
};
pub use auth::{login_handler, logout_handler, session_handler};
pub use cognitive::{get_cognitive_graph, get_event_journal, query_cognitive_graph};
pub use disclosure::disclosure_handler;
pub use drafts::{
    UserDraftStore, delete_draft_handler, draft_database_path, list_drafts_handler,
    save_draft_handler, validate_draft_database_isolation,
};
pub use events::events_handler;
pub use files::{
    FileDownload, create_file_handler, download_file_handler, list_directory_handler,
    read_file_handler, upload_file_handler, write_file_handler,
};
pub use host_files::{
    copy_host_path_handler, create_host_directory_handler, create_host_file_handler,
    delete_host_path_handler, list_host_directory_handler, read_host_file_handler,
    rename_host_path_handler, write_host_file_handler,
};
pub use insight::insight_handler;
pub use learning::{
    evaluate_candidate_handler, get_artifacts_handler, get_candidates_handler,
    get_governance_scopes_handler, propose_candidate_handler, revoke_artifact_handler,
};
pub use meaning::{dialogue_memory_handler, interpret_handler};
pub use notifications::{dismiss_notifications, execute_notification_action, list_notifications};
pub use operations::{cancel_operation, get_operation, get_operation_logs, list_operations};
pub use personal::{
    create_calendar_event, create_contact, create_note, get_calendar, get_contacts, get_mail,
    get_notes, send_mail, update_note,
};
pub use snapshot::{mind_handler, snapshot_handler};
pub use system::{
    add_ssh_key, apply_system_updates, connect_network, create_snapshot, create_user,
    delete_ssh_key, execute_package_action, execute_service_action, get_backup_settings,
    get_network, get_packages, get_security_settings, get_storage, get_system_logs,
    get_system_monitor, get_system_updates, get_users_settings, list_processes, list_services,
    restore_archive, restore_snapshot, send_process_signal, trigger_backup, update_backup_schedule,
    update_security_policy,
};
pub use terminal::terminal_handler;
pub use workspace::{WorkspaceStore, get_layout_handler, save_layout_handler};

/// 404 handler for unmatched API routes preventing SPA fallback on API endpoints.
pub async fn api_not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}
