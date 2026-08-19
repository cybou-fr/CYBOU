// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-intentiond` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

use cybou_intentiond::IntentionCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-intentiond] Initializing Commitments and Obligations engine...");
    let state_path = env::var("CYBOU_INTENTIONS_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let state_dir = env::var("XDG_STATE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
                    PathBuf::from(home).join(".local/state")
                });
            state_dir.join("cybou/intentions.json")
        });
    let core = Arc::new(IntentionCore::open(&state_path)?);

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::INTENTION;
        use cybou_intentiond::service::Intention1Service;

        println!("[cybou-intentiond] Connecting to D-Bus session bus...");
        let service = Intention1Service::new(core);
        let connection = zbus::connection::Builder::session()?
            .name(INTENTION.service)?
            .serve_at(INTENTION.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-intentiond] Registered '{}' at '{}'",
            INTENTION.service, INTENTION.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-intentiond] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-intentiond] Running on non-Linux host in headless mode.");
        let _ = core;
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
