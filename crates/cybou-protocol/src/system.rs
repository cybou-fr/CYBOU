// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Strongly-typed system models for services, processes, hardware telemetry, and logs.

use serde::{Deserialize, Serialize};

/// State of a systemd unit or daemon service.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceState {
    /// Actively running.
    Active,
    /// Inactive or stopped.
    Inactive,
    /// Failed or in error state.
    Failed,
    /// Activating or starting up.
    Activating,
    /// Deactivating or shutting down.
    Deactivating,
    /// Reloading configuration.
    Reloading,
    /// Unknown or unobserved state.
    Unknown,
}

impl ServiceState {
    /// Human-readable label for UI badge.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Inactive => "Inactive",
            Self::Failed => "Failed",
            Self::Activating => "Activating",
            Self::Deactivating => "Deactivating",
            Self::Reloading => "Reloading",
            Self::Unknown => "Unknown",
        }
    }
}

/// Unit type of a system service.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceUnitType {
    /// Traditional daemon service.
    Service,
    /// Socket-activated listener.
    Socket,
    /// Target group.
    Target,
    /// Scheduled timer unit.
    Timer,
    /// Mount unit.
    Mount,
    /// Other unit type.
    Other,
}

/// A system service record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ServiceRecord {
    /// Canonical unit name (e.g. `cybou-web-gateway.service`).
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// High-level service state.
    pub state: ServiceState,
    /// Systemd substate string (e.g. `running`, `dead`, `failed`).
    pub substate: String,
    /// Whether unit is enabled at boot.
    pub enabled: bool,
    /// Main daemon PID if active.
    pub main_pid: Option<u32>,
    /// Resident memory in bytes if active.
    pub memory_bytes: Option<u64>,
    /// Unit classification.
    pub unit_type: ServiceUnitType,
}

/// Permitted action on a system service.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceAction {
    /// Start an inactive service.
    Start,
    /// Stop an active service.
    Stop,
    /// Restart a service.
    Restart,
    /// Reload configuration.
    Reload,
    /// Enable service at boot.
    Enable,
    /// Disable service at boot.
    Disable,
}

/// An operating system process record.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ProcessRecord {
    /// Process ID.
    pub pid: u32,
    /// Parent Process ID.
    pub ppid: u32,
    /// Process executable name.
    pub name: String,
    /// Full command line arguments.
    pub cmdline: String,
    /// User name executing the process.
    pub user: String,
    /// Instantaneous CPU usage percent (0.0 - 100.0%).
    pub cpu_percent: f32,
    /// Resident set size (RSS) memory in bytes.
    pub memory_bytes: u64,
    /// Percentage of total RAM consumed.
    pub memory_percent: f32,
    /// Process execution state string (e.g. `Running`, `Sleeping`, `Zombie`, `Idle`).
    pub state: String,
    /// Active thread count.
    pub threads: u32,
}

/// Signal sent to an operating system process.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessSignal {
    /// Graceful termination request (SIGTERM).
    Terminate,
    /// Immediate forced kill (SIGKILL).
    Kill,
    /// Suspend process execution (SIGSTOP).
    Pause,
    /// Resume process execution (SIGCONT).
    Resume,
}

/// Per-core CPU utilization metric.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CpuCoreStat {
    /// Logical core index.
    pub core_id: usize,
    /// Core utilization percentage (0.0 - 100.0%).
    pub usage_percent: f32,
}

/// Filesystem storage partition metrics.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DiskPartitionInfo {
    /// Mount path (e.g. `/` or `/home`).
    pub mount_point: String,
    /// Device name (e.g. `/dev/nvme0n1p2`).
    pub device: String,
    /// Filesystem type (e.g. `btrfs`, `ext4`, `zfs`).
    pub fs_type: String,
    /// Total capacity in bytes.
    pub total_bytes: u64,
    /// Used capacity in bytes.
    pub used_bytes: u64,
    /// Available capacity in bytes.
    pub available_bytes: u64,
}

/// Network interface telemetry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NetworkInterfaceInfo {
    /// Interface name (e.g. `eth0`, `wlan0`, `tailscale0`).
    pub name: String,
    /// Total received bytes.
    pub rx_bytes: u64,
    /// Total transmitted bytes.
    pub tx_bytes: u64,
    /// Interface link state.
    pub is_up: bool,
}

/// A system log entry from journald or system daemons.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SystemLogEntry {
    /// UTC timestamp of log entry.
    pub timestamp: String,
    /// Originating unit or daemon name.
    pub unit: Option<String>,
    /// Syslog severity level (`emerg`, `alert`, `crit`, `err`, `warning`, `notice`, `info`, `debug`).
    pub severity: String,
    /// Log line text.
    pub message: String,
    /// Process ID if available.
    pub pid: Option<u32>,
}

