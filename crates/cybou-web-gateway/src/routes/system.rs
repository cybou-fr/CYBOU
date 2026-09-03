// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! HTTP endpoints for querying system services, processes, resource telemetry, and logs.

use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};

use crate::system_hub::SystemHub;
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
///
/// # Errors
///
/// Refuses when the request holds no authenticated seat, and reports unavailable when the owner cannot be read.
pub async fn list_services(
    State(state): State<GatewayState>,
) -> Result<Json<ServicesListProjection>, GatewayError> {
    Ok(Json(state.system.list_services()))
}

/// POST `/api/v1/system/services/action`
///
/// # Errors
///
/// Refuses when the request holds no authenticated seat, and reports unavailable when the owner cannot be read.
pub async fn execute_service_action(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<ServiceActionRequest>,
) -> Result<Json<cybou_web_contracts::ActionRecordProjection>, GatewayError> {
    // The seat, which is the one thing neither `Action1` nor the executor can establish for
    // itself. Everything else on this path — whether the operation exists, whether it is
    // forbidden, whether a permit follows — belongs to `Action1` (ADR-0048).
    let seat = state
        .authenticated_principal(&headers)
        .ok_or(GatewayError::Refused)?;
    let verb = SystemHub::verb_for(request.action)?;

    state
        .presence
        .request_action(verb, &SystemHub::service_target(&request.name), &seat)
        .await
        .map(Json)
        .ok_or(GatewayError::Unavailable)
}

/// GET `/api/v1/system/processes`
///
/// # Errors
///
/// Refuses when the request holds no authenticated seat, and reports unavailable when the owner cannot be read.
pub async fn list_processes(
    State(state): State<GatewayState>,
) -> Result<Json<ProcessesListProjection>, GatewayError> {
    Ok(Json(state.system.list_processes()))
}

/// POST `/api/v1/system/processes/signal`
///
/// Signalling a process is an authorized action like any other, and this route does the one part
/// of it that only the gateway can do: establish who is asking, and that the process they named is
/// theirs. Everything after that — whether the verb exists, what it costs, whether a permit
/// follows — belongs to `Action1` (ADR-0048).
///
/// # Errors
///
/// Refuses when the request holds no authenticated seat, and reports unavailable when the owner cannot be read.
pub async fn send_process_signal(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<ProcessSignalRequest>,
) -> Result<Json<cybou_web_contracts::ActionRecordProjection>, GatewayError> {
    let seat = state
        .authenticated_principal(&headers)
        .ok_or(GatewayError::Refused)?;
    let owner_uid = SystemHub::signalling_seat_owns(&seat, request.pid)?;
    let verb = SystemHub::verb_for_signal(request.signal);

    state
        .presence
        .request_action(
            verb,
            &SystemHub::process_target(owner_uid, request.pid),
            &seat,
        )
        .await
        .map(Json)
        .ok_or(GatewayError::Unavailable)
}

/// GET `/api/v1/system/monitor`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn get_system_monitor(
    State(state): State<GatewayState>,
) -> Result<Json<SystemMonitorProjection>, GatewayError> {
    Ok(Json(state.system.get_monitor()))
}

/// GET `/api/v1/system/logs`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn get_system_logs(
    State(state): State<GatewayState>,
    Query(query): Query<SystemLogsQueryRequest>,
) -> Result<Json<SystemLogsProjection>, GatewayError> {
    Ok(Json(state.system.get_logs(&query)?))
}

/// GET `/api/v1/system/storage`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn get_storage(
    State(state): State<GatewayState>,
) -> Result<Json<StorageProjection>, GatewayError> {
    Ok(Json(state.system.get_storage()))
}

/// POST `/api/v1/system/storage/snapshots`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
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
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn restore_snapshot(
    State(state): State<GatewayState>,
    Json(request): Json<RestoreSnapshotRequest>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let outcome = state.system.restore_snapshot(&request.snapshot_id)?;
    Ok(Json(serde_json::json!({ "outcome": outcome })))
}

/// GET `/api/v1/system/network`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn get_network(
    State(state): State<GatewayState>,
) -> Result<Json<NetworkProjection>, GatewayError> {
    Ok(Json(state.system.get_network()))
}

/// POST `/api/v1/system/network/connect`
///
/// # Errors
///
/// Refuses when the request holds no authenticated seat.
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
///
/// # Errors
///
/// Refuses when the request holds no authenticated seat, and reports unavailable when the owner cannot be read.
pub async fn get_packages(
    State(state): State<GatewayState>,
) -> Result<Json<PackagesProjection>, GatewayError> {
    Ok(Json(state.system.get_packages()))
}

