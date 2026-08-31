// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! HTTP endpoints for querying system services, processes, resource telemetry, and logs.

use axum::{
    Json,
    extract::{Query, State},
};
use cybou_web_contracts::{
    AddSshKeyRequest, ApplyUpdatesRequest, BackupArchiveRecord, BackupScheduleRecord,
    BackupSettingsProjection, CreateSnapshotRequest, CreateUserRequest, DeleteSshKeyRequest,
    NetworkConnectRequest, NetworkProjection, PackageActionRequest, PackagesProjection,
    ProcessSignalRequest, ProcessesListProjection, RestoreArchiveRequest, RestoreSnapshotRequest,
    SecurityPolicyRecord, SecuritySettingsProjection, ServiceActionRequest, ServicesListProjection,
    SnapshotRecord, SshKeyRecord, StorageProjection, SystemLogsProjection, SystemLogsQueryRequest,
    SystemMonitorProjection, SystemUpdatesProjection, TriggerBackupRequest,
    UpdateBackupScheduleRequest, UpdateSecurityPolicyRequest, UserAccountRecord,
    UsersSettingsProjection,
};

use crate::state::{GatewayError, GatewayState};

/// GET `/api/v1/system/services`
pub async fn list_services(
    State(state): State<GatewayState>,
) -> Result<Json<ServicesListProjection>, GatewayError> {
    Ok(Json(state.system.list_services()))
}

/// POST `/api/v1/system/services/action`
pub async fn execute_service_action(
    State(state): State<GatewayState>,
    Json(request): Json<ServiceActionRequest>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let outcome = state
        .system
        .execute_service_action(&request.name, request.action)?;
    Ok(Json(serde_json::json!({ "outcome": outcome })))
}

/// GET `/api/v1/system/processes`
pub async fn list_processes(
    State(state): State<GatewayState>,
) -> Result<Json<ProcessesListProjection>, GatewayError> {
    Ok(Json(state.system.list_processes()))
}

/// POST `/api/v1/system/processes/signal`
pub async fn send_process_signal(
    State(state): State<GatewayState>,
    Json(request): Json<ProcessSignalRequest>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let outcome = state
        .system
        .send_process_signal(request.pid, request.signal)?;
    Ok(Json(serde_json::json!({ "outcome": outcome })))
}

/// GET `/api/v1/system/monitor`
pub async fn get_system_monitor(
    State(state): State<GatewayState>,
) -> Result<Json<SystemMonitorProjection>, GatewayError> {
    Ok(Json(state.system.get_monitor()))
}

/// GET `/api/v1/system/logs`
pub async fn get_system_logs(
    State(state): State<GatewayState>,
    Query(query): Query<SystemLogsQueryRequest>,
) -> Result<Json<SystemLogsProjection>, GatewayError> {
    Ok(Json(state.system.get_logs(&query)?))
}

/// GET `/api/v1/system/storage`
pub async fn get_storage(
    State(state): State<GatewayState>,
) -> Result<Json<StorageProjection>, GatewayError> {
    Ok(Json(state.system.get_storage()))
}

/// POST `/api/v1/system/storage/snapshots`
pub async fn create_snapshot(
    State(state): State<GatewayState>,
    Json(request): Json<CreateSnapshotRequest>,
) -> Result<Json<SnapshotRecord>, GatewayError> {
    let record =
        state
            .system
            .create_snapshot(&request.subvolume_path, &request.name, request.readonly)?;
    Ok(Json(record))
}

/// POST `/api/v1/system/storage/restore`
pub async fn restore_snapshot(
    State(state): State<GatewayState>,
    Json(request): Json<RestoreSnapshotRequest>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let outcome = state.system.restore_snapshot(&request.snapshot_id)?;
    Ok(Json(serde_json::json!({ "outcome": outcome })))
}

/// GET `/api/v1/system/network`
pub async fn get_network(
    State(state): State<GatewayState>,
) -> Result<Json<NetworkProjection>, GatewayError> {
    Ok(Json(state.system.get_network()))
}

