// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-epistemicd` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

use cybou_epistemicd::EpistemicCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-epistemicd] Initializing Epistemic projection engine (observation != knowledge)...");
    let state_path = env::var("CYBOU_EPISTEMIC_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let state_dir = env::var("XDG_STATE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
                    PathBuf::from(home).join(".local/state")
                });
            state_dir.join("cybou/epistemic.json")
        });

    let core = Arc::new(EpistemicCore::open(&state_path)?);

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::{event_client::EventClient, EPISTEMIC};
        use cybou_epistemicd::service::Epistemic1Service;

        // Perform initial catch-up replay from Event1
        let replay_core = core.clone();
        tokio::spawn(async move {
            if let Ok(client) = EventClient::session().await {
                let start_cursor = replay_core.cursor();
                if let Ok(envelopes) = client.replay(start_cursor, 500).await {
                    let mut seq = start_cursor;
                    let batch: Vec<_> = envelopes.into_iter().map(|e| {
                        seq += 1;
                        (seq, e)
                    }).collect();
                    replay_core.replay_batch(&batch);
                    println!(
                        "[cybou-epistemicd] Replayed {} events up to cursor {}",
                        batch.len(),
                        replay_core.cursor()
                    );
                }
            }
        });

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