/// POST `/api/v1/system/packages/action`
///
/// Installing or upgrading software is an authorized action like any other, and this route does the
/// one part only the gateway can do: establish who is asking. Whether the verb exists, what it
/// costs, whether a person must confirm it and whether a permit follows all belong to `Action1`
/// (ADR-0048). Nothing here installs anything.
///
/// # Errors
///
/// Refuses when the request holds no authenticated seat, and reports unavailable when the owner cannot be read.
pub async fn execute_package_action(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<PackageActionRequest>,
) -> Result<Json<cybou_web_contracts::ActionRecordProjection>, GatewayError> {
    let seat = state
        .authenticated_principal(&headers)
        .ok_or(GatewayError::Refused)?;
    let verb = SystemHub::verb_for_package(request.action)?;

    state
        .presence
        .request_action(verb, &SystemHub::package_target(&request.name), &seat)
        .await
        .map(Json)
        .ok_or(GatewayError::Unavailable)
}

/// GET `/api/v1/system/updates`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn get_system_updates(
    State(state): State<GatewayState>,
) -> Result<Json<SystemUpdatesProjection>, GatewayError> {
    Ok(Json(state.system.get_system_updates()))
}

/// POST `/api/v1/system/updates/apply`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn apply_system_updates(
    State(state): State<GatewayState>,
    Json(request): Json<ApplyUpdatesRequest>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let outcome = state.system.apply_system_updates(request.package_names)?;
    Ok(Json(serde_json::json!({ "outcome": outcome })))
}

/// GET `/api/v1/system/users`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn get_users_settings(
    State(state): State<GatewayState>,
) -> Result<Json<UsersSettingsProjection>, GatewayError> {
    Ok(Json(state.system.get_users_settings()))
}

/// POST `/api/v1/system/users`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
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
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
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
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn delete_ssh_key(
    State(state): State<GatewayState>,
    Json(request): Json<DeleteSshKeyRequest>,
) -> Result<Json<serde_json::Value>, GatewayError> {
    let outcome = state.system.delete_ssh_key(&request.key_id)?;
    Ok(Json(serde_json::json!({ "outcome": outcome })))
}

/// GET `/api/v1/system/security`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn get_security_settings(
    State(state): State<GatewayState>,
) -> Result<Json<SecuritySettingsProjection>, GatewayError> {
    Ok(Json(state.system.get_security_settings()))
}

/// POST `/api/v1/system/security/policy`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn update_security_policy(
    State(state): State<GatewayState>,
    Json(request): Json<UpdateSecurityPolicyRequest>,
) -> Result<Json<SecurityPolicyRecord>, GatewayError> {
    let record = state.system.update_security_policy(request)?;
    Ok(Json(record))
}

/// GET `/api/v1/system/backup`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn get_backup_settings(
    State(state): State<GatewayState>,
) -> Result<Json<BackupSettingsProjection>, GatewayError> {
    Ok(Json(state.system.get_backup_settings()))
}

/// POST `/api/v1/system/backup/trigger`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
pub async fn trigger_backup(
    State(state): State<GatewayState>,
    Json(request): Json<TriggerBackupRequest>,
) -> Result<Json<BackupArchiveRecord>, GatewayError> {
    let record = state.system.trigger_backup(request.name)?;
    Ok(Json(record))
}

