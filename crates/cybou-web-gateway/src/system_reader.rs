// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Grounded Linux system and hardware reader for real OS observation without mock fixtures.

use std::{collections::HashMap, fs, path::Path};

use cybou_protocol::system::LogsUnavailable;
use cybou_protocol::system::{
    CpuCoreStat, DiskPartitionInfo, NetworkInterfaceInfo, ProcessRecord, ServiceRecord,
    ServiceState, ServiceUnitType,
};
use cybou_web_contracts::{
    ProcessesListProjection, ServicesListProjection, SystemLogsProjection, SystemLogsQueryRequest,
    SystemMonitorProjection, WEB_SCHEMA_V1,
};

/// Reads real system monitor metrics from Linux /proc and /sys.
#[must_use]
pub fn read_real_monitor() -> SystemMonitorProjection {
    #[cfg(target_os = "linux")]
    {
        let hostname = fs::read_to_string("/proc/sys/kernel/hostname")
            .or_else(|_| fs::read_to_string("/etc/hostname"))
            .unwrap_or_else(|_| "cybou-host".to_owned())
            .trim()
            .to_owned();

        let os_release = read_os_pretty_name();
        let uptime_seconds = read_uptime_seconds();
        let load_avg = read_load_avg();

        let (
            memory_total_bytes,
            memory_used_bytes,
            memory_free_bytes,
            swap_total_bytes,
            swap_used_bytes,
        ) = read_meminfo();
        let cores = read_cpu_cores();
        let total_cpu_percent = if cores.is_empty() {
            0.0f32
        } else {
            cores.iter().map(|c| c.usage_percent).sum::<f32>() / cores.len() as f32
        };

        let disk_partitions = read_disk_partitions();
        let network_interfaces = read_network_interfaces();

        SystemMonitorProjection {
            schema_version: WEB_SCHEMA_V1,
            hostname,
            os_release,
            uptime_seconds,
            load_avg,
            total_cpu_percent,
            cores,
            memory_total_bytes,
            memory_used_bytes,
            memory_free_bytes,
            swap_total_bytes,
            swap_used_bytes,
            disk_partitions,
            network_interfaces,
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        SystemMonitorProjection {
            schema_version: WEB_SCHEMA_V1,
            hostname: "cybou-host".to_owned(),
            os_release: "Linux (unobserved)".to_owned(),
            uptime_seconds: 0,
            load_avg: [0.0, 0.0, 0.0],
            total_cpu_percent: 0.0,
            cores: Vec::new(),
            memory_total_bytes: 0,
            memory_used_bytes: 0,
            memory_free_bytes: 0,
            swap_total_bytes: 0,
            swap_used_bytes: 0,
            disk_partitions: Vec::new(),
            network_interfaces: Vec::new(),
        }
    }
}

/// Reads real operating system processes from /proc.
#[must_use]
pub fn read_real_processes() -> ProcessesListProjection {
    #[cfg(target_os = "linux")]
    {
        let users_map = load_users_map();
        let mut processes = Vec::new();

        if let Ok(entries) = fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let name_str = file_name.to_string_lossy();
                if let Ok(pid) = name_str.parse::<u32>()
                    && let Some(proc) = read_single_process(pid, &users_map)
                {
                    processes.push(proc);
                }
            }
        }

        processes.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));

        let total_cpu = processes.iter().map(|p| p.cpu_percent).sum();
        let total_mem = processes.iter().map(|p| p.memory_bytes).sum();
        let total_count = processes.len();
        processes.truncate(500);
        let showing_count = processes.len();

        ProcessesListProjection {
            schema_version: WEB_SCHEMA_V1,
            total_count,
            showing_count,
            truncated: showing_count < total_count,
            total_cpu_percent: total_cpu,
            total_memory_bytes: total_mem,
            processes,
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        ProcessesListProjection {
            schema_version: WEB_SCHEMA_V1,
            total_count: 0,
            showing_count: 0,
            truncated: false,
            total_cpu_percent: 0.0,
            total_memory_bytes: 0,
            processes: Vec::new(),
        }
    }
}

// -------------------------------------------------------------------------------------------------
// Journal
// -------------------------------------------------------------------------------------------------

/// The most entries one read will ever return, whatever a caller asks for.
const LOG_LIMIT_MAX: usize = 500;

