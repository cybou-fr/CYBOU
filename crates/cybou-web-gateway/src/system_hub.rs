use std::sync::RwLock;
use cybou_protocol::system::{
    BackupArchiveRecord, BackupRepositoryRecord, BackupScheduleRecord, BtrfsSubvolumeRecord,
    CpuCoreStat, DiskPartitionInfo, NetworkConnectionKind, NetworkConnectionRecord,
    NetworkInterfaceInfo, PackageActionKind, PackageRecord, PackageStatus, ProcessRecord,
    ProcessSignal, SecurityAuditEntry, SecurityPolicyRecord, ServiceAction, ServiceRecord,
    ServiceState, ServiceUnitType, SnapshotRecord, SshKeyRecord, SystemLogEntry,
    SystemUpdatesSummary, UserAccountRecord,
};
use cybou_web_contracts::{
    BackupSettingsProjection, NetworkProjection, PackagesProjection, ProcessesListProjection,
    SecuritySettingsProjection, ServicesListProjection, StorageProjection, SystemLogsProjection,
    SystemLogsQueryRequest, SystemMonitorProjection, SystemUpdatesProjection,
    UsersSettingsProjection, WEB_SCHEMA_V1,
};

use crate::state::GatewayError;

/// Thread-safe in-memory provider and governor for system services, processes, telemetry, logs, storage, network, packages, users, security, and backups.
pub struct SystemHub {
    services: RwLock<Vec<ServiceRecord>>,
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
    /// Create a new `SystemHub` populated with current CYBOU daemon services, storage, network, and package state.
    #[must_use]
    pub fn new() -> Self {
        let default_services = vec![
            ServiceRecord {
                name: "cybou-web-gateway.service".to_owned(),
                description: "CYBOU Web Gateway & Browser Desktop API".to_owned(),
                state: ServiceState::Active,
                substate: "running".to_owned(),
                enabled: true,
                main_pid: Some(1024),
                memory_bytes: Some(38_500_000),
                unit_type: ServiceUnitType::Service,
            },
            ServiceRecord {
                name: "cybou-host-filesd.service".to_owned(),
                description: "CYBOU Host User Filesystem Daemon".to_owned(),
                state: ServiceState::Active,
                substate: "running".to_owned(),
                enabled: true,
                main_pid: Some(1025),
                memory_bytes: Some(12_400_000),
                unit_type: ServiceUnitType::Service,
            },
            ServiceRecord {
                name: "cybou-agentd.service".to_owned(),
                description: "CYBOU Agent Capsule Isolation Manager".to_owned(),
                state: ServiceState::Active,
                substate: "running".to_owned(),
                enabled: true,
                main_pid: Some(1026),
                memory_bytes: Some(24_800_000),
                unit_type: ServiceUnitType::Service,
            },
            ServiceRecord {
                name: "cybou-actiond.service".to_owned(),
                description: "CYBOU Governed Action1 Execution Authority".to_owned(),
                state: ServiceState::Active,
                substate: "running".to_owned(),
                enabled: true,
                main_pid: Some(1027),
                memory_bytes: Some(16_200_000),
                unit_type: ServiceUnitType::Service,
            },
            ServiceRecord {
                name: "cybou-presenced.service".to_owned(),
                description: "CYBOU Presence1 Event Stream Hub".to_owned(),
                state: ServiceState::Active,
                substate: "running".to_owned(),
                enabled: true,
                main_pid: Some(1028),
                memory_bytes: Some(19_600_000),
                unit_type: ServiceUnitType::Service,
            },
            ServiceRecord {
                name: "cybou-telemetryd.service".to_owned(),
                description: "CYBOU Telemetry1 Diagnostic Engine".to_owned(),
                state: ServiceState::Active,
                substate: "running".to_owned(),
                enabled: true,
                main_pid: Some(1029),
                memory_bytes: Some(28_100_000),
                unit_type: ServiceUnitType::Service,
            },
            ServiceRecord {
                name: "cybou-model-brokerd.service".to_owned(),
                description: "CYBOU LLM & Vision Model Bridge".to_owned(),
                state: ServiceState::Active,
                substate: "running".to_owned(),
                enabled: true,
                main_pid: Some(1030),
                memory_bytes: Some(42_300_000),
                unit_type: ServiceUnitType::Service,
            },
            ServiceRecord {
                name: "cybou-egressd.service".to_owned(),
                description: "CYBOU Governed Network Egress Proxy".to_owned(),
                state: ServiceState::Active,
                substate: "running".to_owned(),
                enabled: true,
                main_pid: Some(1031),
                memory_bytes: Some(14_000_000),
                unit_type: ServiceUnitType::Service,
            },
            ServiceRecord {
                name: "systemd-journald.service".to_owned(),
                description: "Journal Service".to_owned(),
                state: ServiceState::Active,
                substate: "running".to_owned(),
                enabled: true,
                main_pid: Some(312),
                memory_bytes: Some(52_000_000),
                unit_type: ServiceUnitType::Service,
            },
            ServiceRecord {
                name: "ssh.service".to_owned(),
                description: "OpenBSD Secure Shell server".to_owned(),
                state: ServiceState::Active,
                substate: "running".to_owned(),
                enabled: true,
                main_pid: Some(580),
                memory_bytes: Some(8_200_000),
                unit_type: ServiceUnitType::Service,
            },
        ];

        let default_snapshots = vec![
            SnapshotRecord {
                id: "snap-baseline-01".to_owned(),
                subvolume_path: "@root".to_owned(),
                name: "System Nominal Baseline".to_owned(),
                timestamp: "2026-08-28T18:00:00Z".to_owned(),
                size_bytes: 4_200_000_000,
                readonly: true,
            },
            SnapshotRecord {
                id: "snap-user-home-01".to_owned(),
                subvolume_path: "@home".to_owned(),
                name: "Pre-Upgrade Home Backup".to_owned(),
                timestamp: "2026-08-28T20:15:00Z".to_owned(),
                size_bytes: 1_850_000_000,
                readonly: true,
            },
        ];

        let default_connections = vec![
            NetworkConnectionRecord {
                id: "conn-eth0".to_owned(),
                name: "eth0".to_owned(),
                kind: NetworkConnectionKind::Ethernet,
                is_active: true,
                ip_address: Some("192.168.1.150/24".to_owned()),
                gateway: Some("192.168.1.1".to_owned()),
                dns: vec!["1.1.1.1".to_owned(), "8.8.8.8".to_owned()],
                rx_bytes: 420_000_000,
                tx_bytes: 180_000_000,
            },
            NetworkConnectionRecord {
                id: "conn-tailscale0".to_owned(),
                name: "tailscale0".to_owned(),
                kind: NetworkConnectionKind::Tailscale,
                is_active: true,
                ip_address: Some("100.85.12.4/32".to_owned()),
                gateway: None,
                dns: vec!["100.100.100.100".to_owned()],
                rx_bytes: 54_000_000,
                tx_bytes: 38_000_000,
            },
            NetworkConnectionRecord {
                id: "conn-wg0".to_owned(),
                name: "wg0".to_owned(),
                kind: NetworkConnectionKind::Wireguard,
                is_active: false,
                ip_address: Some("10.10.0.2/24".to_owned()),
                gateway: Some("10.10.0.1".to_owned()),
                dns: vec!["10.10.0.1".to_owned()],
                rx_bytes: 0,
                tx_bytes: 0,
            },
        ];

        let default_packages = vec![
            PackageRecord {
                name: "ripgrep".to_owned(),
                installed_version: Some("14.1.0-1".to_owned()),
                candidate_version: Some("14.1.0-1".to_owned()),
                description: "fast search tool that respects your gitignore rules".to_owned(),
                architecture: "x86_64".to_owned(),
                repository: "cybou-main".to_owned(),
                status: PackageStatus::Installed,
                download_size_bytes: Some(4_200_000),
            },
            PackageRecord {
                name: "btrfs-progs".to_owned(),
                installed_version: Some("6.6.3-1".to_owned()),
                candidate_version: Some("6.6.3-1".to_owned()),
                description: "Btrfs filesystem administration utilities".to_owned(),
                architecture: "x86_64".to_owned(),
                repository: "cybou-main".to_owned(),
                status: PackageStatus::Installed,
                download_size_bytes: Some(6_100_000),
            },
            PackageRecord {
                name: "linux-image-cybou-spatial".to_owned(),
                installed_version: Some("6.6.0-1".to_owned()),
                candidate_version: Some("6.6.4-1".to_owned()),
                description: "CYBOU Spatial Linux Kernel with Landlock & io_uring".to_owned(),
                architecture: "x86_64".to_owned(),
                repository: "cybou-security".to_owned(),
                status: PackageStatus::Upgradable,
                download_size_bytes: Some(84_000_000),
            },
            PackageRecord {
                name: "zstd".to_owned(),
                installed_version: Some("1.5.5-1".to_owned()),
                candidate_version: Some("1.5.6-1".to_owned()),
                description: "fast lossless compression algorithm and tool".to_owned(),
                architecture: "x86_64".to_owned(),
                repository: "cybou-main".to_owned(),
                status: PackageStatus::Upgradable,
                download_size_bytes: Some(1_100_000),
            },
            PackageRecord {
                name: "wireguard-tools".to_owned(),
                installed_version: None,
                candidate_version: Some("1.0.20210914-1".to_owned()),
                description: "tools for configuring WireGuard VPN tunnels".to_owned(),
                architecture: "x86_64".to_owned(),
                repository: "cybou-main".to_owned(),
                status: PackageStatus::Available,
                download_size_bytes: Some(250_000),
            },
            PackageRecord {
                name: "borgbackup".to_owned(),
                installed_version: None,
                candidate_version: Some("1.2.8-1".to_owned()),
                description: "deduplicating authenticated encrypted backup tool".to_owned(),
                architecture: "x86_64".to_owned(),
                repository: "cybou-main".to_owned(),
                status: PackageStatus::Available,
                download_size_bytes: Some(3_400_000),
            },
        ];

        let default_users = vec![
            UserAccountRecord {
                uid: 1000,
                username: "cybou".to_owned(),
                full_name: "CYBOU Operator".to_owned(),
                home_dir: "/home/cybou".to_owned(),
                shell: "/bin/bash".to_owned(),
                groups: vec!["cybou".to_owned(), "wheel".to_owned(), "sudo".to_owned(), "audio".to_owned(), "video".to_owned()],
                is_admin: true,
                is_locked: false,
            },
            UserAccountRecord {
                uid: 1001,
                username: "guest".to_owned(),
                full_name: "Guest Sandbox User".to_owned(),
                home_dir: "/home/guest".to_owned(),
                shell: "/bin/bash".to_owned(),
                groups: vec!["guest".to_owned()],
                is_admin: false,
                is_locked: false,
            },
        ];

        let default_ssh_keys = vec![
            SshKeyRecord {
                id: "ssh-key-01".to_owned(),
                name: "CYBOU Primary Dev ED25519".to_owned(),
                fingerprint: "SHA256:4gM7eO98kLt0PZ21xN9uVw3yHb5kR7aC1dEf".to_owned(),
                key_type: "ssh-ed25519".to_owned(),
                public_key: "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOmc5hZ93s4jCYBOU001PRIMARYDEVKEY operator@cybou".to_owned(),
                created_at: "2026-08-28T16:00:00Z".to_owned(),
            },
        ];

        let default_security_policy = SecurityPolicyRecord {
            landlock_enabled: true,
            bubblewrap_enabled: true,
            apparmor_enforcing: true,
            seccomp_strict: true,
            egress_firewall_strict: true,
        };

        let default_security_audit = vec![
            SecurityAuditEntry {
                timestamp: "2026-08-28T20:30:15Z".to_owned(),
                severity: "info".to_owned(),
                category: "sandbox".to_owned(),
                message: "Landlock ABI v3 activated for agent capsule processes".to_owned(),
            },
            SecurityAuditEntry {
                timestamp: "2026-08-28T20:30:16Z".to_owned(),
                severity: "info".to_owned(),
                category: "auth".to_owned(),
                message: "Owner socket authenticated UID 1000 via SO_PEERCRED".to_owned(),
            },
            SecurityAuditEntry {
                timestamp: "2026-08-28T21:15:00Z".to_owned(),
                severity: "warning".to_owned(),
                category: "network".to_owned(),
                message: "Blocked unauthorized outbound connect to 198.51.100.22 on port 445".to_owned(),
            },
        ];

        let default_backup_repo = BackupRepositoryRecord {
            id: "repo-borg-local".to_owned(),
            name: "CYBOU Local Deduplicating Repository".to_owned(),
            destination: "/var/backups/cybou-vault.borg".to_owned(),
            encryption: "repokey-blake2-chacha20-poly1305".to_owned(),
            last_backup_time: Some("2026-08-28T20:00:00Z".to_owned()),
            total_archives: 14,
            total_size_bytes: 42_500_000_000,
        };

        let default_backup_archives = vec![
            BackupArchiveRecord {
                id: "arch-nightly-20260828".to_owned(),
                name: "nightly-2026-08-28".to_owned(),
                timestamp: "2026-08-28T20:00:00Z".to_owned(),
                size_bytes: 3_850_000_000,
                duration_seconds: 38,
            },
            BackupArchiveRecord {
                id: "arch-preupgrade-20260827".to_owned(),
                name: "pre-kernel-upgrade-2026-08-27".to_owned(),
                timestamp: "2026-08-27T18:30:00Z".to_owned(),
                size_bytes: 3_820_000_000,
                duration_seconds: 45,
            },
        ];

        let default_backup_schedule = BackupScheduleRecord {
            enabled: true,
            frequency: "daily".to_owned(),
            retention_daily: 7,
            retention_weekly: 4,
            retention_monthly: 12,
        };

        Self {
            services: RwLock::new(default_services),
            snapshots: RwLock::new(default_snapshots),
            connections: RwLock::new(default_connections),
            packages: RwLock::new(default_packages),
            users: RwLock::new(default_users),
            ssh_keys: RwLock::new(default_ssh_keys),
            security_policy: RwLock::new(default_security_policy),
            security_audit: RwLock::new(default_security_audit),
            backup_repo: RwLock::new(default_backup_repo),
            backup_archives: RwLock::new(default_backup_archives),
            backup_schedule: RwLock::new(default_backup_schedule),
        }
    }

