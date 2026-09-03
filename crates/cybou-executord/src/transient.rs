// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Run one bounded command in a systemd transient unit and wait for what it did.
//!
//! The executor is a long-lived root process, and the sandbox it runs under is the reason that is
//! tolerable. A package manager needs to write `/usr` and reach the network, and relaxing the
//! executor's own unit to allow that would hand every adapter in this process what one of them
//! needs. So the command runs somewhere else: systemd starts it as its own unit, with its own
//! sandbox, and this process only asks for it and reads the result.
//!
//! Waiting is the part worth being careful about. `StartTransientUnit` returns as soon as the job
//! is enqueued, so a caller that returned there would report success for something that had not
//! started. This subscribes to `JobRemoved` before starting anything, waits for the job it was
//! given, and then reads the unit's own verdict — an exit status, not a job result, because a job
//! that ran a command which failed is a job that completed.

use std::time::Duration;

use futures_util::StreamExt as _;
use zbus::{Proxy, zvariant::Value};

use crate::ExecutorError;

/// Longest a transient command may take before this stops waiting for it.
///
/// Not a kill: the unit keeps running and systemd keeps its result. What ends is this process's
/// claim to know how it went, which is reported as exactly that.
const MAX_WAIT: Duration = Duration::from_mins(30);

/// Where the transient unit is started.
///
/// The system manager on a deployed host. The user manager exists so the mechanism can be proven on
/// a machine where nothing may touch the system one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Manager {
    /// `org.freedesktop.systemd1` on the system bus.
    System,
    /// `org.freedesktop.systemd1` on the caller's own session bus.
    User,
}

impl Manager {
    /// Connect to this manager's bus.
    ///
    /// # Errors
    ///
    /// Returns an adapter error when the bus cannot be reached.
    pub async fn connect(self) -> Result<zbus::Connection, ExecutorError> {
        let connection = match self {
            Self::System => zbus::Connection::system().await,
            Self::User => zbus::Connection::session().await,
        };
        connection.map_err(|error| ExecutorError::Adapter(error.to_string()))
    }
}

async fn manager_proxy(connection: &zbus::Connection) -> Result<Proxy<'_>, ExecutorError> {
    Proxy::new(
        connection,
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    )
    .await
    .map_err(|error| ExecutorError::Adapter(error.to_string()))
}