/// What a caller gets when it names no limit.
const LOG_LIMIT_DEFAULT: usize = 200;

/// How many entries are fetched per entry a search is allowed to keep.
///
/// A substring filter is applied here rather than handed to `journalctl --grep`, which takes a
/// regular expression: a search box wired to a regex engine on the host is a way for a browser to
/// choose how long the journal spends answering. The cost is that the filter runs after the fetch,
/// so a search asks for more than it keeps.
const LOG_SEARCH_OVERFETCH: usize = 10;

/// The most entries a search will fetch before filtering, however many it was allowed to keep.
const LOG_SEARCH_FETCH_MAX: usize = 5000;

/// Syslog severity names, indexed by their numeric priority.
const SEVERITY_NAMES: [&str; 8] = [
    "emerg", "alert", "crit", "err", "warning", "notice", "info", "debug",
];

/// The numeric priority of a syslog severity name, if it is one.
///
/// The set is closed: a name outside it is a caller error rather than a filter that quietly
/// matches everything.
#[must_use]
pub fn severity_priority(severity: &str) -> Option<u8> {
    SEVERITY_NAMES
        .iter()
        .position(|name| *name == severity)
        .and_then(|index| u8::try_from(index).ok())
}

/// The syslog severity name for a numeric priority.
fn severity_name(priority: u8) -> &'static str {
    SEVERITY_NAMES
        .get(usize::from(priority))
        .copied()
        .unwrap_or("info")
}

/// An empty feed that says why it is empty.
fn unavailable_logs(reason: LogsUnavailable) -> SystemLogsProjection {
    SystemLogsProjection {
        schema_version: WEB_SCHEMA_V1,
        logs: Vec::new(),
        unavailable: Some(reason),
        system_journal_readable: false,
    }
}

/// Reads the systemd journal, or says why it could not.
///
/// `priority` is a floor rather than an exact level: `err` means *this bad or worse*, which is
/// `journalctl`'s own reading of `--priority` and the only one under which an "Errors" filter does
/// not hide the emergencies above it.
#[must_use]
pub fn read_journal(query: &SystemLogsQueryRequest, priority: Option<u8>) -> SystemLogsProjection {
    #[cfg(target_os = "linux")]
    {
        let limit = query
            .limit
            .unwrap_or(LOG_LIMIT_DEFAULT)
            .clamp(1, LOG_LIMIT_MAX);
        let search = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|needle| !needle.is_empty())
            .map(str::to_lowercase);

        // A filtered read keeps at most `limit`, so it has to look at more than `limit`.
        let fetch = if search.is_some() {
            limit
                .saturating_mul(LOG_SEARCH_OVERFETCH)
                .min(LOG_SEARCH_FETCH_MAX)
        } else {
            limit
        };

        let mut command = std::process::Command::new("journalctl");
        command
            .arg("--output=json")
            .arg("--no-pager")
            // Without this, journalctl elides long fields and the message a person opened the
            // journal for arrives as a note saying it was too long.
            .arg("--all")
            .arg(format!("--lines={fetch}"));

        // `--unit=VALUE` is one argv token, so a value beginning with `-` cannot become a second
        // option, and there is no shell between here and the process for it to reach.
        if let Some(unit) = query
            .unit
            .as_deref()
            .map(str::trim)
            .filter(|unit| !unit.is_empty())
        {
            command.arg(format!("--unit={unit}"));
        }
        if let Some(priority) = priority {
            command.arg(format!("--priority={priority}"));
        }

        let output = match command.output() {
            Ok(output) => output,
            Err(error) => {
                let reason = match error.kind() {
                    std::io::ErrorKind::NotFound => LogsUnavailable::ReaderMissing,
                    std::io::ErrorKind::PermissionDenied => LogsUnavailable::ReaderRefused,
                    _ => LogsUnavailable::ReaderFailed,
                };
                return unavailable_logs(reason);
            }
        };

        if !output.status.success() {
            return unavailable_logs(LogsUnavailable::ReaderFailed);
        }

        let mut logs = Vec::new();
        for line in output.stdout.split(|byte| *byte == b'\n') {
            if line.is_empty() {
                continue;
            }
            let Ok(record) = serde_json::from_slice::<serde_json::Value>(line) else {
                continue;
            };
            let Some(entry) = journal_entry(&record) else {
                continue;
            };
            if let Some(needle) = search.as_deref() {
                let matched = entry.message.to_lowercase().contains(needle)
                    || entry
                        .unit
                        .as_deref()
                        .is_some_and(|unit| unit.to_lowercase().contains(needle));
                if !matched {
                    continue;
                }
            }
            logs.push(entry);
        }

        // Over-fetching for a search collects the oldest matches too; the newest are the ones a
        // person opened the viewer for.
        if logs.len() > limit {
            logs.drain(..logs.len() - limit);
        }

        SystemLogsProjection {
            schema_version: WEB_SCHEMA_V1,
            logs,
            unavailable: None,
            system_journal_readable: system_journal_readable(),
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (query, priority);
        unavailable_logs(LogsUnavailable::NotLinux)
    }
}

