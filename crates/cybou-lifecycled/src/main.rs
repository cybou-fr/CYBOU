// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-lifecycled` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

use cybou_lifecycled::LifecycleCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-lifecycled] Initializing cognitive lifecycle engine...");
    let state_path = env::var("CYBOU_LIFECYCLE_PATH").map_or_else(
        |_| {
            let state_dir = env::var("XDG_STATE_HOME").map_or_else(
                |_| {
                    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
                    PathBuf::from(home).join(".local/state")
                },
                PathBuf::from,
            );
            state_dir.join("cybou/lifecycle.json")
        },
        PathBuf::from,
    );
    let core = Arc::new(LifecycleCore::open(&state_path)?);

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::LIFECYCLE;
        use cybou_lifecycled::service::Lifecycle1Service;

        println!("[cybou-lifecycled] Connecting to D-Bus session bus...");
        let service = Lifecycle1Service::new(core);
        let connection = zbus::connection::Builder::session()?
            .name(LIFECYCLE.service)?
            .serve_at(LIFECYCLE.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-lifecycled] Registered '{}' at '{}'",
            LIFECYCLE.service, LIFECYCLE.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-lifecycled] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-lifecycled] Running on non-Linux host in headless mode.");
        let _ = core;
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
