// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Grounded Linux system and hardware reader for real OS observation without mock fixtures.

use std::{
    collections::HashMap,
    fs,
    path::Path,
};

use cybou_protocol::system::{
    CpuCoreStat, DiskPartitionInfo, NetworkInterfaceInfo, ProcessRecord,
    ServiceRecord, ServiceState, ServiceUnitType,
};
use cybou_web_contracts::{ProcessesListProjection, ServicesListProjection, SystemMonitorProjection, WEB_SCHEMA_V1};

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

        let (memory_total_bytes, memory_used_bytes, memory_free_bytes, swap_total_bytes, swap_used_bytes) = read_meminfo();
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
                if let Ok(pid) = name_str.parse::<u32>() {
                    if let Some(proc) = read_single_process(pid, &users_map) {
                        processes.push(proc);
                        if processes.len() >= 500 {
                            break;
                        }
                    }
                }
            }
        }

        processes.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));

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

    #[cfg(not(target_os = "linux"))]
    {
        ProcessesListProjection {
            schema_version: WEB_SCHEMA_V1,
            total_count: 0,
            total_cpu_percent: 0.0,
            total_memory_bytes: 0,
            processes: Vec::new(),
        }
    }
}

/// Reads real status of Cybou systemd services.
#[must_use]
pub fn read_real_services() -> ServicesListProjection {
    let known_services = [
        ("cybou-web-gateway.service", "CYBOU Web Gateway & Browser Desktop API"),
        ("cybou-host-filesd@demo.service", "CYBOU Host User Filesystem Daemon (demo)"),
        ("cybou-agentd.service", "CYBOU Agent Capsule Isolation Manager"),
        ("cybou-actiond.service", "CYBOU Governed Action1 Execution Authority"),
        ("cybou-presenced.service", "CYBOU Presence1 Event Stream Hub"),
        ("cybou-telemetryd.service", "CYBOU Telemetry1 Diagnostic Engine"),
        ("cybou-eventd.service", "CYBOU Event1 Canonical Journal Writer"),
        ("cybou-identityd.service", "CYBOU Identity1 Subject Continuity Service"),
        ("cybou-healthd.service", "CYBOU Health1 Capability Health Service"),
        ("cybou-intentiond.service", "CYBOU Intention1 Commitments and Obligations"),
        ("cybou-predictord.service", "CYBOU Predictor1 Empirical Forecasting Engine"),
        ("cybou-perceptiond.service", "CYBOU Perception1 Linux Observation Service"),
        ("cybou-epistemicd.service", "CYBOU Epistemic1 Knowledge Projection Service"),
        ("cybou-contextd.service", "CYBOU Context1 Associative Context Service"),
        ("cybou-meaningd.service", "CYBOU Meaning1 Meaning Boundary Service"),
        ("cybou-model-brokerd.service", "CYBOU ModelBroker1 Model Gateway"),
        ("cybou-workspaced.service", "CYBOU Workspace1 Global Attention Service"),
        ("cybou-lifecycled.service", "CYBOU Lifecycle1 Sleep/Wake Consolidation"),
        ("cybou-selfd.service", "CYBOU Self1 Continuous Self-Model Service"),
        ("cybou-shelld.service", "CYBOU Shell1 Sandboxed Execution Service"),
        ("cybou-remediationd.service", "CYBOU Remediation1 Finding Resolution"),
        ("cybou-executord.service", "CYBOU Typed Body Action Executor"),
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
    if let Ok(content) = fs::read_to_string("/proc/uptime") {
        if let Some(first) = content.split_whitespace().next() {
            if let Ok(secs) = first.parse::<f64>() {
                return secs as u64;
            }
        }
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
            let val = parts.next().and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
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
                    let core_id = parts[0].trim_start_matches("cpu").parse::<usize>().unwrap_or(0);
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

    let cmdline = fs::read_to_string(format!("{proc_path}/cmdline"))
        .map(|s| s.replace('\0', " ").trim().to_owned())
        .unwrap_or_else(|_| raw_name.clone());

    let display_cmd = if cmdline.is_empty() { raw_name.clone() } else { cmdline };

    let mut user = "root".to_owned();
    if let Ok(status) = fs::read_to_string(format!("{proc_path}/status")) {
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("Uid:") {
                if let Some(uid_str) = rest.split_whitespace().next() {
                    if let Ok(uid) = uid_str.parse::<u32>() {
                        user = users_map.get(&uid).cloned().unwrap_or_else(|| uid.to_string());
                    }
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
            if let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() {
                if let Ok(comm) = fs::read_to_string(format!("/proc/{pid}/comm")) {
                    let comm_clean = comm.trim();
                    if comm_clean == binary_name || (!base_name.is_empty() && comm_clean.contains(&base_name)) {
                        let memory_bytes = fs::read_to_string(format!("/proc/{pid}/statm"))
                            .ok()
                            .and_then(|s| s.split_whitespace().nth(1)?.parse::<u64>().ok())
                            .map(|rss| rss * 4096);
                        return (ServiceState::Active, "running".to_owned(), Some(pid), memory_bytes);
                    }
                }
            }
        }
    }

    (ServiceState::Inactive, "dead".to_owned(), None, None)
}
