// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Linux implementations of the fixed Body adapters and Action1 permit claim.

use std::process::{Command, Stdio};

use rustix::process::Signal;

use async_trait::async_trait;
use cybou_fabric::{ACTION, decode, encode};
use cybou_protocol::action::{BodyReading, ExecutionAttempt, ExecutionClaim};
use uuid::Uuid;
use zbus::Proxy;

use crate::{Body, ExecutorError, PermitSource};

/// Claims capabilities from the process that owns their lifecycle.
pub struct Action1PermitSource {
    connection: zbus::Connection,
}

impl Action1PermitSource {
    /// Connect to Action1 on the configured cognitive bus.
    ///
    /// # Errors
    ///
    /// Returns a typed refusal when the configured bus cannot be reached.
    pub async fn session() -> Result<Self, ExecutorError> {
        let connection = match std::env::var("CYBOU_ACTION_BUS_ADDRESS") {
            Ok(address) => {
                zbus::connection::Builder::address(address.as_str())
                    .map_err(|error| ExecutorError::PermitRefused(error.to_string()))?
                    .build()
                    .await
            }
            Err(_) if std::env::var_os("CYBOU_ACTION_SYSTEM_BUS").is_some() => {
                zbus::Connection::system().await
            }
            Err(_) => zbus::Connection::session().await,
        };
        Ok(Self {
            connection: connection
                .map_err(|error| ExecutorError::PermitRefused(error.to_string()))?,
        })
    }
}

#[async_trait]
impl PermitSource for Action1PermitSource {
    async fn claim(&self, permit_id: Uuid) -> Result<ExecutionClaim, ExecutorError> {
        let proxy = Proxy::new(
            &self.connection,
            ACTION.service,
            ACTION.object_path,
            ACTION.interface,
        )
        .await
        .map_err(|error| ExecutorError::PermitRefused(error.to_string()))?;
        let encoded: Vec<u8> = proxy
            .call("ClaimPermit", &(permit_id.to_string()))
            .await
            .map_err(|error| ExecutorError::PermitRefused(error.to_string()))?;
        decode(&encoded).map_err(|error| ExecutorError::PermitRefused(error.to_string()))
    }

    async fn record_attempt(&self, attempt: &ExecutionAttempt) -> Result<(), ExecutorError> {
        let proxy = Proxy::new(
            &self.connection,
            ACTION.service,
            ACTION.object_path,
            ACTION.interface,
        )
        .await
        .map_err(|error| ExecutorError::ReportNotRecorded(error.to_string()))?;
        let encoded =
            encode(attempt).map_err(|error| ExecutorError::ReportNotRecorded(error.to_string()))?;
        proxy
            .call::<_, _, ()>("RecordAttempt", &(encoded))
            .await
            .map_err(|error| ExecutorError::ReportNotRecorded(error.to_string()))
    }
}

/// Direct typed access to the host's systemd manager and fixed package-cache cleaner.
pub struct LinuxBody {
    system_bus: zbus::Connection,
}

impl LinuxBody {
    /// Connect to the host system manager.
    ///
    /// # Errors
    ///
    /// Returns an adapter error when the system bus is unavailable.
    pub async fn system() -> Result<Self, ExecutorError> {
        Ok(Self {
            system_bus: zbus::Connection::system()
                .await
                .map_err(|error| ExecutorError::Adapter(error.to_string()))?,
        })
    }

    async fn manager(&self) -> Result<Proxy<'_>, ExecutorError> {
        Proxy::new(
            &self.system_bus,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await
        .map_err(|error| ExecutorError::Adapter(error.to_string()))
    }
}