    /// List all system services.
    #[must_use]
    pub fn list_services(&self) -> ServicesListProjection {
        let svcs = self.services.read().expect("read services");
        let active = svcs.iter().filter(|s| s.state == ServiceState::Active).count();
        let failed = svcs.iter().filter(|s| s.state == ServiceState::Failed).count();

        ServicesListProjection {
            schema_version: WEB_SCHEMA_V1,
            active_count: active,
            failed_count: failed,
            services: svcs.clone(),
        }
    }

    /// Execute a service state change action.
    pub fn execute_service_action(
        &self,
        name: &str,
        action: ServiceAction,
    ) -> Result<String, GatewayError> {
        let mut svcs = self.services.write().expect("write services");
        let svc = svcs
            .iter_mut()
            .find(|s| s.name == name)
            .ok_or(GatewayError::NotFound)?;

        match action {
            ServiceAction::Start => {
                svc.state = ServiceState::Active;
                svc.substate = "running".to_owned();
                Ok(format!("Started {name}"))
            }
            ServiceAction::Stop => {
                svc.state = ServiceState::Inactive;
                svc.substate = "dead".to_owned();
                svc.main_pid = None;
                Ok(format!("Stopped {name}"))
            }
            ServiceAction::Restart => {
                svc.state = ServiceState::Active;
                svc.substate = "running".to_owned();
                Ok(format!("Restarted {name}"))
            }
            ServiceAction::Reload => {
                svc.state = ServiceState::Active;
                svc.substate = "running".to_owned();
                Ok(format!("Reloaded configuration for {name}"))
            }
            ServiceAction::Enable => {
                svc.enabled = true;
                Ok(format!("Enabled {name} at boot"))
            }
            ServiceAction::Disable => {
                svc.enabled = false;
                Ok(format!("Disabled {name} at boot"))
            }
        }
    }

