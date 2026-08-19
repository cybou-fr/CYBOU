// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-epistemicd` daemon entrypoint.

use std::sync::Arc;

use cybou_epistemicd::EpistemicCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-epistemicd] Initializing Epistemic projection engine (observation != knowledge)...");
    let core = Arc::new(EpistemicCore::new());

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::EPISTEMIC;
        use cybou_epistemicd::service::Epistemic1Service;

        println!("[cybou-epistemicd] Connecting to D-Bus session bus...");
        let service = Epistemic1Service::new(core);
        let connection = zbus::connection::Builder::session()?
            .name(EPISTEMIC.service)?
            .serve_at(EPISTEMIC.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-epistemicd] Registered '{}' at '{}'",
            EPISTEMIC.service, EPISTEMIC.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-epistemicd] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-epistemicd] Running on non-Linux host in headless mode.");
        let _ = core;
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
