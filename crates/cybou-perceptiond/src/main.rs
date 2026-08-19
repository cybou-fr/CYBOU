// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-perceptiond` daemon entrypoint.

use std::{sync::Arc, time::Duration};

use cybou_perception::LinuxSystemSource;
use cybou_perceptiond::PerceptionCore;
use time::OffsetDateTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-perceptiond] Initializing Linux system perception engine...");
    let source = LinuxSystemSource::new_standard(300);
    let core = Arc::new(PerceptionCore::new(source));

    let now = OffsetDateTime::now_utc();
    let initial_env = core.acquire_once(now, 0);
    println!("[cybou-perceptiond] Initial perception health: {}", core.health());

    // Spawn periodic background acquisition task
    let sampling_core = core.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        let mut monotonic = 1u64;
        loop {
            interval.tick().await;
            let now = OffsetDateTime::now_utc();
            if let Some(_envelope) = sampling_core.acquire_once(now, monotonic) {
                monotonic += 1;
                // On real bus, newly acquired envelopes are submitted to Event1
            }
        }
    });

    let _ = initial_env;

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
