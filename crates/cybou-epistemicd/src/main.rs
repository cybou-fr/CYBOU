// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-epistemicd` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

use cybou_epistemicd::EpistemicCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "[cybou-epistemicd] Initializing Epistemic projection engine (observation != knowledge)..."
    );
    let state_path = env::var("CYBOU_EPISTEMIC_PATH").map_or_else(
        |_| {
            let state_dir = env::var("XDG_STATE_HOME").map_or_else(
                |_| {
                    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
                    PathBuf::from(home).join(".local/state")
                },
                PathBuf::from,
            );
            state_dir.join("cybou/epistemic.json")
        },
        PathBuf::from,
    );

    let core = Arc::new(EpistemicCore::open(&state_path)?);

    #[cfg(target_os = "linux")]
    {
        use cybou_epistemicd::service::Epistemic1Service;
        use cybou_fabric::EPISTEMIC;

        // One primitive replaces the replay-then-subscribe pair this organ used to run as two
        // independent tasks: it subscribes first, catches up in pages to the head it saw, and
        // then stays live, so nothing is read twice and nothing falls between the two phases.
        let follow_core = core.clone();
        tokio::spawn(async move {
            let from = follow_core.cursor();
            if let Err(error) =
                cybou_fabric::event_client::follow_contributions(from, move |sequence, envelope| {
                    follow_core.ingest_envelope(envelope, sequence);
                })
                .await
            {
                println!("[cybou-epistemicd] Cannot follow Event1: {error}");
            }
        });

        println!("[cybou-epistemicd] Connecting to D-Bus session bus...");
        let service = Epistemic1Service::new(core);
        // Bound, not discarded: dropping the connection would release the well-known name.
        let _connection = zbus::connection::Builder::session()?
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
