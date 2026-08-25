// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Linux implementations of the fixed Body adapters and Action1 permit claim.

use std::process::{Command, Stdio};

use async_trait::async_trait;
use cybou_fabric::{ACTION, decode};
use cybou_protocol::action::{BodyReading, ExecutionPermit};
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
    async fn claim(&self, permit_id: Uuid) -> Result<ExecutionPermit, ExecutorError> {
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
        let manager = self.manager().await?;
        let _: zbus::zvariant::OwnedObjectPath = manager
            .call("RestartUnit", &(unit, "replace"))
            .await
            .map_err(|error| ExecutorError::Adapter(error.to_string()))?;
        Ok(Vec::new())
    }
}
