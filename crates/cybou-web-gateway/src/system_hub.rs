// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Grounded provider and Action1 boundary for system services, processes, telemetry, logs, storage, network, packages, users, security, and backups.

use cybou_protocol::system::{
    BackupArchiveRecord, BackupRepositoryRecord, BackupScheduleRecord, NetworkConnectionRecord,
    PackageRecord, ProcessSignal, SecurityAuditEntry, SecurityPolicyRecord, ServiceAction,
    SnapshotRecord, SshKeyRecord, SystemUpdatesSummary, UserAccountRecord,
};
use cybou_web_contracts::{
    BackupSettingsProjection, NetworkProjection, PackagesProjection, ProcessesListProjection,
    SecuritySettingsProjection, ServicesListProjection, StorageProjection, SystemLogsProjection,
    SystemLogsQueryRequest, SystemMonitorProjection, SystemUpdatesProjection,
    UpdateBackupScheduleRequest, UpdateSecurityPolicyRequest, UsersSettingsProjection,
    WEB_SCHEMA_V1,
};
use std::sync::RwLock;

use crate::state::GatewayError;
use crate::system_reader;

/// Provider and governor for host observation and governed actions.
pub struct SystemHub {
    snapshots: RwLock<Vec<SnapshotRecord>>,
    connections: RwLock<Vec<NetworkConnectionRecord>>,
    packages: RwLock<Vec<PackageRecord>>,
    users: RwLock<Vec<UserAccountRecord>>,
    ssh_keys: RwLock<Vec<SshKeyRecord>>,
    security_policy: RwLock<SecurityPolicyRecord>,
    security_audit: RwLock<Vec<SecurityAuditEntry>>,
    backup_repo: RwLock<BackupRepositoryRecord>,
    backup_archives: RwLock<Vec<BackupArchiveRecord>>,
    backup_schedule: RwLock<BackupScheduleRecord>,
}

