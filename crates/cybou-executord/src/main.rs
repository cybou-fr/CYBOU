// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-executord` daemon entrypoint.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        use std::sync::Arc;

        use cybou_executord::{
            linux::{Action1PermitSource, LinuxBody},
            service::Executor1Service,
        };
        use cybou_fabric::EXECUTOR;

        let permits = Arc::new(Action1PermitSource::session().await?);
        let body = Arc::new(LinuxBody::system().await?);
        let builder = if std::env::var_os("CYBOU_EXECUTOR_SYSTEM_BUS").is_some() {
            zbus::connection::Builder::system()?
        } else {
            zbus::connection::Builder::session()?
        };
        let _connection = builder
            .name(EXECUTOR.service)?
            .serve_at(EXECUTOR.object_path, Executor1Service::new(permits, body))?
            .build()
            .await?;
        println!("[cybou-executord] Registered {}", EXECUTOR.service);
        tokio::signal::ctrl_c().await?;
    }

    #[cfg(not(target_os = "linux"))]
    tokio::signal::ctrl_c().await?;

    Ok(())
}