#[async_trait]
impl Body for LinuxBody {
    async fn service_status(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError> {
        let manager = self.manager().await?;
        let path: zbus::zvariant::OwnedObjectPath = manager
            .call("GetUnit", &(unit))
            .await
            .map_err(|error| ExecutorError::Adapter(error.to_string()))?;
        let unit = Proxy::new(
            &self.system_bus,
            "org.freedesktop.systemd1",
            path,
            "org.freedesktop.systemd1.Unit",
        )
        .await
        .map_err(|error| ExecutorError::Adapter(error.to_string()))?;
        let active: String = unit
            .get_property("ActiveState")
            .await
            .map_err(|error| ExecutorError::Adapter(error.to_string()))?;
        let sub: String = unit
            .get_property("SubState")
            .await
            .map_err(|error| ExecutorError::Adapter(error.to_string()))?;
        Ok(vec![
            BodyReading {
                field: "systemd.active-state".to_owned(),
                value: active,
            },
            BodyReading {
                field: "systemd.sub-state".to_owned(),
                value: sub,
            },
        ])
    }

    async fn clean_package_cache(&self) -> Result<Vec<BodyReading>, ExecutorError> {
        tokio::task::spawn_blocking(|| {
            Command::new("/usr/bin/apt-get")
                .arg("clean")
                .env_clear()
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .output()
        })
        .await
        .map_err(|error| ExecutorError::Adapter(error.to_string()))?
        .map_err(|error| ExecutorError::Adapter(error.to_string()))
        .and_then(|output| {
            output.status.success().then(Vec::new).ok_or_else(|| {
                ExecutorError::Adapter(String::from_utf8_lossy(&output.stderr).trim().to_owned())
            })
        })
    }

    async fn restart_service(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError> {
        self.unit_job("RestartUnit", unit).await
    }

    async fn start_service(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError> {
        self.unit_job("StartUnit", unit).await
    }

    async fn stop_service(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError> {
        self.unit_job("StopUnit", unit).await
    }

    async fn reload_service(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError> {
        self.unit_job("ReloadUnit", unit).await
    }

    async fn enable_service(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError> {
        let manager = self.manager().await?;
        // `EnableUnitFiles` answers with whether the unit carried install information and with the
        // symlinks it made. A unit with no `[Install]` section can be enabled all day and will
        // still not start at the next boot, so that first flag is the difference between having
        // done the thing and having appeared to.
        let (carries_install_info, _changes): (bool, Vec<(String, String, String)>) = manager
            .call("EnableUnitFiles", &(vec![unit], false, false))
            .await
            .map_err(|error| ExecutorError::Adapter(error.to_string()))?;
        if !carries_install_info {
            return Err(ExecutorError::Adapter(format!(
                "{unit} has no [Install] section, so enabling it would not start it at boot"
            )));
        }
        Self::reload_unit_files(&manager).await?;
        Ok(vec![BodyReading {
            field: "systemd.unit-file-state".to_owned(),
            value: "enabled".to_owned(),
        }])
    }

    async fn disable_service(&self, unit: &str) -> Result<Vec<BodyReading>, ExecutorError> {
        let manager = self.manager().await?;
        // Not the same call with a flag: disabling answers only with the changes it made, and
        // there is no install-information question to ask on the way out.
        let _changes: Vec<(String, String, String)> = manager
            .call("DisableUnitFiles", &(vec![unit], false))
            .await
            .map_err(|error| ExecutorError::Adapter(error.to_string()))?;
        Self::reload_unit_files(&manager).await?;
        Ok(vec![BodyReading {
            field: "systemd.unit-file-state".to_owned(),
            value: "disabled".to_owned(),
        }])
    }

    async fn terminate_process(
        &self,
        pid: u32,
        owner_uid: u32,
    ) -> Result<Vec<BodyReading>, ExecutorError> {
        Self::signal(pid, owner_uid, Signal::TERM)
    }

    async fn kill_process(
        &self,
        pid: u32,
        owner_uid: u32,
    ) -> Result<Vec<BodyReading>, ExecutorError> {
        Self::signal(pid, owner_uid, Signal::KILL)
    }

    async fn pause_process(
        &self,
        pid: u32,
        owner_uid: u32,
    ) -> Result<Vec<BodyReading>, ExecutorError> {
        Self::signal(pid, owner_uid, Signal::STOP)
    }

    async fn resume_process(
        &self,
        pid: u32,
        owner_uid: u32,
    ) -> Result<Vec<BodyReading>, ExecutorError> {
        Self::signal(pid, owner_uid, Signal::CONT)
    }
}

/// The real user id `/proc` reports for a process, or `None` if it has gone.
///
/// Free-standing so the check below can be tested against a directory of files rather than against
/// whatever this machine happens to be running.
fn owner_of(status: &str) -> Option<u32> {
    status
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|uid| uid.parse().ok())
}

impl LinuxBody {
    /// Send one signal to one process, having established for itself that it may.
    ///
    /// The executor runs as root, so it can signal anything on the machine. That is exactly why it
    /// does not take the permit's word for who owns the process: it reads `/proc` again and refuses
    /// if the answer differs from the one the proposal was decided against.
    ///
    /// This is not a second opinion for its own sake. A pid is a number the kernel reuses. Between
    /// the moment somebody selected a process of their own and the moment a permit is carried out,
    /// that process can exit and its number be handed to a new one — and if the new one belongs to
    /// root, an authorization to end your own editor becomes an authorization to end something
    /// else. The window is small and the check that closes it is a file read.
    ///
    /// pid 1 is refused here as well as in Action1. Init is not a process with a risk level.
    fn signal(pid: u32, owner_uid: u32, signal: Signal) -> Result<Vec<BodyReading>, ExecutorError> {
        if pid <= 1 {
            return Err(ExecutorError::Adapter(format!(
                "refusing to signal pid {pid}"
            )));
        }
        let status = std::fs::read_to_string(format!("/proc/{pid}/status"))
            .map_err(|_| ExecutorError::Adapter(format!("process {pid} is gone")))?;
        let actual = owner_of(&status)
            .ok_or_else(|| ExecutorError::Adapter(format!("process {pid} reports no owner")))?;
        if actual != owner_uid {
            return Err(ExecutorError::Adapter(format!(
                "process {pid} belongs to uid {actual}, not to the uid {owner_uid} this was decided for"
            )));
        }

        let target = rustix::process::Pid::from_raw(
            i32::try_from(pid)
                .map_err(|_| ExecutorError::Adapter(format!("pid {pid} is not a pid")))?,
        )
        .ok_or_else(|| ExecutorError::Adapter(format!("pid {pid} is not a pid")))?;
        rustix::process::kill_process(target, signal)
            .map_err(|error| ExecutorError::Adapter(error.to_string()))?;

        // What the executor saw, rather than what it intended. The attempt carries this, so a
        // reader of the Journal learns which process was signalled and who owned it at the moment
        // it happened — not at the moment somebody clicked.
        Ok(vec![
            BodyReading {
                field: "process.pid".to_owned(),
                value: pid.to_string(),
            },
            BodyReading {
                field: "process.owner-uid".to_owned(),
                value: actual.to_string(),
            },
        ])
    }

    /// Make systemd re-read the unit files after they have been changed on disk.
    ///
    /// `systemctl` does this for you and the bus API does not. Without it systemd keeps answering
    /// questions about enablement from what it loaded earlier, so the change is on disk and
    /// invisible until something else provokes a reload — which is indistinguishable, to anybody
    /// looking, from the change not having happened.
    async fn reload_unit_files(manager: &Proxy<'_>) -> Result<(), ExecutorError> {
        manager
            .call::<_, _, ()>("Reload", &())
            .await
            .map_err(|error| ExecutorError::Adapter(error.to_string()))
    }

    /// Ask systemd to do one named thing to one named unit.
    ///
    /// The method name is chosen here from a closed set of adapters and never travels from a
    /// caller, so this is four adapters sharing a call rather than one adapter taking an
    /// instruction. `replace` is systemd's own word for "supersede whatever job is queued for this
    /// unit", which is what somebody pressing a button a second time means.
    async fn unit_job(&self, job: &str, unit: &str) -> Result<Vec<BodyReading>, ExecutorError> {
        let manager = self.manager().await?;
        let _: zbus::zvariant::OwnedObjectPath = manager
            .call(job, &(unit, "replace"))
            .await
            .map_err(|error| ExecutorError::Adapter(error.to_string()))?;
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::owner_of;

    #[test]
    fn the_owner_is_the_real_uid_and_not_the_effective_one() {
        // `/proc/<pid>/status` reports four of them on one line: real, effective, saved and
        // filesystem. A process that dropped privileges has an effective uid that is not who
        // started it, and the question being asked here is whose process this is.
        let status = "Name:\tsleep\nState:\tS (sleeping)\nUid:\t1000\t0\t0\t1000\n";
        assert_eq!(owner_of(status), Some(1000));
    }

    #[test]
    fn a_status_without_an_owner_is_not_read_as_root() {
        // A missing line must not fall back to 0: that would be the check passing for the one uid
        // it most needs to refuse.
        assert_eq!(owner_of("Name:\tsleep\nState:\tS (sleeping)\n"), None);
        assert_eq!(owner_of("Uid:\n"), None);
        assert_eq!(owner_of("Uid:\tnobody\n"), None);
    }
}
