// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Operation1 daemon entrypoint.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        let endpoint = cybou_fabric::OPERATION;
        let service = cybou_operationd::service::Operation1Service::durable_default()?;
        let reconciler = service.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
            loop {
                interval.tick().await;
                if let Err(error) = reconciler.reconcile_agents().await {
                    eprintln!("[cybou-operationd] Agent1 reconciliation failed: {error}");
                }
            }
        });
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