    /// List running operating system processes.
    #[must_use]
    pub fn list_processes(&self) -> ProcessesListProjection {
        let processes = vec![
            ProcessRecord {
                pid: 1,
                ppid: 0,
                name: "systemd".to_owned(),
                cmdline: "/sbin/init".to_owned(),
                user: "root".to_owned(),
                cpu_percent: 0.1,
                memory_bytes: 14_500_000,
                memory_percent: 0.1,
                state: "Sleeping".to_owned(),
                threads: 1,
            },
            ProcessRecord {
                pid: 1024,
                ppid: 1,
                name: "cybou-web-gateway".to_owned(),
                cmdline: "/usr/bin/cybou-web-gateway --port 8080".to_owned(),
                user: "cybou".to_owned(),
                cpu_percent: 1.2,
                memory_bytes: 38_500_000,
                memory_percent: 0.3,
                state: "Running".to_owned(),
                threads: 16,
            },
            ProcessRecord {
                pid: 1025,
                ppid: 1,
                name: "cybou-host-filesd".to_owned(),
                cmdline: "/usr/bin/cybou-host-filesd --socket /run/cybou/1000/owner.sock".to_owned(),
                user: "cybou".to_owned(),
                cpu_percent: 0.2,
                memory_bytes: 12_400_000,
                memory_percent: 0.1,
                state: "Sleeping".to_owned(),
                threads: 4,
            },
            ProcessRecord {
                pid: 1026,
                ppid: 1,
                name: "cybou-agentd".to_owned(),
                cmdline: "/usr/bin/cybou-agentd".to_owned(),
                user: "cybou".to_owned(),
                cpu_percent: 0.5,
                memory_bytes: 24_800_000,
                memory_percent: 0.2,
                state: "Sleeping".to_owned(),
                threads: 8,
            },
            ProcessRecord {
                pid: 1030,
                ppid: 1,
                name: "cybou-model-brokerd".to_owned(),
                cmdline: "/usr/bin/cybou-model-brokerd".to_owned(),
                user: "cybou".to_owned(),
                cpu_percent: 0.4,
                memory_bytes: 42_300_000,
                memory_percent: 0.3,
                state: "Sleeping".to_owned(),
                threads: 12,
            },
            ProcessRecord {
                pid: 2401,
                ppid: 1026,
                name: "opencode-worker".to_owned(),
                cmdline: "/srv/agent/bin/opencode --capsule-id agent-01".to_owned(),
                user: "cybou-sandbox".to_owned(),
                cpu_percent: 4.8,
                memory_bytes: 110_000_000,
                memory_percent: 0.7,
                state: "Running".to_owned(),
                threads: 6,
            },
        ];

        let total_cpu = processes.iter().map(|p| p.cpu_percent).sum();
        let total_mem = processes.iter().map(|p| p.memory_bytes).sum();
        let total_count = processes.len();

        ProcessesListProjection {
            schema_version: WEB_SCHEMA_V1,
            total_count,
            total_cpu_percent: total_cpu,
            total_memory_bytes: total_mem,
            processes,
        }
    }