/// Why a log projection carries no entries, when the reason is not "there were none".
///
/// A viewer that draws an empty feed for a reader who is merely not permitted to see the journal
/// reports silence on a machine that is talking. The distinction is on the wire so the card can
/// say which of the two it is looking at.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LogsUnavailable {
    /// This build is not running on Linux, so there is no journal to read.
    NotLinux,
    /// No `journalctl` was found on PATH.
    ReaderMissing,
    /// The reader ran and was refused: this process is not in `systemd-journal`.
    ReaderRefused,
    /// The reader could not be run, or exited for a reason that is not a refusal.
    ReaderFailed,
}

impl LogsUnavailable {
    /// One line an operator can act on.
    #[must_use]
    pub const fn explain(self) -> &'static str {
        match self {
            Self::NotLinux => "not a Linux host: there is no journal to read",
            Self::ReaderMissing => "journalctl was not found on PATH",
            Self::ReaderRefused => {
                "the journal refused this reader: add the gateway account to the systemd-journal group"
            }
            Self::ReaderFailed => "the journal reader could not be run",
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Storage Substrate (Btrfs Subvolumes & Snapshots)
// -------------------------------------------------------------------------------------------------

/// A Btrfs subvolume or snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BtrfsSubvolumeRecord {
    /// Subvolume internal ID.
    pub id: u64,
    /// Subvolume relative mount path.
    pub path: String,
    /// UUID of parent snapshot if derived.
    pub parent_uuid: Option<String>,
    /// Whether this subvolume is a point-in-time snapshot.
    pub is_snapshot: bool,
    /// Read-only snapshot flag.
    pub readonly: bool,
}

/// A point-in-time filesystem snapshot record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SnapshotRecord {
    /// Unique snapshot identifier.
    pub id: String,
    /// Target subvolume path (e.g. `@home` or `@root`).
    pub subvolume_path: String,
    /// User-facing snapshot label or name.
    pub name: String,
    /// UTC creation timestamp.
    pub timestamp: String,
    /// Exclusive snapshot size in bytes.
    pub size_bytes: u64,
    /// Read-only lock.
    pub readonly: bool,
}

// -------------------------------------------------------------------------------------------------
// Network Substrate (Interfaces, Wi-Fi, VPNs)
// -------------------------------------------------------------------------------------------------

/// Network connection type.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkConnectionKind {
    /// Wired Ethernet.
    Ethernet,
    /// Wireless Wi-Fi.
    Wifi,
    /// Tailscale Mesh VPN.
    Tailscale,
    /// `WireGuard` Point-to-Point Tunnel.
    Wireguard,
    /// Local Loopback.
    Loopback,
}

/// Network connection profile and active state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NetworkConnectionRecord {
    /// Connection identifier.
    pub id: String,
    /// Interface name (e.g. `eth0`, `tailscale0`).
    pub name: String,
    /// Connection type.
    pub kind: NetworkConnectionKind,
    /// Link state active status.
    pub is_active: bool,
    /// Primary assigned IPv4/IPv6 address with CIDR.
    pub ip_address: Option<String>,
    /// Default gateway IP.
    pub gateway: Option<String>,
    /// Configured DNS nameservers.
    pub dns: Vec<String>,
    /// Total received bytes.
    pub rx_bytes: u64,
    /// Total transmitted bytes.
    pub tx_bytes: u64,
}

// -------------------------------------------------------------------------------------------------
// Packages & Governed System Updates
// -------------------------------------------------------------------------------------------------

/// Package installation status.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageStatus {
    /// Installed and current.
    Installed,
    /// Installed with pending upgrade.
    Upgradable,
    /// Available in repository.
    Available,
    /// Removed or uninstalled.
    Removed,
}

/// System software package record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PackageRecord {
    /// Package name (e.g. `ripgrep`, `btrfs-progs`).
    pub name: String,
    /// Current installed version if present.
    pub installed_version: Option<String>,
    /// Repository candidate version for upgrade or install.
    pub candidate_version: Option<String>,
    /// Package summary description.
    pub description: String,
    /// CPU architecture (e.g. `x86_64`, `aarch64`).
    pub architecture: String,
    /// Originating repository (e.g. `cybou-main`, `debian-bookworm`, `crates-io`).
    pub repository: String,
    /// Installation status.
    pub status: PackageStatus,
    /// Approximate download size in bytes.
    pub download_size_bytes: Option<u64>,
}