/// Whether the persistent or volatile system journal is readable by this process.
///
/// Asked of the filesystem rather than of `journalctl`'s stderr hint, which is prose and is
/// translated. `journalctl` does not fail for a reader outside the `systemd-journal` group; it
/// narrows to that account's own entries and says so only in a hint.
#[cfg(target_os = "linux")]
fn system_journal_readable() -> bool {
    ["/var/log/journal", "/run/log/journal"]
        .iter()
        .any(|path| fs::read_dir(path).is_ok())
}

/// One journal record, or nothing if it carries no message.
#[cfg(target_os = "linux")]
fn journal_entry(record: &serde_json::Value) -> Option<cybou_protocol::system::SystemLogEntry> {
    let message = journal_text(record.get("MESSAGE")?)?;

    let timestamp = record
        .get("__REALTIME_TIMESTAMP")
        .and_then(journal_text)
        .and_then(|raw| raw.parse::<i128>().ok())
        .and_then(|micros| time::OffsetDateTime::from_unix_timestamp_nanos(micros * 1_000).ok())
        .and_then(|instant| {
            instant
                .format(&time::format_description::well_known::Rfc3339)
                .ok()
        })
        .unwrap_or_else(|| "unknown".to_owned());

    let unit = record
        .get("_SYSTEMD_UNIT")
        .or_else(|| record.get("SYSLOG_IDENTIFIER"))
        .and_then(journal_text);

    let severity = record
        .get("PRIORITY")
        .and_then(journal_text)
        .and_then(|raw| raw.parse::<u8>().ok())
        .map_or("info", severity_name)
        .to_owned();

    let pid = record
        .get("_PID")
        .and_then(journal_text)
        .and_then(|raw| raw.parse::<u32>().ok());

    Some(cybou_protocol::system::SystemLogEntry {
        timestamp,
        unit,
        severity,
        message,
        pid,
    })
}

/// A journal field as text.
///
/// A field that is not valid UTF-8 arrives as an array of byte values rather than a string, so a
/// reader that only handled strings would drop exactly the lines carrying something unusual.
#[cfg(target_os = "linux")]
fn journal_text(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Array(bytes) => {
            let raw: Vec<u8> = bytes
                .iter()
                .filter_map(serde_json::Value::as_u64)
                .filter_map(|byte| u8::try_from(byte).ok())
                .collect();
            Some(String::from_utf8_lossy(&raw).into_owned())
        }
        _ => None,
    }
}

