// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Grounded provider and Action1 boundary for system services, processes, telemetry, logs, storage, network, packages, users, security, and backups.

use cybou_protocol::system::{
    BackupArchiveRecord, BackupScheduleRecord, NetworkConnectionRecord, PackageRecord,
    ProcessSignal, SecurityAuditEntry, SecurityPolicyRecord, ServiceAction, SnapshotRecord,
    SshKeyRecord, SystemUpdatesSummary, UserAccountRecord,
};
use cybou_web_contracts::{
    BackupSettingsProjection, NetworkProjection, PackagesProjection, ProcessesListProjection,
    SecuritySettingsProjection, ServicesListProjection, StorageProjection, SystemLogsProjection,
    SystemLogsQueryRequest, SystemMonitorProjection, SystemSurfaceState, SystemUpdatesProjection,
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
    security_audit: RwLock<Vec<SecurityAuditEntry>>,
    backup_archives: RwLock<Vec<BackupArchiveRecord>>,
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
            security_audit: RwLock::new(Vec::new()),
            backup_archives: RwLock::new(Vec::new()),
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
    ///
    /// # Errors
    ///
    /// Returns the refusal the owner gave, unchanged.
    pub fn verb_for(action: ServiceAction) -> Result<&'static str, GatewayError> {
        match action {
            ServiceAction::Restart => Ok("service.restart"),
            ServiceAction::Start => Ok("service.start"),
            ServiceAction::Stop => Ok("service.stop"),
            ServiceAction::Reload => Ok("service.reload"),
            // These two were refused here by name, with the reason that they change what happens at
            // the next boot rather than what is happening now and so wanted their own decision
            // about risk. That decision is made: both are Medium, because the delay between the
            // act and its effect is the risk, and neither relieves any finding.
            ServiceAction::Enable => Ok("service.enable"),
            ServiceAction::Disable => Ok("service.disable"),
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

    /// The operation table's name for one of the four signals a person may send.
    ///
    /// Four verbs rather than one with an argument, because they are four different acts with four
    /// different risks: `SIGTERM` lets a process save what it was holding and `SIGKILL` does not,
    /// and a table that could not tell them apart would have to price both as the worse one.
    #[must_use]
    pub const fn verb_for_signal(signal: ProcessSignal) -> &'static str {
        match signal {
            ProcessSignal::Terminate => "process.terminate",
            ProcessSignal::Kill => "process.kill",
            ProcessSignal::Pause => "process.pause",
            ProcessSignal::Resume => "process.resume",
        }
    }

    /// How a process reaches the executor: the uid that owns it and the pid, both as numbers.
    ///
    /// The uid is carried so that the executor can disagree. It reads `/proc` again at the moment
    /// it acts, and a pid the kernel has recycled in the meantime no longer matches.
    #[must_use]
    pub fn process_target(owner_uid: u32, pid: u32) -> String {
        format!("process:{owner_uid}:{pid}")
    }

    /// The uid this seat runs as, and the uid that owns this process, if both can be established.
    ///
    /// Refuses rather than guesses. A signal aimed at a process whose owner cannot be read is a
    /// signal aimed at something nobody can name.
    ///
    /// # Errors
    ///
    /// Returns the refusal the owner gave, unchanged.
    pub fn signalling_seat_owns(seat: &str, pid: u32) -> Result<u32, GatewayError> {
        let account = seat
            .strip_prefix("linux-account:")
            .ok_or(GatewayError::Refused)?;
        let seat_uid = system_reader::uid_for_user(account).ok_or(GatewayError::Refused)?;
        let owner = system_reader::owner_of_process(pid).ok_or(GatewayError::Refused)?;
        // The one rule the gateway is in a position to enforce, because it is the only party that
        // knows who is asking. A person may end their own processes; ending somebody else's is a
        // different act and does not have a door here.
        if owner != seat_uid {
            return Err(GatewayError::Refused);
        }
        Ok(owner)
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
            // The capacity figures come from a real filesystem read. Subvolumes need a btrfs query
            // this gateway cannot make unprivileged, so that list stays empty on every host.
            state: if total_space_bytes == 0 {
                SystemSurfaceState::Unknown
            } else {
                SystemSurfaceState::Known
            },
            subvolumes: Vec::new(),
            snapshots,
            total_space_bytes,
            free_space_bytes,
        }
    }

    /// Create a storage snapshot.
    ///
    /// # Errors
    ///
    /// Returns the refusal the owner gave, unchanged.
    pub fn create_snapshot(
        &self,
        _subvolume: &str,
        _name: &str,
        _readonly: bool,
    ) -> Result<SnapshotRecord, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Restore a storage snapshot.
    ///
    /// # Errors
    ///
    /// Returns the refusal the owner gave, unchanged.
    pub fn restore_snapshot(&self, _snapshot_id: &str) -> Result<String, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Get network status.
    #[must_use]
    pub fn get_network(&self) -> NetworkProjection {
        // Operator-declared connections, plus whatever the kernel itself says exists. When the
        // kernel cannot be read the surface stays unknown rather than reporting a host with no
        // network at all.
        let declared = self
            .connections
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let Some(observed) = system_reader::read_real_network() else {
            return NetworkProjection {
                schema_version: WEB_SCHEMA_V1,
                state: SystemSurfaceState::Unknown,
                connections: declared,
            };
        };
        let mut connections = observed;
        for connection in declared {
            if !connections.iter().any(|value| value.id == connection.id) {
                connections.push(connection);
            }
        }
        NetworkProjection {
            schema_version: WEB_SCHEMA_V1,
            state: SystemSurfaceState::Known,
            connections,
        }
    }

    /// Connect to a network.
    ///
    /// # Errors
    ///
    /// Returns the refusal the owner gave, unchanged.
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
        let Some(packages) = system_reader::read_real_packages() else {
            let packages = self
                .packages
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            let installed_count = packages.len();
            return PackagesProjection {
                schema_version: WEB_SCHEMA_V1,
                state: SystemSurfaceState::Unknown,
                installed_count,
                upgradable_count: None,
                packages,
            };
        };
        PackagesProjection {
            schema_version: WEB_SCHEMA_V1,
            state: SystemSurfaceState::Known,
            installed_count: packages.len(),
            // dpkg says what is installed. Nothing here consults the repositories, so how many
            // packages could be upgraded is not established, and is not reported as none.
            upgradable_count: None,
            packages,
        }
    }

    /// Which operation, if any, this build can carry out for a package action.
    ///
    /// A pure mapping, like the service one: the hub decides nothing about whether it may happen.
    /// Installing and upgrading are typed operations with an executor adapter behind them, both
    /// High risk and neither undoable, so both reach a person for confirmation unless an operator
    /// pre-authorized them.
    ///
    /// Removing and reinstalling are refused here by name. Neither is in the closed operation table
    /// and neither has an adapter, and a refusal that says what is missing is worth more than one
    /// that arrives three layers down. Removal in particular is a different act: it takes software
    /// away from a host that may be depending on it, and it wants its own decision about risk
    /// before it gets a verb.
    ///
    /// # Errors
    ///
    /// Refuses an action this build cannot express as a typed operation.
    pub fn verb_for_package(
        action: cybou_protocol::system::PackageActionKind,
    ) -> Result<&'static str, GatewayError> {
        match action {
            cybou_protocol::system::PackageActionKind::Install => Ok("package.install"),
            cybou_protocol::system::PackageActionKind::Upgrade => Ok("package.upgrade"),
            cybou_protocol::system::PackageActionKind::Remove
            | cybou_protocol::system::PackageActionKind::Reinstall => Err(GatewayError::Refused),
        }
    }

    /// How a package name reaches the executor.
    ///
    /// The prefix is what tells Action1 this names an archive package rather than something else
    /// spelled the same way, and both Action1 and the executor refuse a name that is not concrete.
    #[must_use]
    pub fn package_target(name: &str) -> String {
        format!("apt:{name}")
    }

    /// Get pending system updates.
    #[must_use]
    pub fn get_system_updates(&self) -> SystemUpdatesProjection {
        SystemUpdatesProjection {
            schema_version: WEB_SCHEMA_V1,
            state: SystemSurfaceState::Unknown,
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
    ///
    /// # Errors
    ///
    /// Returns the refusal the owner gave, unchanged.
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
        // Authorized keys live in each person's own home and are readable by that person. This
        // gateway does not hold them, so the list stays empty rather than being filled from
        // somewhere it has no business reading.
        let Some(users) = system_reader::read_real_users() else {
            return UsersSettingsProjection {
                schema_version: WEB_SCHEMA_V1,
                state: SystemSurfaceState::Unknown,
                users,
                ssh_keys,
            };
        };
        UsersSettingsProjection {
            schema_version: WEB_SCHEMA_V1,
            state: SystemSurfaceState::Known,
            users,
            ssh_keys,
        }
    }

    /// Create a new user account.
    ///
    /// # Errors
    ///
    /// Returns the refusal the owner gave, unchanged.
    pub fn create_user(
        &self,
        _username: &str,
        _full_name: &str,
        _is_admin: bool,
    ) -> Result<UserAccountRecord, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Add an authorized SSH key.
    ///
    /// # Errors
    ///
    /// Returns the refusal the owner gave, unchanged.
    pub fn add_ssh_key(
        &self,
        _name: &str,
        _public_key: &str,
    ) -> Result<SshKeyRecord, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Delete an authorized SSH key.
    ///
    /// # Errors
    ///
    /// Returns the refusal the owner gave, unchanged.
    pub fn delete_ssh_key(&self, _key_id: &str) -> Result<String, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Get security settings and policy.
    #[must_use]
    pub fn get_security_settings(&self) -> SecuritySettingsProjection {
        let audit_log = self
            .security_audit
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        SecuritySettingsProjection {
            schema_version: WEB_SCHEMA_V1,
            state: SystemSurfaceState::Unknown,
            policy: None,
            audit_log,
        }
    }

    /// Update security policy.
    ///
    /// # Errors
    ///
    /// Returns the refusal the owner gave, unchanged.
    pub fn update_security_policy(
        &self,
        _req: UpdateSecurityPolicyRequest,
    ) -> Result<SecurityPolicyRecord, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Get backup repository settings.
    #[must_use]
    pub fn get_backup_settings(&self) -> BackupSettingsProjection {
        let archives = self
            .backup_archives
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        BackupSettingsProjection {
            schema_version: WEB_SCHEMA_V1,
            state: SystemSurfaceState::NotConfigured,
            repository: None,
            archives,
            schedule: None,
        }
    }

    /// Trigger a backup job.
    ///
    /// # Errors
    ///
    /// Returns the refusal the owner gave, unchanged.
    pub fn trigger_backup(
        &self,
        _comment: Option<String>,
    ) -> Result<BackupArchiveRecord, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Restore a backup archive.
    ///
    /// # Errors
    ///
    /// Returns the refusal the owner gave, unchanged.
    pub fn restore_archive(
        &self,
        _archive_id: &str,
        _target_path: Option<String>,
    ) -> Result<String, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Update backup schedule.
    ///
    /// # Errors
    ///
    /// Returns the refusal the owner gave, unchanged.
    pub fn update_backup_schedule(
        &self,
        _req: UpdateBackupScheduleRequest,
    ) -> Result<BackupScheduleRecord, GatewayError> {
        Err(GatewayError::Refused)
    }
}
