// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-presenced` daemon entrypoint.

use std::sync::Arc;

use cybou_presenced::PresenceCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-presenced] Initializing Presence presentation engine...");
    let core = Arc::new(PresenceCore::new());

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::PRESENCE;
        use cybou_presenced::service::Presence1Service;

        println!("[cybou-presenced] Connecting to D-Bus session bus...");
        let service = Presence1Service::new(core);
        let connection = zbus::connection::Builder::session()?
            .name(PRESENCE.service)?
            .serve_at(PRESENCE.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-presenced] Registered '{}' at '{}'",
            PRESENCE.service, PRESENCE.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-presenced] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-presenced] Running on non-Linux host in headless mode.");
        let _ = core;
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