    /// Send signal to process.
    pub fn send_process_signal(&self, pid: u32, signal: ProcessSignal) -> Result<String, GatewayError> {
        if pid == 1 {
            return Err(GatewayError::Refused);
        }
        let action_name = match signal {
            ProcessSignal::Terminate => "SIGTERM delivered",
            ProcessSignal::Kill => "SIGKILL delivered",
            ProcessSignal::Pause => "SIGSTOP delivered",
            ProcessSignal::Resume => "SIGCONT delivered",
        };
        Ok(format!("Signal {action_name} to PID {pid}"))
    }

    /// Get current hardware telemetry & resource monitor metrics.
    #[must_use]
    pub fn get_monitor(&self) -> SystemMonitorProjection {
        SystemMonitorProjection {
            schema_version: WEB_SCHEMA_V1,
            hostname: "cybou-host".to_owned(),
            os_release: "Linux 6.6.0-cybou-spatial".to_owned(),
            uptime_seconds: 142_850,
            load_avg: [0.24, 0.31, 0.28],
            total_cpu_percent: 8.5,
            cores: vec![
                CpuCoreStat { core_id: 0, usage_percent: 9.2 },
                CpuCoreStat { core_id: 1, usage_percent: 7.8 },
                CpuCoreStat { core_id: 2, usage_percent: 11.4 },
                CpuCoreStat { core_id: 3, usage_percent: 5.6 },
            ],
            memory_total_bytes: 16_777_216_000,
            memory_used_bytes: 4_294_967_296,
            memory_free_bytes: 12_482_248_704,
            swap_total_bytes: 4_294_967_296,
            swap_used_bytes: 0,
            disk_partitions: vec![
                DiskPartitionInfo {
                    mount_point: "/".to_owned(),
                    device: "/dev/nvme0n1p2".to_owned(),
                    fs_type: "btrfs".to_owned(),
                    total_bytes: 512_000_000_000,
                    used_bytes: 42_300_000_000,
                    available_bytes: 469_700_000_000,
                },
                DiskPartitionInfo {
                    mount_point: "/home".to_owned(),
                    device: "/dev/nvme0n1p3".to_owned(),
                    fs_type: "btrfs".to_owned(),
                    total_bytes: 1_024_000_000_000,
                    used_bytes: 184_000_000_000,
                    available_bytes: 840_000_000_000,
                },
            ],
            network_interfaces: vec![
                NetworkInterfaceInfo {
                    name: "eth0".to_owned(),
                    rx_bytes: 184_200_000,
                    tx_bytes: 92_100_000,
                    is_up: true,
                },
                NetworkInterfaceInfo {
                    name: "tailscale0".to_owned(),
                    rx_bytes: 45_000_000,
                    tx_bytes: 31_000_000,
                    is_up: true,
                },
            ],
        }
    }