/// POST `/api/v1/system/network/connect`
pub async fn connect_network(
    State(state): State<GatewayState>,
    Json(request): Json<NetworkConnectRequest>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let outcome = state
        .system
        .connect_network(&request.connection_id, request.activate)?;
    Ok(Json(serde_json::json!({ "outcome": outcome })))
}

/// GET `/api/v1/system/packages`
pub async fn get_packages(
    State(state): State<GatewayState>,
) -> Result<Json<PackagesProjection>, GatewayError> {
    Ok(Json(state.system.get_packages()))
}

/// POST `/api/v1/system/packages/action`
pub async fn execute_package_action(
    State(state): State<GatewayState>,
    Json(request): Json<PackageActionRequest>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let outcome = state
        .system
        .execute_package_action(&request.name, request.action)?;
    Ok(Json(serde_json::json!({ "outcome": outcome })))
}

/// GET `/api/v1/system/updates`
pub async fn get_system_updates(
    State(state): State<GatewayState>,
) -> Result<Json<SystemUpdatesProjection>, GatewayError> {
    Ok(Json(state.system.get_system_updates()))
}

/// POST `/api/v1/system/updates/apply`
pub async fn apply_system_updates(
    State(state): State<GatewayState>,
    Json(request): Json<ApplyUpdatesRequest>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let outcome = state.system.apply_system_updates(request.package_names)?;
    Ok(Json(serde_json::json!({ "outcome": outcome })))
}

/// GET `/api/v1/system/users`
pub async fn get_users_settings(
    State(state): State<GatewayState>,
) -> Result<Json<UsersSettingsProjection>, GatewayError> {
    Ok(Json(state.system.get_users_settings()))
}