/// Reads real status of Cybou systemd services.
#[must_use]
pub fn read_real_services() -> ServicesListProjection {
    let known_services = [
        (
            "cybou-web-gateway.service",
            "CYBOU Web Gateway & Browser Desktop API",
        ),
        (
            "cybou-host-filesd@demo.service",
            "CYBOU Host User Filesystem Daemon (demo)",
        ),
        (
            "cybou-agentd.service",
            "CYBOU Agent Capsule Isolation Manager",
        ),
        (
            "cybou-actiond.service",
            "CYBOU Governed Action1 Execution Authority",
        ),
        (
            "cybou-presenced.service",
            "CYBOU Presence1 Event Stream Hub",
        ),
        (
            "cybou-telemetryd.service",
            "CYBOU Telemetry1 Diagnostic Engine",
        ),
        (
            "cybou-eventd.service",
            "CYBOU Event1 Canonical Journal Writer",
        ),
        (
            "cybou-identityd.service",
            "CYBOU Identity1 Subject Continuity Service",
        ),
        (
            "cybou-healthd.service",
            "CYBOU Health1 Capability Health Service",
        ),
        (
            "cybou-intentiond.service",
            "CYBOU Intention1 Commitments and Obligations",
        ),
        (
            "cybou-predictord.service",
            "CYBOU Predictor1 Empirical Forecasting Engine",
        ),
        (
            "cybou-perceptiond.service",
            "CYBOU Perception1 Linux Observation Service",
        ),
        (
            "cybou-epistemicd.service",
            "CYBOU Epistemic1 Knowledge Projection Service",
        ),
        (
            "cybou-contextd.service",
            "CYBOU Context1 Associative Context Service",
        ),
        (
            "cybou-meaningd.service",
            "CYBOU Meaning1 Meaning Boundary Service",
        ),
        (
            "cybou-model-brokerd.service",
            "CYBOU ModelBroker1 Model Gateway",
        ),
        (
            "cybou-workspaced.service",
            "CYBOU Workspace1 Global Attention Service",
        ),
        (
            "cybou-lifecycled.service",
            "CYBOU Lifecycle1 Sleep/Wake Consolidation",
        ),
        (
            "cybou-selfd.service",
            "CYBOU Self1 Continuous Self-Model Service",
        ),
        (
            "cybou-shelld.service",
            "CYBOU Shell1 Sandboxed Execution Service",
        ),
        (
            "cybou-remediationd.service",
            "CYBOU Remediation1 Finding Resolution",
        ),
        (
            "cybou-executord.service",
            "CYBOU Typed Body Action Executor",
        ),
        ("caddy.service", "Caddy Sovereign TLS & Reverse Proxy"),
    ];

    #[cfg(target_os = "linux")]
    {
        let mut services = Vec::new();
        let mut active_count = 0;
        let mut failed_count = 0;

        for (name, desc) in known_services {
            let (state, substate, pid, mem) = query_unit_status(name);
            if state == ServiceState::Active {
                active_count += 1;
            } else if state == ServiceState::Failed {
                failed_count += 1;
            }
            services.push(ServiceRecord {
                name: name.to_owned(),
                description: desc.to_owned(),
                state,
                substate,
                enabled: true,
                main_pid: pid,
                memory_bytes: mem,
                unit_type: ServiceUnitType::Service,
            });
        }

        ServicesListProjection {
            schema_version: WEB_SCHEMA_V1,
            services,
            active_count,
            failed_count,
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let services: Vec<ServiceRecord> = known_services
            .into_iter()
            .map(|(name, desc)| ServiceRecord {
                name: name.to_owned(),
                description: desc.to_owned(),
                state: ServiceState::Unknown,
                substate: "unobserved".to_owned(),
                enabled: false,
                main_pid: None,
                memory_bytes: None,
                unit_type: ServiceUnitType::Service,
            })
            .collect();

        ServicesListProjection {
            schema_version: WEB_SCHEMA_V1,
            services,
            active_count: 0,
            failed_count: 0,
        }
    }
}

// ---------------- Helper functions for Linux procfs / sysfs ----------------

#[cfg(target_os = "linux")]
fn read_os_pretty_name() -> String {
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if let Some(val) = line.strip_prefix("PRETTY_NAME=") {
                return val.trim_matches('"').to_owned();
            }
        }
    }
    "Linux (Debian)".to_owned()
}

#[cfg(target_os = "linux")]
fn read_uptime_seconds() -> u64 {
    if let Ok(content) = fs::read_to_string("/proc/uptime")
        && let Some(first) = content.split_whitespace().next()
        && let Ok(secs) = first.parse::<f64>()
    {
        return secs as u64;
    }
    0
}

#[cfg(target_os = "linux")]
fn read_load_avg() -> [f32; 3] {
    if let Ok(content) = fs::read_to_string("/proc/loadavg") {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 3 {
            let l1 = parts[0].parse::<f32>().unwrap_or(0.0);
            let l5 = parts[1].parse::<f32>().unwrap_or(0.0);
            let l15 = parts[2].parse::<f32>().unwrap_or(0.0);
            return [l1, l5, l15];
        }
    }
    [0.0, 0.0, 0.0]
}