    /// Query system logs.
    #[must_use]
    pub fn get_logs(&self, query: SystemLogsQueryRequest) -> SystemLogsProjection {
        let all_logs = vec![
            SystemLogEntry {
                timestamp: "2026-08-28T20:30:15Z".to_owned(),
                unit: Some("cybou-web-gateway.service".to_owned()),
                severity: "info".to_owned(),
                message: "Listening on http://127.0.0.1:8080 with sandboxed Axum router".to_owned(),
                pid: Some(1024),
            },
            SystemLogEntry {
                timestamp: "2026-08-28T20:30:16Z".to_owned(),
                unit: Some("cybou-host-filesd.service".to_owned()),
                severity: "info".to_owned(),
                message: "Owner filesystem socket bound at /run/cybou/1000/owner.sock under UID 1000".to_owned(),
                pid: Some(1025),
            },
            SystemLogEntry {
                timestamp: "2026-08-28T20:32:00Z".to_owned(),
                unit: Some("cybou-telemetryd.service".to_owned()),
                severity: "notice".to_owned(),
                message: "Health check all daemons OK: 8 active, 0 failed".to_owned(),
                pid: Some(1029),
            },
            SystemLogEntry {
                timestamp: "2026-08-28T20:35:12Z".to_owned(),
                unit: Some("cybou-agentd.service".to_owned()),
                severity: "info".to_owned(),
                message: "Spawned sandboxed capsule agent-opencode-01 with Landlock + bubblewrap confinement".to_owned(),
                pid: Some(1026),
            },
            SystemLogEntry {
                timestamp: "2026-08-28T20:38:44Z".to_owned(),
                unit: Some("systemd-journald.service".to_owned()),
                severity: "info".to_owned(),
                message: "Journal rotated, 1.2 GB active log archive preserved".to_owned(),
                pid: Some(312),
            },
        ];

        let filtered = all_logs
            .into_iter()
            .filter(|l| {
                if let Some(ref u) = query.unit {
                    l.unit.as_ref().is_some_and(|unit_name| unit_name.contains(u))
                } else {
                    true
                }
            })
            .filter(|l| {
                if let Some(ref s) = query.severity {
                    l.severity.eq_ignore_ascii_case(s)
                } else {
                    true
                }
            })
            .filter(|l| {
                if let Some(ref kw) = query.search {
                    l.message.to_lowercase().contains(&kw.to_lowercase())
                } else {
                    true
                }
            })
            .take(query.limit.unwrap_or(200))
            .collect();

        SystemLogsProjection {
            schema_version: WEB_SCHEMA_V1,
            logs: filtered,
        }
    }

