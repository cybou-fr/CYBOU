// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-perceptiond` daemon entrypoint.

use std::sync::Arc;

use cybou_perception::LinuxSystemSource;
use cybou_perceptiond::PerceptionCore;
use time::OffsetDateTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-perceptiond] Initializing Linux system perception engine...");
    let source = LinuxSystemSource::new_standard(300);
    let core = Arc::new(PerceptionCore::new(source));

    let now = OffsetDateTime::now_utc();
    let _ = core.acquire_once(now, 0);
    println!("[cybou-perceptiond] Initial perception health: {}", core.health());

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::PERCEPTION;
        use cybou_perceptiond::service::Perception1Service;

        println!("[cybou-perceptiond] Connecting to D-Bus session bus...");
        let service = Perception1Service::new(core);
        let connection = zbus::connection::Builder::session()?
            .name(PERCEPTION.service)?
            .serve_at(PERCEPTION.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-perceptiond] Registered '{}' at '{}'",
            PERCEPTION.service, PERCEPTION.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-perceptiond] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-perceptiond] Running on non-Linux host in headless mode.");
        let _ = core;
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