#[cfg(target_os = "linux")]
fn read_meminfo() -> (u64, u64, u64, u64, u64) {
    let mut total_mem = 0u64;
    let mut free_mem = 0u64;
    let mut available_mem = 0u64;
    let mut total_swap = 0u64;
    let mut free_swap = 0u64;

    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            let mut parts = line.split_whitespace();
            let key = parts.next().unwrap_or_default();
            let val = parts
                .next()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);
            match key {
                "MemTotal:" => total_mem = val * 1024,
                "MemFree:" => free_mem = val * 1024,
                "MemAvailable:" => available_mem = val * 1024,
                "SwapTotal:" => total_swap = val * 1024,
                "SwapFree:" => free_swap = val * 1024,
                _ => {}
            }
        }
    }

    let used_mem = if available_mem > 0 {
        total_mem.saturating_sub(available_mem)
    } else {
        total_mem.saturating_sub(free_mem)
    };
    let used_swap = total_swap.saturating_sub(free_swap);

    (total_mem, used_mem, available_mem, total_swap, used_swap)
}

#[cfg(target_os = "linux")]
fn read_cpu_cores() -> Vec<CpuCoreStat> {
    let mut cores = Vec::new();
    if let Ok(content) = fs::read_to_string("/proc/stat") {
        for line in content.lines() {
            if line.starts_with("cpu") && !line.starts_with("cpu ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    let core_id = parts[0]
                        .trim_start_matches("cpu")
                        .parse::<usize>()
                        .unwrap_or(0);
                    let user: u64 = parts[1].parse().unwrap_or(0);
                    let nice: u64 = parts[2].parse().unwrap_or(0);
                    let system: u64 = parts[3].parse().unwrap_or(0);
                    let idle: u64 = parts[4].parse().unwrap_or(0);
                    let total = user + nice + system + idle;
                    let busy = user + nice + system;
                    let usage_percent = if total > 0 {
                        (busy as f32 / total as f32) * 100.0
                    } else {
                        0.0
                    };
                    cores.push(CpuCoreStat {
                        core_id,
                        usage_percent,
                    });
                }
            }
        }
    }
    cores
}

#[cfg(target_os = "linux")]
fn read_disk_partitions() -> Vec<DiskPartitionInfo> {
    let mut disks = Vec::new();
    for (mount_point, dev_name) in [("/", "/dev/root"), ("/home", "/dev/home")] {
        if let Ok(stat) = rustix::fs::statvfs(mount_point) {
            let block_size = stat.f_frsize;
            let total_bytes = stat.f_blocks.saturating_mul(block_size);
            let available_bytes = stat.f_bavail.saturating_mul(block_size);
            let used_bytes = total_bytes.saturating_sub(available_bytes);
            disks.push(DiskPartitionInfo {
                mount_point: mount_point.to_owned(),
                device: dev_name.to_owned(),
                fs_type: "btrfs/ext4".to_owned(),
                total_bytes,
                used_bytes,
                available_bytes,
            });
        }
    }
    disks
}

#[cfg(target_os = "linux")]
fn read_network_interfaces() -> Vec<NetworkInterfaceInfo> {
    let mut interfaces = Vec::new();
    if let Ok(content) = fs::read_to_string("/proc/net/dev") {
        for line in content.lines().skip(2) {
            let mut parts = line.split(':');
            if let (Some(iface), Some(stats_str)) = (parts.next(), parts.next()) {
                let name = iface.trim().to_owned();
                if name == "lo" {
                    continue;
                }
                let stats: Vec<u64> = stats_str
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if stats.len() >= 10 {
                    let rx_bytes = stats[0];
                    let tx_bytes = stats[8];
                    interfaces.push(NetworkInterfaceInfo {
                        name,
                        rx_bytes,
                        tx_bytes,
                        is_up: true,
                    });
                }
            }
        }
    }
    interfaces
}