/// POST `/api/v1/system/backup/restore`
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
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
///
/// # Errors
///
/// Returns the refusal the owner gave, unchanged.
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
    #[expect(
        clippy::too_many_lines,
        reason = "one surface per block; the point of the test is that every one of them is here"
    )]
    fn system_hub_manages_services_and_processes() {
        let hub = SystemHub::new();
        let svcs = hub.list_services();
        assert!(!svcs.services.is_empty());

        // Four of the six name an operation with an adapter behind it. The table and the adapter
        // are checked where they live; what this holds is that the panel's buttons reach the right
        // verb — a Stop that asked for `service.restart` would look, to the person who pressed it,
        // exactly like it worked.
        for (action, verb) in [
            (ServiceAction::Restart, "service.restart"),
            (ServiceAction::Start, "service.start"),
            (ServiceAction::Stop, "service.stop"),
            (ServiceAction::Reload, "service.reload"),
        ] {
            assert_eq!(
                SystemHub::verb_for(action).expect("this build can express it"),
                verb
            );
        }
        // All six now. Enable and disable were refused here by name until the risk of an act that
        // takes effect at the next boot had been decided rather than assumed.
        assert_eq!(
            SystemHub::verb_for(ServiceAction::Enable).expect("a verb"),
            "service.enable"
        );
        assert_eq!(
            SystemHub::verb_for(ServiceAction::Disable).expect("a verb"),
            "service.disable"
        );
        assert_eq!(
            SystemHub::service_target("nginx.service"),
            "systemd:nginx.service"
        );

        let procs = hub.list_processes();
        assert_eq!(procs.schema_version, cybou_web_contracts::WEB_SCHEMA_V1);

        // Four signals, four verbs. One verb with the signal as an argument would have forced the
        // operation table to price a request the process can catch and a kill it cannot as the
        // same act.
        assert_eq!(
            SystemHub::verb_for_signal(ProcessSignal::Terminate),
            "process.terminate"
        );
        assert_eq!(
            SystemHub::verb_for_signal(ProcessSignal::Kill),
            "process.kill"
        );
        assert_eq!(SystemHub::process_target(1000, 4321), "process:1000:4321");
        // A seat this gateway cannot resolve to a uid signals nothing. `local-desktop` is a real
        // seat for reading and has no Linux account behind it to own a process.
        assert!(SystemHub::signalling_seat_owns("local-desktop", 1024).is_err());

        let mon = hub.get_monitor();
        assert_eq!(mon.schema_version, cybou_web_contracts::WEB_SCHEMA_V1);

        let storage = hub.get_storage();
        assert_eq!(storage.schema_version, cybou_web_contracts::WEB_SCHEMA_V1);
        // Established only where a filesystem read produced a capacity; a host that answered
        // nothing must not report a pool of zero bytes as a fact about itself.
        if storage.state == cybou_web_contracts::SystemSurfaceState::Unknown {
            assert_eq!(storage.total_space_bytes, 0);
        } else {
            assert!(storage.total_space_bytes > 0);
        }
        // Subvolume listing needs a privileged btrfs query this gateway does not make.
        assert!(storage.subvolumes.is_empty());
        let snap = hub.create_snapshot("@home", "test-backup", true);
        assert!(snap.is_err());

        // Both surfaces now have real readers, so what they report depends on the host the tests
        // run on. What must hold everywhere is that a surface only claims to be established when a
        // reader established it, and that reading changes nothing about what may be mutated.
        let network = hub.get_network();
        assert_eq!(network.schema_version, cybou_web_contracts::WEB_SCHEMA_V1);
        if network.state == cybou_web_contracts::SystemSurfaceState::Unknown {
            assert!(network.connections.is_empty());
        }
        let conn_res = hub.connect_network("conn-wg0", true);
        assert!(conn_res.is_err());

        let pkgs = hub.get_packages();
        assert_eq!(pkgs.schema_version, cybou_web_contracts::WEB_SCHEMA_V1);
        // Nothing consulted the repositories, so the upgradable count is never established.
        assert_eq!(pkgs.upgradable_count, None);
        if pkgs.state == cybou_web_contracts::SystemSurfaceState::Unknown {
            assert!(pkgs.packages.is_empty());
        } else {
            assert_eq!(pkgs.installed_count, pkgs.packages.len());
        }
        // Installing and upgrading are typed operations now; removing and reinstalling are not,
        // and are refused by name rather than proposed and refused three layers down.
        assert_eq!(
            SystemHub::verb_for_package(PackageActionKind::Install).expect("a verb"),
            "package.install"
        );
        assert_eq!(
            SystemHub::verb_for_package(PackageActionKind::Upgrade).expect("a verb"),
            "package.upgrade"
        );
        assert!(SystemHub::verb_for_package(PackageActionKind::Remove).is_err());
        assert!(SystemHub::verb_for_package(PackageActionKind::Reinstall).is_err());
        assert_eq!(SystemHub::package_target("ripgrep"), "apt:ripgrep");

        let updates = hub.get_system_updates();
        assert_eq!(updates.schema_version, cybou_web_contracts::WEB_SCHEMA_V1);
        assert_eq!(
            updates.state,
            cybou_web_contracts::SystemSurfaceState::Unknown
        );
        let update_res = hub.apply_system_updates(None);
        assert!(update_res.is_err());

        let users = hub.get_users_settings();
        assert_eq!(users.schema_version, cybou_web_contracts::WEB_SCHEMA_V1);
        if users.state == cybou_web_contracts::SystemSurfaceState::Unknown {
            assert!(users.users.is_empty());
        } else {
            // Every account is a person's, and no account's lock state is claimed from an
            // unprivileged read.
            assert!(users.users.iter().all(|account| account.uid >= 1000));
            assert!(
                users
                    .users
                    .iter()
                    .all(|account| account.is_locked.is_none())
            );
        }
        // Authorized keys belong to the account that holds them; this gateway reads none.
        assert!(users.ssh_keys.is_empty());
        let new_user = hub.create_user("bob", "Bob Smith", false);
        assert!(new_user.is_err());

        let sec = hub.get_security_settings();
        assert_eq!(sec.state, cybou_web_contracts::SystemSurfaceState::Unknown);
        assert!(sec.policy.is_none());
        let update_sec = hub.update_security_policy(UpdateSecurityPolicyRequest {
            landlock_enabled: true,
            bubblewrap_enabled: true,
            apparmor_enforcing: true,
            seccomp_strict: true,
            egress_firewall_strict: true,
        });
        assert!(update_sec.is_err());

        let backup = hub.get_backup_settings();
        assert_eq!(
            backup.state,
            cybou_web_contracts::SystemSurfaceState::NotConfigured
        );
        assert!(backup.repository.is_none());
        assert!(backup.schedule.is_none());
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
