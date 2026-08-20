// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-workspaced` daemon entrypoint.

use std::sync::Arc;

use cybou_workspaced::WorkspaceCore;

/// How many contributions the workspace holds at once, and therefore how many it seeds with.
const WORKSPACE_CAPACITY: u32 = 32;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-workspaced] Initializing Global Workspace attention engine...");
    let core = Arc::new(WorkspaceCore::new(WORKSPACE_CAPACITY as usize));

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::{WORKSPACE, event_client::EventClient};
        use cybou_workspaced::service::Workspace1Service;

        // Attention is about what is recent, so this seeds from the tail rather than replaying the
        // whole biography: a workspace that starts by weighing the system's opening moments
        // deliberates over nothing. Live updates then arrive through the shared follower, which
        // subscribes before it reads, so nothing accepted during startup is missed.
        let seed_core = core.clone();
        tokio::spawn(async move {
            if let Ok(client) = EventClient::session().await
                && let Ok(envelopes) = client.recent(WORKSPACE_CAPACITY).await
            {
                let count = envelopes.len();
                for env in envelopes {
                    seed_core.accept(env);
                }
                println!("[cybou-workspaced] Seeded global workspace with {count} contributions");
            }
        });

        let stream_core = core.clone();
        tokio::spawn(async move {
            // Only what arrives from now on: the tail above is the starting point, and the
            // follower's own catch-up would re-read the whole Journal into a bounded buffer.
            let from = match EventClient::session().await {
                Ok(client) => client.count().await.unwrap_or(0),
                Err(_) => 0,
            };
            if let Err(error) = cybou_fabric::event_client::follow_contributions(
                from,
                move |_sequence, envelope| {
                    stream_core.accept(envelope.clone());
                },
            )
            .await
            {
                println!("[cybou-workspaced] Cannot follow Event1: {error}");
            }
        });

        println!("[cybou-workspaced] Connecting to D-Bus session bus...");
        let service = Workspace1Service::new(core);
        // Bound, not discarded: dropping the connection would release the well-known name.
        let _connection = zbus::connection::Builder::session()?
            .name(WORKSPACE.service)?
            .serve_at(WORKSPACE.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-workspaced] Registered '{}' at '{}'",
            WORKSPACE.service, WORKSPACE.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-workspaced] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-workspaced] Running on non-Linux host in headless mode.");
        let _ = core;
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