/// Permitted governed action on a package.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageActionKind {
    /// Install an uninstalled package.
    Install,
    /// Upgrade an existing package.
    Upgrade,
    /// Remove an installed package.
    Remove,
    /// Reinstall/repair package files.
    Reinstall,
}

/// System-wide update status summary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SystemUpdatesSummary {
    /// Total pending update package count.
    pub pending_count: usize,
    /// Security-critical updates count.
    pub security_updates_count: usize,
    /// Whether a new Linux kernel update is pending.
    pub kernel_update: bool,
    /// Whether system reboot is required after update.
    pub reboot_required: bool,
    /// Total download payload in bytes.
    pub total_download_bytes: u64,
    /// Packages with pending updates.
    pub packages: Vec<PackageRecord>,
}

// -------------------------------------------------------------------------------------------------
// Users & Security Substrate (Accounts, PAM, SSH Keys, Sandboxing) (Milestone 5)
// -------------------------------------------------------------------------------------------------

/// A Linux user account record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UserAccountRecord {
    /// Linux UID.
    pub uid: u32,
    /// Primary username.
    pub username: String,
    /// Display full name (GECOS).
    pub full_name: String,
    /// Home directory absolute path.
    pub home_dir: String,
    /// Default login shell.
    pub shell: String,
    /// Supplementary group memberships.
    pub groups: Vec<String>,
    /// Whether the user has administrative/sudo rights.
    pub is_admin: bool,
    /// Account lock status.
    pub is_locked: bool,
}

/// An authorized SSH public key record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SshKeyRecord {
    /// Key identifier.
    pub id: String,
    /// User-friendly key name or comment.
    pub name: String,
    /// Key fingerprint (e.g. SHA256:...).
    pub fingerprint: String,
    /// Key algorithm type (e.g. `ssh-ed25519`, `ssh-rsa`).
    pub key_type: String,
    /// Public key string.
    pub public_key: String,
    /// Creation/addition timestamp.
    pub created_at: String,
}

/// System sandboxing and confinement security policy.
///
/// Five independent switches rather than one state: each names a different kernel mechanism, and
/// collapsing them into an enum would have to invent an order of severity between confinements
/// that do not compare.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[allow(clippy::struct_excessive_bools, reason = "one field per kernel confinement mechanism")]
pub struct SecurityPolicyRecord {
    /// Linux Landlock filesystem sandbox status.
    pub landlock_enabled: bool,
    /// Bubblewrap unprivileged user namespace isolation status.
    pub bubblewrap_enabled: bool,
    /// `AppArmor` LSM enforcement status.
    pub apparmor_enforcing: bool,
    /// Strict Seccomp BPF syscall filter status.
    pub seccomp_strict: bool,
    /// Strict network egress firewall enforcement.
    pub egress_firewall_strict: bool,
}

/// Security audit log entry.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SecurityAuditEntry {
    /// UTC timestamp of event.
    pub timestamp: String,
    /// Severity level (`info`, `warning`, `critical`).
    pub severity: String,
    /// Category (`auth`, `sandbox`, `network`, `privilege`).
    pub category: String,
    /// Event description.
    pub message: String,
}

// -------------------------------------------------------------------------------------------------
// Backup & Retention Substrate (Borg / Btrfs) (Milestone 5)
// -------------------------------------------------------------------------------------------------

/// A Borg deduplicating backup repository.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackupRepositoryRecord {
    /// Repository identifier.
    pub id: String,
    /// Repository display name.
    pub name: String,
    /// Destination path or SSH URI (e.g. `/var/backups/cybou.borg` or `borg@remote:...`).
    pub destination: String,
    /// Encryption mode (e.g. `repokey-blake2-chacha20-poly1305`, `none`).
    pub encryption: String,
    /// Timestamp of most recent backup run.
    pub last_backup_time: Option<String>,
    /// Total archive snapshots count in repository.
    pub total_archives: usize,
    /// Deduplicated repository size on disk in bytes.
    pub total_size_bytes: u64,
}

/// A single snapshot archive within a backup repository.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackupArchiveRecord {
    /// Archive identifier or name.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Archive completion timestamp.
    pub timestamp: String,
    /// Uncompressed source size in bytes.
    pub size_bytes: u64,
    /// Duration of backup process in seconds.
    pub duration_seconds: u32,
}

/// Automated backup schedule and pruning retention policy.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BackupScheduleRecord {
    /// Whether automated backups are enabled.
    pub enabled: bool,
    /// Frequency schedule (`hourly`, `daily`, `weekly`).
    pub frequency: String,
    /// Number of daily archives to retain.
    pub retention_daily: u32,
    /// Number of weekly archives to retain.
    pub retention_weekly: u32,
    /// Number of monthly archives to retain.
    pub retention_monthly: u32,
}