#[cfg(target_os = "linux")]
/// The user id `/etc/passwd` gives this account name, if it names one.
///
/// The seat arrives as a name because that is what somebody signs in as; every check further down
/// is about a number, because that is what `/proc` reports. This is the one place the two meet.
#[must_use]
pub fn uid_for_user(name: &str) -> Option<u32> {
    #[cfg(target_os = "linux")]
    {
        let content = fs::read_to_string("/etc/passwd").ok()?;
        content.lines().find_map(|line| {
            let mut parts = line.split(':');
            let user = parts.next()?;
            let _password = parts.next()?;
            let uid = parts.next()?;
            (user == name).then(|| uid.parse().ok())?
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = name;
        None
    }
}

/// The user id `/proc` reports as the real owner of a running process.
#[must_use]
pub fn owner_of_process(pid: u32) -> Option<u32> {
    #[cfg(target_os = "linux")]
    {
        let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
        status
            .lines()
            .find_map(|line| line.strip_prefix("Uid:"))
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|uid| uid.parse().ok())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        None
    }
}

#[cfg(target_os = "linux")]
fn load_users_map() -> HashMap<u32, String> {
    let mut map = HashMap::new();
    if let Ok(content) = fs::read_to_string("/etc/passwd") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                let name = parts[0].to_owned();
                if let Ok(uid) = parts[2].parse::<u32>() {
                    map.insert(uid, name);
                }
            }
        }
    }
    map
}

#[cfg(target_os = "linux")]
fn read_single_process(pid: u32, users_map: &HashMap<u32, String>) -> Option<ProcessRecord> {
    let proc_path = format!("/proc/{pid}");
    if !Path::new(&proc_path).exists() {
        return None;
    }

    let stat_content = fs::read_to_string(format!("{proc_path}/stat")).ok()?;
    let stat_parts: Vec<&str> = stat_content.split_whitespace().collect();
    if stat_parts.len() < 24 {
        return None;
    }

    let raw_name = stat_parts[1].trim_matches('(').trim_matches(')').to_owned();
    let state_char = stat_parts[2];
    let state = match state_char {
        "R" => "Running",
        "S" => "Sleeping",
        "D" => "Disk Sleep",
        "Z" => "Zombie",
        "T" => "Stopped",
        _ => "Idle",
    }
    .to_owned();

    let ppid = stat_parts[3].parse::<u32>().unwrap_or(0);
    let threads = stat_parts[19].parse::<u32>().unwrap_or(1);
    let rss_pages = stat_parts[23].parse::<u64>().unwrap_or(0);
    let memory_bytes = rss_pages * 4096;

    let cmdline = fs::read_to_string(format!("{proc_path}/cmdline")).map_or_else(
        |_| raw_name.clone(),
        |s| s.replace('\0', " ").trim().to_owned(),
    );

    let display_cmd = if cmdline.is_empty() {
        raw_name.clone()
    } else {
        cmdline
    };

    let mut user = "root".to_owned();
    if let Ok(status) = fs::read_to_string(format!("{proc_path}/status")) {
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("Uid:") {
                if let Some(uid_str) = rest.split_whitespace().next()
                    && let Ok(uid) = uid_str.parse::<u32>()
                {
                    user = users_map
                        .get(&uid)
                        .cloned()
                        .unwrap_or_else(|| uid.to_string());
                }
                break;
            }
        }
    }

    Some(ProcessRecord {
        pid,
        ppid,
        name: raw_name,
        cmdline: display_cmd,
        user,
        cpu_percent: 0.0,
        memory_bytes,
        memory_percent: 0.0,
        state,
        threads,
    })
}

#[cfg(target_os = "linux")]
fn query_unit_status(name: &str) -> (ServiceState, String, Option<u32>, Option<u64>) {
    let binary_name = name.trim_end_matches(".service").replace('@', "-");
    let base_name = binary_name.split('-').take(2).collect::<Vec<_>>().join("-");

    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            if let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>()
                && let Ok(comm) = fs::read_to_string(format!("/proc/{pid}/comm"))
            {
                let comm_clean = comm.trim();
                if comm_clean == binary_name
                    || (!base_name.is_empty() && comm_clean.contains(&base_name))
                {
                    let memory_bytes = fs::read_to_string(format!("/proc/{pid}/statm"))
                        .ok()
                        .and_then(|s| s.split_whitespace().nth(1)?.parse::<u64>().ok())
                        .map(|rss| rss * 4096);
                    return (
                        ServiceState::Active,
                        "running".to_owned(),
                        Some(pid),
                        memory_bytes,
                    );
                }
            }
        }
    }

    (ServiceState::Inactive, "dead".to_owned(), None, None)
}

