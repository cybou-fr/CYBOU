// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-lifecycled` daemon entrypoint.

use std::sync::Arc;

use cybou_lifecycled::LifecycleCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-lifecycled] Initializing cognitive lifecycle engine...");
    let core = Arc::new(LifecycleCore::new());

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