impl Default for SystemHub {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemHub {
    /// Create a new `SystemHub` with grounded initial state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            snapshots: RwLock::new(Vec::new()),
            connections: RwLock::new(Vec::new()),
            packages: RwLock::new(Vec::new()),
            users: RwLock::new(Vec::new()),
            ssh_keys: RwLock::new(Vec::new()),
            security_policy: RwLock::new(SecurityPolicyRecord {
                landlock_enabled: true,
                bubblewrap_enabled: true,
                apparmor_enforcing: true,
                seccomp_strict: true,
                egress_firewall_strict: true,
            }),
            security_audit: RwLock::new(Vec::new()),
            backup_repo: RwLock::new(BackupRepositoryRecord {
                id: "repo-primary".to_owned(),
                name: "Primary Backup Vault".to_owned(),
                destination: "/var/lib/cybou/backup-vault".to_owned(),
                encryption: "repokey-blake2".to_owned(),
                last_backup_time: None,
                total_archives: 0,
                total_size_bytes: 0,
            }),
            backup_archives: RwLock::new(Vec::new()),
            backup_schedule: RwLock::new(BackupScheduleRecord {
                enabled: false,
                frequency: "daily".to_owned(),
                retention_daily: 7,
                retention_weekly: 4,
                retention_monthly: 6,
            }),
        }
    }

    /// List real running systemd services observed on host.
    #[must_use]
    pub fn list_services(&self) -> ServicesListProjection {
        system_reader::read_real_services()
    }

    /// Which operation, if any, this build can actually carry out for a service action.
    ///
    /// A pure mapping and nothing more: the hub decides nothing about whether it may happen, which
    /// is `Action1`'s to answer. What this settles is narrower and is the whole of what the panel's
    /// six buttons come down to — only one of them names an operation with an adapter behind it.
    ///
    /// Four of the six are in the closed operation table with an executor adapter behind each.
    /// Enable and disable are not, and the reason is not that they were forgotten: they change what
    /// happens at the next boot rather than what is happening now, so nobody is present when they
    /// take effect. That is a different act and wants its own decision about risk before it gets a
    /// verb. They are refused here by name rather than proposed and refused three layers down,
    /// because a refusal that says what is missing is worth more than one that arrives from
    /// somewhere else.
    pub fn verb_for(action: ServiceAction) -> Result<&'static str, GatewayError> {
        match action {
            ServiceAction::Restart => Ok("service.restart"),
            ServiceAction::Start => Ok("service.start"),
            ServiceAction::Stop => Ok("service.stop"),
            ServiceAction::Reload => Ok("service.reload"),
            // Enable and disable change what happens at the next boot rather than what is happening
            // now, which is a different kind of act: nobody is present when it takes effect. They
            // are not in the operation table and are refused here by name.
            ServiceAction::Enable | ServiceAction::Disable => Err(GatewayError::Refused),
        }
    }

    /// How a service name reaches the executor.
    ///
    /// The prefix is what tells `Action1` this names a systemd unit rather than something else with
    /// the same spelling, and the executor refuses a target that is not concrete.
    #[must_use]
    pub fn service_target(name: &str) -> String {
        format!("systemd:{name}")
    }

    /// List real running operating system processes from /proc.
    #[must_use]
    pub fn list_processes(&self) -> ProcessesListProjection {
        system_reader::read_real_processes()
    }

    /// Send a signal to an operating system process.
    ///
    /// Direct in-memory simulation is prohibited: privileged process termination requires Action1 or unprivileged self ownership.
    pub fn send_process_signal(
        &self,
        _pid: u32,
        _signal: ProcessSignal,
    ) -> Result<String, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Read real system hardware and telemetry metrics from /proc and /sys.
    #[must_use]
    pub fn get_monitor(&self) -> SystemMonitorProjection {
        system_reader::read_real_monitor()
    }

    /// Read the systemd journal.
    ///
    /// The severity filter is a closed set of syslog names. A name outside it is refused rather
    /// than dropped: a filter that silently stopped filtering would answer "show me the errors"
    /// with everything, and look like a quiet host.
    ///
    /// # Errors
    ///
    /// Refuses a severity that is not one of the eight syslog names. A journal that could not be
    /// read is not an error here: it is a projection that says why it is empty.
    pub fn get_logs(
        &self,
        query: &SystemLogsQueryRequest,
    ) -> Result<SystemLogsProjection, GatewayError> {
        let priority = match query
            .severity
            .as_deref()
            .map(str::trim)
            .filter(|severity| !severity.is_empty())
        {
            Some(severity) => {
                Some(system_reader::severity_priority(severity).ok_or(GatewayError::Refused)?)
            }
            None => None,
        };

        Ok(system_reader::read_journal(query, priority))
    }

    /// Get current storage and snapshot state.
    #[must_use]
    pub fn get_storage(&self) -> StorageProjection {
        let monitor = system_reader::read_real_monitor();
        let root_disk = monitor
            .disk_partitions
            .into_iter()
            .find(|d| d.mount_point == "/");
        let (total_space_bytes, free_space_bytes) =
            root_disk.map_or((0, 0), |d| (d.total_bytes, d.available_bytes));

        let snapshots = self
            .snapshots
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();

        StorageProjection {
            schema_version: WEB_SCHEMA_V1,
            subvolumes: Vec::new(),
            snapshots,
            total_space_bytes,
            free_space_bytes,
        }
    }

    /// Create a storage snapshot.
    pub fn create_snapshot(
        &self,
        _subvolume: &str,
        _name: &str,
        _readonly: bool,
    ) -> Result<SnapshotRecord, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Restore a storage snapshot.
    pub fn restore_snapshot(&self, _snapshot_id: &str) -> Result<String, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Get network status.
    #[must_use]
    pub fn get_network(&self) -> NetworkProjection {
        let connections = self
            .connections
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();

        NetworkProjection {
            schema_version: WEB_SCHEMA_V1,
            connections,
        }
    }

    /// Connect to a network.
    pub fn connect_network(
        &self,
        _connection_id: &str,
        _activate: bool,
    ) -> Result<String, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Get package repository and installation status.
    #[must_use]
    pub fn get_packages(&self) -> PackagesProjection {
        let packages = self
            .packages
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let installed_count = packages.len();
        PackagesProjection {
            schema_version: WEB_SCHEMA_V1,
            installed_count,
            upgradable_count: 0,
            packages,
        }
    }

    /// Execute a package operation.
    pub fn execute_package_action(
        &self,
        _package: &str,
        _action: cybou_protocol::system::PackageActionKind,
    ) -> Result<String, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Get pending system updates.
    #[must_use]
    pub fn get_system_updates(&self) -> SystemUpdatesProjection {
        SystemUpdatesProjection {
            schema_version: WEB_SCHEMA_V1,
            summary: SystemUpdatesSummary {
                pending_count: 0,
                security_updates_count: 0,
                kernel_update: false,
                reboot_required: false,
                total_download_bytes: 0,
                packages: Vec::new(),
            },
        }
    }

    /// Apply system updates.
    pub fn apply_system_updates(
        &self,
        _package_names: Option<Vec<String>>,
    ) -> Result<String, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Get users and SSH keys.
    #[must_use]
    pub fn get_users_settings(&self) -> UsersSettingsProjection {
        let users = self
            .users
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let ssh_keys = self
            .ssh_keys
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        UsersSettingsProjection {
            schema_version: WEB_SCHEMA_V1,
            users,
            ssh_keys,
        }
    }

    /// Create a new user account.
    pub fn create_user(
        &self,
        _username: &str,
        _full_name: &str,
        _is_admin: bool,
    ) -> Result<UserAccountRecord, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Add an authorized SSH key.
    pub fn add_ssh_key(
        &self,
        _name: &str,
        _public_key: &str,
    ) -> Result<SshKeyRecord, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Delete an authorized SSH key.
    pub fn delete_ssh_key(&self, _key_id: &str) -> Result<String, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Get security settings and policy.
    #[must_use]
    pub fn get_security_settings(&self) -> SecuritySettingsProjection {
        let policy = self
            .security_policy
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let audit_log = self
            .security_audit
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        SecuritySettingsProjection {
            schema_version: WEB_SCHEMA_V1,
            policy,
            audit_log,
        }
    }

    /// Update security policy.
    pub fn update_security_policy(
        &self,
        _req: UpdateSecurityPolicyRequest,
    ) -> Result<SecurityPolicyRecord, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Get backup repository settings.
    #[must_use]
    pub fn get_backup_settings(&self) -> BackupSettingsProjection {
        let repository = self
            .backup_repo
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let archives = self
            .backup_archives
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let schedule = self
            .backup_schedule
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        BackupSettingsProjection {
            schema_version: WEB_SCHEMA_V1,
            repository,
            archives,
            schedule,
        }
    }

    /// Trigger a backup job.
    pub fn trigger_backup(
        &self,
        _comment: Option<String>,
    ) -> Result<BackupArchiveRecord, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Restore a backup archive.
    pub fn restore_archive(
        &self,
        _archive_id: &str,
        _target_path: Option<String>,
    ) -> Result<String, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Update backup schedule.
    pub fn update_backup_schedule(
        &self,
        _req: UpdateBackupScheduleRequest,
    ) -> Result<BackupScheduleRecord, GatewayError> {
        Err(GatewayError::Refused)
    }
}
