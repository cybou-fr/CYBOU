// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Operation1 daemon entrypoint.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        let endpoint = cybou_fabric::OPERATION;
        let service = cybou_operationd::service::Operation1Service::durable_default()?;
        let _connection = zbus::connection::Builder::session()?
            .name(endpoint.service)?
            .serve_at(endpoint.object_path, service)?
            .build()
            .await?;
        tokio::signal::ctrl_c().await?;
    }
    #[cfg(not(target_os = "linux"))]
    tokio::signal::ctrl_c().await?;
    Ok(())
}
