// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-contextd` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

use cybou_contextd::ContextCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-contextd] Initializing Associative Context engine (association != truth)...");
    let state_path = env::var("CYBOU_CONTEXT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let state_dir = env::var("XDG_STATE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
                    PathBuf::from(home).join(".local/state")
                });
            state_dir.join("cybou/context.json")
        });

    let core = Arc::new(ContextCore::open(&state_path)?);

    #[cfg(target_os = "linux")]
    {
        use cybou_contextd::service::Context1Service;
        use cybou_fabric::CONTEXT;

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