/// POST `/api/v1/system/users`
pub async fn create_user(
    State(state): State<GatewayState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<Json<UserAccountRecord>, GatewayError> {
    let record =
        state
            .system
            .create_user(&request.username, &request.full_name, request.is_admin)?;
    Ok(Json(record))
}

/// POST `/api/v1/system/users/ssh-keys`
pub async fn add_ssh_key(
    State(state): State<GatewayState>,
    Json(request): Json<AddSshKeyRequest>,
) -> Result<Json<SshKeyRecord>, GatewayError> {
    let record = state
        .system
        .add_ssh_key(&request.name, &request.public_key)?;
    Ok(Json(record))
}

/// POST `/api/v1/system/users/ssh-keys/delete`
pub async fn delete_ssh_key(
    State(state): State<GatewayState>,
    Json(request): Json<DeleteSshKeyRequest>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let outcome = state.system.delete_ssh_key(&request.key_id)?;
    Ok(Json(serde_json::json!({ "outcome": outcome })))
}

/// GET `/api/v1/system/security`
pub async fn get_security_settings(
    State(state): State<GatewayState>,
) -> Result<Json<SecuritySettingsProjection>, GatewayError> {
    Ok(Json(state.system.get_security_settings()))
}

/// POST `/api/v1/system/security/policy`
pub async fn update_security_policy(
    State(state): State<GatewayState>,
    Json(request): Json<UpdateSecurityPolicyRequest>,
) -> Result<Json<SecurityPolicyRecord>, GatewayError> {
    let record = state.system.update_security_policy(request)?;
    Ok(Json(record))
}

/// GET `/api/v1/system/backup`
pub async fn get_backup_settings(
    State(state): State<GatewayState>,
) -> Result<Json<BackupSettingsProjection>, GatewayError> {
    Ok(Json(state.system.get_backup_settings()))
}

/// POST `/api/v1/system/backup/trigger`
pub async fn trigger_backup(
    State(state): State<GatewayState>,
    Json(request): Json<TriggerBackupRequest>,
) -> Result<Json<BackupArchiveRecord>, GatewayError> {
    let record = state.system.trigger_backup(request.name)?;
    Ok(Json(record))
}

/// POST `/api/v1/system/backup/restore`
pub async fn restore_archive(
    State(state): State<GatewayState>,
    Json(request): Json<RestoreArchiveRequest>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let outcome = state
        .system
        .restore_archive(&request.archive_id, request.target_path)?;
    Ok(Json(serde_json::json!({ "outcome": outcome })))
}

/// POST `/api/v1/system/backup/schedule`
pub async fn update_backup_schedule(
    State(state): State<GatewayState>,
    Json(request): Json<UpdateBackupScheduleRequest>,
) -> Result<Json<BackupScheduleRecord>, GatewayError> {
    let record = state.system.update_backup_schedule(request)?;
    Ok(Json(record))
}

#[cfg(test)]
mod tests {
    use crate::system_hub::SystemHub;
    use cybou_protocol::system::{PackageActionKind, ProcessSignal, ServiceAction};
    use cybou_web_contracts::{UpdateBackupScheduleRequest, UpdateSecurityPolicyRequest};

    #[test]
    fn system_hub_manages_services_and_processes() {
        let hub = SystemHub::new();
        let svcs = hub.list_services();
        assert!(!svcs.services.is_empty());

        // Privileged mutations must fail-closed requiring Action1
        let restart_res =
            hub.execute_service_action("cybou-web-gateway.service", ServiceAction::Restart);
        assert!(restart_res.is_err());

        let procs = hub.list_processes();
        assert_eq!(procs.schema_version, cybou_web_contracts::WEB_SCHEMA_V1);

        let signal_res = hub.send_process_signal(1024, ProcessSignal::Terminate);
        assert!(signal_res.is_err());

        let mon = hub.get_monitor();
        assert_eq!(mon.schema_version, cybou_web_contracts::WEB_SCHEMA_V1);

        let storage = hub.get_storage();
        assert_eq!(storage.schema_version, cybou_web_contracts::WEB_SCHEMA_V1);
        let snap = hub.create_snapshot("@home", "test-backup", true);
        assert!(snap.is_err());

        let network = hub.get_network();
        assert_eq!(network.schema_version, cybou_web_contracts::WEB_SCHEMA_V1);
        let conn_res = hub.connect_network("conn-wg0", true);
        assert!(conn_res.is_err());

        let pkgs = hub.get_packages();
        assert_eq!(pkgs.schema_version, cybou_web_contracts::WEB_SCHEMA_V1);
        let pkg_res = hub.execute_package_action("borgbackup", PackageActionKind::Install);
        assert!(pkg_res.is_err());

        let updates = hub.get_system_updates();
        assert_eq!(updates.schema_version, cybou_web_contracts::WEB_SCHEMA_V1);
        let update_res = hub.apply_system_updates(None);
        assert!(update_res.is_err());

        let users = hub.get_users_settings();
        assert_eq!(users.schema_version, cybou_web_contracts::WEB_SCHEMA_V1);
        let new_user = hub.create_user("bob", "Bob Smith", false);
        assert!(new_user.is_err());

        let sec = hub.get_security_settings();
        assert!(sec.policy.landlock_enabled);
        let update_sec = hub.update_security_policy(UpdateSecurityPolicyRequest {
            landlock_enabled: true,
            bubblewrap_enabled: true,
            apparmor_enforcing: true,
            seccomp_strict: true,
            egress_firewall_strict: true,
        });
        assert!(update_sec.is_err());

        let backup = hub.get_backup_settings();
        assert_eq!(backup.repository.destination, "/var/lib/cybou/backup-vault");
        let trig = hub.trigger_backup(None);
        assert!(trig.is_err());
        let sched = hub.update_backup_schedule(UpdateBackupScheduleRequest {
            enabled: true,
            frequency: "daily".to_owned(),
            retention_daily: 7,
            retention_weekly: 4,
            retention_monthly: 6,
        });
        assert!(sched.is_err());
    }
}