#[cfg(test)]
mod journal_tests {
    use super::{read_journal, severity_priority};
    use cybou_web_contracts::SystemLogsQueryRequest;

    fn query(limit: Option<usize>) -> SystemLogsQueryRequest {
        SystemLogsQueryRequest {
            unit: None,
            severity: None,
            search: None,
            limit,
        }
    }

    #[test]
    fn severity_names_are_a_closed_set() {
        assert_eq!(severity_priority("emerg"), Some(0));
        assert_eq!(severity_priority("err"), Some(3));
        assert_eq!(severity_priority("warning"), Some(4));
        assert_eq!(severity_priority("debug"), Some(7));

        // Not a syslog level, a level in the wrong case, and a level with whitespace on it are all
        // caller errors. Answering `None` is what lets the hub refuse rather than quietly drop the
        // filter and return everything under the label "Errors".
        assert_eq!(severity_priority("critical"), None);
        assert_eq!(severity_priority("ERR"), None);
        assert_eq!(severity_priority(" err"), None);
        assert_eq!(severity_priority(""), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_read_that_worked_names_no_reason_and_a_read_that_did_not_always_does() {
        let projection = read_journal(&query(Some(3)), None);

        // The invariant this whole field exists for: an empty feed is never silent about why.
        if projection.unavailable.is_none() {
            assert!(
                projection.logs.len() <= 3,
                "a limit of 3 returned {} entries",
                projection.logs.len()
            );
            // Without this the assertion above passes on an empty feed, which is the exact failure
            // it was written to catch. A host whose whole journal is readable has written to it.
            if projection.system_journal_readable {
                assert!(
                    !projection.logs.is_empty(),
                    "the system journal is readable and the reader returned nothing"
                );
            }
        } else {
            assert!(
                projection.logs.is_empty(),
                "a projection that says it could not read carried {} entries",
                projection.logs.len()
            );
            assert!(!projection.system_journal_readable);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn every_entry_carries_a_severity_this_build_knows() {
        let projection = read_journal(&query(Some(50)), None);

        for entry in &projection.logs {
            assert!(
                severity_priority(&entry.severity).is_some(),
                "entry carried severity {:?}, which is not a syslog level",
                entry.severity
            );
            assert!(!entry.timestamp.is_empty());
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_search_keeps_only_what_matches_it() {
        let mut request = query(Some(20));
        // A string no journal line contains, so the only honest answer is nothing at all. If the
        // filter were handed to journalctl and dropped, this would come back full.
        request.search = Some("zzz-no-line-contains-this-zzz".to_owned());
        let projection = read_journal(&request, None);

        assert!(
            projection.logs.is_empty(),
            "a search matching nothing returned {} entries",
            projection.logs.len()
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_unit_beginning_with_a_dash_is_a_value_rather_than_an_option() {
        let mut request = query(Some(5));
        // `--unit=-x` is one argv token. If it were split into two, journalctl would read `-x` as
        // an option and either fail or mean something nobody asked for.
        request.unit = Some("-x".to_owned());
        let projection = read_journal(&request, None);

        // Either journalctl rejected the unit name, or it matched nothing. What must not happen is
        // a full feed, which is what a mis-parsed argument would produce.
        assert!(projection.logs.is_empty());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_field_that_is_not_utf8_arrives_as_bytes_and_is_still_read() {
        let record = serde_json::json!({
            "MESSAGE": [104, 105, 255],
            "PRIORITY": "3",
            "_PID": "42",
            "_SYSTEMD_UNIT": "cybou-web-gateway.service",
            "__REALTIME_TIMESTAMP": "1788047701250979",
        });

        let entry = super::journal_entry(&record).expect("a record with a message is an entry");
        assert_eq!(entry.severity, "err");
        assert_eq!(entry.pid, Some(42));
        assert_eq!(entry.unit.as_deref(), Some("cybou-web-gateway.service"));
        assert!(entry.message.starts_with("hi"));
        assert!(entry.timestamp.starts_with("2026-"), "{}", entry.timestamp);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_record_with_no_message_is_not_an_entry() {
        let record = serde_json::json!({ "PRIORITY": "6" });
        assert!(super::journal_entry(&record).is_none());
    }
}