    /// List Btrfs subvolumes and point-in-time snapshots.
    #[must_use]
    pub fn get_storage(&self) -> StorageProjection {
        let subvolumes = vec![
            BtrfsSubvolumeRecord {
                id: 256,
                path: "@root".to_owned(),
                parent_uuid: None,
                is_snapshot: false,
                readonly: false,
            },
            BtrfsSubvolumeRecord {
                id: 257,
                path: "@home".to_owned(),
                parent_uuid: None,
                is_snapshot: false,
                readonly: false,
            },
            BtrfsSubvolumeRecord {
                id: 258,
                path: "@var-log".to_owned(),
                parent_uuid: None,
                is_snapshot: false,
                readonly: false,
            },
        ];

        let snapshots = self.snapshots.read().expect("read snapshots").clone();

        StorageProjection {
            schema_version: WEB_SCHEMA_V1,
            subvolumes,
            snapshots,
            total_space_bytes: 1_536_000_000_000,
            free_space_bytes: 1_309_700_000_000,
        }
    }

    /// Create a new point-in-time Btrfs snapshot.
    pub fn create_snapshot(
        &self,
        subvolume_path: &str,
        name: &str,
        readonly: bool,
    ) -> Result<SnapshotRecord, GatewayError> {
        let mut snaps = self.snapshots.write().expect("write snapshots");
        let id = format!("snap-{}-{}", subvolume_path.trim_start_matches('@'), snaps.len() + 1);
        let record = SnapshotRecord {
            id: id.clone(),
            subvolume_path: subvolume_path.to_owned(),
            name: name.to_owned(),
            timestamp: "2026-08-28T22:30:00Z".to_owned(),
            size_bytes: 350_000_000,
            readonly,
        };
        snaps.push(record.clone());
        Ok(record)
    }

    /// Restore a Btrfs snapshot.
    pub fn restore_snapshot(&self, snapshot_id: &str) -> Result<String, GatewayError> {
        let snaps = self.snapshots.read().expect("read snapshots");
        let found = snaps.iter().find(|s| s.id == snapshot_id).ok_or(GatewayError::NotFound)?;
        Ok(format!("Restored snapshot {} ({}) to {}", found.id, found.name, found.subvolume_path))
    }

    /// List active and available network connections.
    #[must_use]
    pub fn get_network(&self) -> NetworkProjection {
        let connections = self.connections.read().expect("read connections").clone();
        NetworkProjection {
            schema_version: WEB_SCHEMA_V1,
            connections,
        }
    }

    /// Connect or disconnect a network profile.
    pub fn connect_network(&self, connection_id: &str, activate: bool) -> Result<String, GatewayError> {
        let mut conns = self.connections.write().expect("write connections");
        let conn = conns.iter_mut().find(|c| c.id == connection_id).ok_or(GatewayError::NotFound)?;
        conn.is_active = activate;
        let action = if activate { "connected" } else { "disconnected" };
        Ok(format!("Interface {} {}", conn.name, action))
    }

    /// List software packages.
    #[must_use]
    pub fn get_packages(&self) -> PackagesProjection {
        let packages = self.packages.read().expect("read packages").clone();
        let installed = packages.iter().filter(|p| p.status == PackageStatus::Installed || p.status == PackageStatus::Upgradable).count();
        let upgradable = packages.iter().filter(|p| p.status == PackageStatus::Upgradable).count();

        PackagesProjection {
            schema_version: WEB_SCHEMA_V1,
            installed_count: installed,
            upgradable_count: upgradable,
            packages,
        }
    }

