// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-healthd` daemon entrypoint.

use std::{sync::Arc, time::Duration};

use cybou_healthd::HealthCore;
use time::OffsetDateTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-healthd] Initializing capability health engine...");
    let core = Arc::new(HealthCore::new());

    core.recalculate(OffsetDateTime::now_utc());
    println!(
        "[cybou-healthd] Initial overall health: {}",
        core.overall_health()
    );

    // Spawn periodic health refresh task
    let refresh_core = core.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            let now = OffsetDateTime::now_utc();
            refresh_core.recalculate(now);
        }
    });

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::HEALTH;
        use cybou_healthd::service::Health1Service;

        println!("[cybou-healthd] Connecting to D-Bus session bus...");
        let service = Health1Service::new(core);
        let connection = zbus::connection::Builder::session()?
            .name(HEALTH.service)?
            .serve_at(HEALTH.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-healthd] Registered '{}' at '{}'",
            HEALTH.service, HEALTH.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-healthd] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-healthd] Running on non-Linux host in headless mode.");
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
