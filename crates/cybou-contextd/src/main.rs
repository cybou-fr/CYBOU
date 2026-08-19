// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-contextd` daemon entrypoint.

use std::sync::Arc;

use cybou_contextd::ContextCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-contextd] Initializing Associative Context engine (association != truth)...");
    let core = Arc::new(ContextCore::new());

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::CONTEXT;
        use cybou_contextd::service::Context1Service;

        println!("[cybou-contextd] Connecting to D-Bus session bus...");
        let service = Context1Service::new(core);
        let connection = zbus::connection::Builder::session()?
            .name(CONTEXT.service)?
            .serve_at(CONTEXT.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-contextd] Registered '{}' at '{}'",
            CONTEXT.service, CONTEXT.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-contextd] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-contextd] Running on non-Linux host in headless mode.");
        let _ = core;
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