    /// Execute a governed package action (install/upgrade/remove).
    pub fn execute_package_action(&self, name: &str, action: PackageActionKind) -> Result<String, GatewayError> {
        let mut pkgs = self.packages.write().expect("write packages");
        let pkg = pkgs.iter_mut().find(|p| p.name == name).ok_or(GatewayError::NotFound)?;

        match action {
            PackageActionKind::Install | PackageActionKind::Reinstall => {
                pkg.status = PackageStatus::Installed;
                pkg.installed_version = pkg.candidate_version.clone();
                Ok(format!("Installed {}", pkg.name))
            }
            PackageActionKind::Upgrade => {
                pkg.status = PackageStatus::Installed;
                pkg.installed_version = pkg.candidate_version.clone();
                Ok(format!("Upgraded {} to latest version", pkg.name))
            }
            PackageActionKind::Remove => {
                pkg.status = PackageStatus::Available;
                pkg.installed_version = None;
                Ok(format!("Removed {}", pkg.name))
            }
        }
    }

    /// Get system updates status summary.
    #[must_use]
    pub fn get_system_updates(&self) -> SystemUpdatesProjection {
        let pkgs = self.packages.read().expect("read packages");
        let upgradable: Vec<PackageRecord> = pkgs.iter().filter(|p| p.status == PackageStatus::Upgradable).cloned().collect();
        let pending_count = upgradable.len();
        let security_updates_count = upgradable.iter().filter(|p| p.repository.contains("security")).count();
        let kernel_update = upgradable.iter().any(|p| p.name.contains("linux-image"));
        let total_download_bytes = upgradable.iter().filter_map(|p| p.download_size_bytes).sum();

        SystemUpdatesProjection {
            schema_version: WEB_SCHEMA_V1,
            summary: SystemUpdatesSummary {
                pending_count,
                security_updates_count,
                kernel_update,
                reboot_required: kernel_update,
                total_download_bytes,
                packages: upgradable,
            },
        }
    }

    /// Apply pending system updates.
    pub fn apply_system_updates(&self, package_names: Option<Vec<String>>) -> Result<String, GatewayError> {
        let mut pkgs = self.packages.write().expect("write packages");
        let mut updated = 0;
        for pkg in pkgs.iter_mut() {
            if pkg.status == PackageStatus::Upgradable {
                let matches = package_names.as_ref().is_none_or(|names| names.contains(&pkg.name));
                if matches {
                    pkg.status = PackageStatus::Installed;
                    pkg.installed_version = pkg.candidate_version.clone();
                    updated += 1;
                }
            }
        }
        Ok(format!("Successfully applied {updated} system updates"))
    }

    /// List user accounts and authorized SSH public keys.
    #[must_use]
    pub fn get_users_settings(&self) -> UsersSettingsProjection {
        let users = self.users.read().expect("read users").clone();
        let ssh_keys = self.ssh_keys.read().expect("read ssh_keys").clone();

        UsersSettingsProjection {
            schema_version: WEB_SCHEMA_V1,
            users,
            ssh_keys,
        }
    }

    /// Create a new local user account.
    pub fn create_user(&self, username: &str, full_name: &str, is_admin: bool) -> Result<UserAccountRecord, GatewayError> {
        let mut users = self.users.write().expect("write users");
        if users.iter().any(|u| u.username == username) {
            return Err(GatewayError::Conflict);
        }
        let next_uid = users.iter().map(|u| u.uid).max().unwrap_or(1000) + 1;
        let mut groups = vec![username.to_owned()];
        if is_admin {
            groups.push("wheel".to_owned());
            groups.push("sudo".to_owned());
        }
        let record = UserAccountRecord {
            uid: next_uid,
            username: username.to_owned(),
            full_name: full_name.to_owned(),
            home_dir: format!("/home/{username}"),
            shell: "/bin/bash".to_owned(),
            groups,
            is_admin,
            is_locked: false,
        };
        users.push(record.clone());
        Ok(record)
    }

    /// Add an authorized SSH public key.
    pub fn add_ssh_key(&self, name: &str, public_key: &str) -> Result<SshKeyRecord, GatewayError> {
        let mut keys = self.ssh_keys.write().expect("write ssh_keys");
        let id = format!("ssh-key-{:02}", keys.len() + 1);
        let key_type = if public_key.starts_with("ssh-ed25519") {
            "ssh-ed25519".to_owned()
        } else if public_key.starts_with("ssh-rsa") {
            "ssh-rsa".to_owned()
        } else {
            "ssh-key".to_owned()
        };
        let record = SshKeyRecord {
            id: id.clone(),
            name: name.to_owned(),
            fingerprint: format!("SHA256:generated-fp-{:x}", keys.len() + 1),
            key_type,
            public_key: public_key.to_owned(),
            created_at: "2026-08-28T22:45:00Z".to_owned(),
        };
        keys.push(record.clone());
        Ok(record)
    }

