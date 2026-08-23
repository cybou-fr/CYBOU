// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-telemetryd` daemon entrypoint.

use std::sync::Arc;

use cybou_telemetryd::TelemetryCore;
use time::Duration;

/// How far back the windows see, and how many readings each holds.
///
/// Six hours at one sample every ten seconds is 2160, so the count bound is the one that binds and
/// the span is what a person would call *recent*. Both are needed: the count alone would let a
/// slow sampler remember a week, and the span alone would let a burst hold everything it produced.
const WINDOW_SPAN: Duration = Duration::hours(6);
const WINDOW_CAPACITY: usize = 2160;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-telemetryd] Watching the Body (telemetry is not biography)...");

    let core = Arc::new(TelemetryCore::new(WINDOW_SPAN, WINDOW_CAPACITY));

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::TELEMETRY;
        use cybou_telemetryd::service::Telemetry1Service;

        let sampling = core.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                for reading in linux::sample(time::OffsetDateTime::now_utc()) {
                    sampling.observe(reading);
                }
            }
        });

        let connection = zbus::connection::Builder::session()?
            .name(TELEMETRY.service)?
            .serve_at(TELEMETRY.object_path, Telemetry1Service::new(core.clone()))?
            .build()
            .await?;
        println!(
            "[cybou-telemetryd] Serving {} at {}",
            TELEMETRY.interface, TELEMETRY.object_path
        );
        std::future::pending::<()>().await;
        drop(connection);
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-telemetryd] /proc and D-Bus are Linux-only; nothing to watch here.");
    }

    Ok(())
}

/// Reading the files this host publishes about itself.
///
/// The only code in this organ that touches the filesystem, and it does nothing but read text and
/// hand it to a parser. Everything that decides what the text means lives in `probe`, where it is
/// tested without a kernel.
#[cfg(target_os = "linux")]
mod linux {
    use cybou_protocol::telemetry::{Reading, Subject};
    use cybou_telemetryd::probe;
    use time::OffsetDateTime;

    /// Take one reading of everything this host will say something about.
    ///
    /// A file that is missing or unreadable produces no reading rather than a zero. One fewer
    /// sample is a gap in a window; a fabricated zero moves what the host believes is ordinary
    /// about itself, which is worse and is invisible afterwards.
    pub fn sample(now: OffsetDateTime) -> Vec<Reading> {
        let mut readings = Vec::new();
        let mut note = |subject: Subject, value: Option<f64>| {
            if let Some(value) = value {
                readings.push(Reading {
                    subject,
                    value,
                    at: now,
                });
            }
        };

        let read = |path: &str| std::fs::read_to_string(path).ok();

        if let Some(loadavg) = read("/proc/loadavg") {
            note(Subject::LoadAverage, probe::load_average(&loadavg));
        }
        if let Some(meminfo) = read("/proc/meminfo") {
            note(Subject::MemoryUsed, probe::memory_used(&meminfo));
            note(Subject::SwapUsed, probe::swap_used(&meminfo));
        }
        for (subject, path) in [
            (Subject::MemoryPressure, "/proc/pressure/memory"),
            (Subject::IoPressure, "/proc/pressure/io"),
            (Subject::CpuPressure, "/proc/pressure/cpu"),
        ] {
            if let Some(contents) = read(path) {
                note(subject, probe::pressure_some_avg10(&contents));
            }
        }
        note(Subject::RootFilesystemUsed, root_filesystem_used());
        note(Subject::RootFilesystemInodesUsed, root_filesystem_inodes());
        if let Some(file_nr) = read("/proc/sys/fs/file-nr") {
            note(Subject::OpenFileDescriptors, probe::open_files(&file_nr));
        }
        note(Subject::FailedUnits, failed_units());
        readings
    }

    /// How full the root filesystem is, from `statvfs`.
    ///
    /// Shelling out to `df` rather than linking a libc binding, because this organ is forbidden
    /// unsafe code like the rest of the workspace and the shape of the answer is what matters here,
    /// not the microseconds. A `df` that is absent or unparseable produces no reading.
    fn root_filesystem_used() -> Option<f64> {
        let output = std::process::Command::new("df")
            .args(["--output=size,avail", "-B1", "/"])
            .output()
            .ok()?;
        let text = String::from_utf8(output.stdout).ok()?;
        let figures = text.lines().nth(1)?;
        let mut fields = figures.split_whitespace();
        let total: u64 = fields.next()?.parse().ok()?;
        let available: u64 = fields.next()?.parse().ok()?;
        probe::filesystem_used(total, available)
    }

    /// How many of the root filesystem's inodes are in use.
    ///
    /// Its own call rather than a second column on the byte query, because a filesystem can report
    /// bytes and no inodes, and one `df` invocation that failed for either reason would take both
    /// readings with it.
    fn root_filesystem_inodes() -> Option<f64> {
        let output = std::process::Command::new("df")
            .args(["--output=itotal,iavail", "/"])
            .output()
            .ok()?;
        let text = String::from_utf8(output.stdout).ok()?;
        let figures = text.lines().nth(1)?;
        let mut fields = figures.split_whitespace();
        let total: u64 = fields.next()?.parse().ok()?;
        let available: u64 = fields.next()?.parse().ok()?;
        probe::inodes_used(total, available)
    }

    /// How many units are in a failed state.
    ///
    /// `None` when `systemctl` cannot be asked at all, which is different from an answer of zero: a
    /// host where nothing can be enumerated has not established that nothing has failed.
    fn failed_units() -> Option<f64> {
        let output = std::process::Command::new("systemctl")
            .args(["list-units", "--failed", "--no-legend", "--plain"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let listing = String::from_utf8(output.stdout).ok()?;
        Some(probe::failed_units(&listing))
    }
}