/// Start one command as a transient unit and wait for it to finish.
///
/// `argv[0]` is the executable, given to systemd as an absolute path, and the whole of `argv` is
/// the command line. Nothing here builds a shell command: there is no shell, so there is nothing
/// for a quoted argument to mean.
///
/// # Errors
///
/// Returns an adapter error when the unit cannot be started, when the command exits non-zero, or
/// when it is still running after the bounded wait. Each of those is a different sentence, because
/// they are different things to have happened.
pub async fn run(
    connection: &zbus::Connection,
    unit: &str,
    argv: &[String],
    environment: &[String],
) -> Result<(), ExecutorError> {
    let Some(executable) = argv.first() else {
        return Err(ExecutorError::Adapter(
            "a transient unit needs a command to run".to_owned(),
        ));
    };
    let manager = manager_proxy(connection).await?;

    // Subscribed before the unit is started, so a command that finishes immediately cannot finish
    // between the start and the wait.
    let mut jobs = manager
        .receive_signal("JobRemoved")
        .await
        .map_err(|error| ExecutorError::Adapter(error.to_string()))?;

    let execution = vec![(executable.clone(), argv.to_vec(), false)];
    let properties: Vec<(&str, Value<'_>)> = vec![
        ("Description", Value::from(format!("Cybou {unit}"))),
        ("Type", Value::from("oneshot")),
        // Kept after it exits so its exit status can still be read, and not collected on failure
        // either — a unit systemd has already forgotten cannot say what it did. Reset below.
        ("RemainAfterExit", Value::from(true)),
        ("Environment", Value::from(environment.to_vec())),
        ("ExecStart", Value::from(execution)),
        // The transient unit's own confinement. It has to write what a package manager writes, so
        // it is not `ProtectSystem=full`; everything it does not need is still taken away.
        ("PrivateTmp", Value::from(true)),
        // A string on the bus, not a boolean: "yes", "read-only" or "tmpfs" are different things.
        ("ProtectHome", Value::from("yes")),
        ("ProtectKernelTunables", Value::from(true)),
        ("ProtectKernelModules", Value::from(true)),
        ("ProtectControlGroups", Value::from(true)),
        ("NoNewPrivileges", Value::from(true)),
        ("RestrictSUIDSGID", Value::from(true)),
        ("LockPersonality", Value::from(true)),
    ];
    let aux: Vec<(String, Vec<(String, Value<'_>)>)> = Vec::new();

    let job: zbus::zvariant::OwnedObjectPath = manager
        .call("StartTransientUnit", &(unit, "fail", properties, aux))
        .await
        .map_err(|error| ExecutorError::Adapter(error.to_string()))?;

    let started = tokio::time::timeout(MAX_WAIT, async {
        while let Some(signal) = jobs.next().await {
            let Ok((_id, path, _unit, result)) =
                signal
                    .body()
                    .deserialize::<(u32, zbus::zvariant::OwnedObjectPath, String, String)>()
            else {
                continue;
            };
            if path == job {
                return result;
            }
        }
        "disconnected".to_owned()
    })
    .await;

    let Ok(outcome) = started else {
        return Err(ExecutorError::Adapter(format!(
            "{unit} was still running after {} minutes; systemd holds what happens next",
            MAX_WAIT.as_secs() / 60
        )));
    };

    // Read before the unit is forgotten. A command that ran and exited non-zero makes the job
    // "failed" too, so the job result alone cannot tell "never started" from "started and said no".
    let status = exit_status(connection, unit).await;
    let result = unit_result(connection, unit).await;
    release(&manager, unit).await;

    match (outcome.as_str(), status, result.as_deref()) {
        ("done", Some(0), Some("success") | None) => Ok(()),
        // 203 is systemd's own status for "the command could not be executed at all", which is a
        // different thing to report than a program that ran and disagreed.
        (_, Some(203), _) => Err(ExecutorError::Adapter(format!(
            "{unit} could not execute its command"
        ))),
        (_, Some(code), _) if code != 0 => Err(ExecutorError::Adapter(format!(
            "{unit} ran and exited {code}"
        ))),
        (_, _, Some(reason)) if reason != "success" => Err(ExecutorError::Adapter(format!(
            "{unit} did not succeed: systemd reported {reason}"
        ))),
        ("done", None, _) => Err(ExecutorError::Adapter(format!(
            "{unit} finished and its exit status could not be read"
        ))),
        (job, _, _) => Err(ExecutorError::Adapter(format!(
            "{unit} did not run: systemd reported {job}"
        ))),
    }
}

/// Why the unit ended, in systemd's own vocabulary: `success`, `exit-code`, `signal`, `timeout`,
/// `oom-kill` and the rest. A different question from what the command returned.
async fn unit_result(connection: &zbus::Connection, unit: &str) -> Option<String> {
    let manager = manager_proxy(connection).await.ok()?;
    let path: zbus::zvariant::OwnedObjectPath = manager.call("GetUnit", &(unit,)).await.ok()?;
    let service = Proxy::new(
        connection,
        "org.freedesktop.systemd1",
        path,
        "org.freedesktop.systemd1.Service",
    )
    .await
    .ok()?;
    service.get_property("Result").await.ok()
}

/// What the command itself exited with, as distinct from what the job did.
async fn exit_status(connection: &zbus::Connection, unit: &str) -> Option<i32> {
    let manager = manager_proxy(connection).await.ok()?;
    let path: zbus::zvariant::OwnedObjectPath = manager.call("GetUnit", &(unit,)).await.ok()?;
    let service = Proxy::new(
        connection,
        "org.freedesktop.systemd1",
        path,
        "org.freedesktop.systemd1.Service",
    )
    .await
    .ok()?;
    service.get_property("ExecMainStatus").await.ok()
}

/// Let go of a transient unit once its result has been read.
///
/// `RemainAfterExit` is what keeps a finished unit readable, and it is also what would keep it
/// loaded and active forever. Stopping it deactivates a unit that has already run and lets systemd
/// collect it; resetting clears one that failed. Neither is a second attempt at anything: the
/// command has already finished by the time this runs.
async fn release(manager: &Proxy<'_>, unit: &str) {
    let _: Result<zbus::zvariant::OwnedObjectPath, _> =
        manager.call("StopUnit", &(unit, "replace")).await;
    let _: Result<(), _> = manager.call("ResetFailedUnit", &(unit,)).await;
}

/// One unit name that cannot collide with another run of the same operation.
#[must_use]
pub fn unit_name(operation: &str) -> String {
    format!("cybou-{operation}-{}.service", uuid::Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_transient_unit_name_is_one_systemd_will_accept() {
        // The names this module is given are built from a fixed prefix and a uuid, which is what
        // keeps two installs of the same package from colliding on one host.
        let name = super::unit_name("install");
        assert!(name.starts_with("cybou-install-"));
        assert!(name.ends_with(".service"));
        assert!(
            name.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_'))
        );
    }
}