    /// Delete an authorized SSH public key.
    pub fn delete_ssh_key(&self, key_id: &str) -> Result<String, GatewayError> {
        let mut keys = self.ssh_keys.write().expect("write ssh_keys");
        let initial_len = keys.len();
        keys.retain(|k| k.id != key_id);
        if keys.len() < initial_len {
            Ok(format!("Deleted SSH key {key_id}"))
        } else {
            Err(GatewayError::NotFound)
        }
    }

    /// Get current security confinement policy and recent audit events.
    #[must_use]
    pub fn get_security_settings(&self) -> SecuritySettingsProjection {
        let policy = self.security_policy.read().expect("read security_policy").clone();
        let audit_log = self.security_audit.read().expect("read security_audit").clone();

        SecuritySettingsProjection {
            schema_version: WEB_SCHEMA_V1,
            policy,
            audit_log,
        }
    }

    /// Update security confinement policies.
    pub fn update_security_policy(&self, req: cybou_web_contracts::UpdateSecurityPolicyRequest) -> Result<SecurityPolicyRecord, GatewayError> {
        let mut policy = self.security_policy.write().expect("write security_policy");
        policy.landlock_enabled = req.landlock_enabled;
        policy.bubblewrap_enabled = req.bubblewrap_enabled;
        policy.apparmor_enforcing = req.apparmor_enforcing;
        policy.seccomp_strict = req.seccomp_strict;
        policy.egress_firewall_strict = req.egress_firewall_strict;

        let mut audit = self.security_audit.write().expect("write security_audit");
        audit.push(SecurityAuditEntry {
            timestamp: "2026-08-28T22:50:00Z".to_owned(),
            severity: "notice".to_owned(),
            category: "sandbox".to_owned(),
            message: format!("Security policy updated: Landlock={}, Bubblewrap={}, Seccomp={}", req.landlock_enabled, req.bubblewrap_enabled, req.seccomp_strict),
        });

        Ok(policy.clone())
    }

    /// Get Borg/Btrfs backup repository status, snapshot archives, and automation schedule.
    #[must_use]
    pub fn get_backup_settings(&self) -> BackupSettingsProjection {
        let repository = self.backup_repo.read().expect("read backup_repo").clone();
        let archives = self.backup_archives.read().expect("read backup_archives").clone();
        let schedule = self.backup_schedule.read().expect("read backup_schedule").clone();

        BackupSettingsProjection {
            schema_version: WEB_SCHEMA_V1,
            repository,
            archives,
            schedule,
        }
    }

    /// Trigger an immediate Borg deduplicating backup snapshot.
    pub fn trigger_backup(&self, name: Option<String>) -> Result<BackupArchiveRecord, GatewayError> {
        let mut archives = self.backup_archives.write().expect("write backup_archives");
        let snap_name = name.unwrap_or_else(|| format!("snapshot-{}", archives.len() + 1));
        let id = format!("arch-{}", snap_name.replace(' ', "-").to_lowercase());
        let record = BackupArchiveRecord {
            id: id.clone(),
            name: snap_name,
            timestamp: "2026-08-28T22:55:00Z".to_owned(),
            size_bytes: 3_900_000_000,
            duration_seconds: 32,
        };
        archives.push(record.clone());

        let mut repo = self.backup_repo.write().expect("write backup_repo");
        repo.total_archives = archives.len();
        repo.last_backup_time = Some("2026-08-28T22:55:00Z".to_owned());
        repo.total_size_bytes += 150_000_000; // Deduplicated differential increment

        Ok(record)
    }

    /// Restore files from a Borg backup archive.
    pub fn restore_archive(&self, archive_id: &str, target_path: Option<String>) -> Result<String, GatewayError> {
        let archives = self.backup_archives.read().expect("read backup_archives");
        let found = archives.iter().find(|a| a.id == archive_id).ok_or(GatewayError::NotFound)?;
        let dest = target_path.unwrap_or_else(|| "/home/cybou".to_owned());
        Ok(format!("Restored archive {} ({}) to {}", found.id, found.name, dest))
    }

    /// Update automated backup schedule and retention policy.
    pub fn update_backup_schedule(&self, req: cybou_web_contracts::UpdateBackupScheduleRequest) -> Result<BackupScheduleRecord, GatewayError> {
        let mut schedule = self.backup_schedule.write().expect("write backup_schedule");
        schedule.enabled = req.enabled;
        schedule.frequency = req.frequency;
        schedule.retention_daily = req.retention_daily;
        schedule.retention_weekly = req.retention_weekly;
        schedule.retention_monthly = req.retention_monthly;
        Ok(schedule.clone())
    }
}
