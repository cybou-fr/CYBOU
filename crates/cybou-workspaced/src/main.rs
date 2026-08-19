// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-workspaced` daemon entrypoint.

use std::sync::Arc;

use cybou_workspaced::WorkspaceCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-workspaced] Initializing Global Workspace attention engine...");
    let core = Arc::new(WorkspaceCore::new(32));

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::{event_client::EventClient, WORKSPACE};
        use cybou_workspaced::service::Workspace1Service;

        // Perform initial catch-up replay to seed the global workspace
        let seed_core = core.clone();
        tokio::spawn(async move {
            if let Ok(client) = EventClient::session().await {
                if let Ok(envelopes) = client.replay(0, 32).await {
                    for env in envelopes {
                        seed_core.accept(env);
                    }
                    println!(
                        "[cybou-workspaced] Seeded global workspace with initial recent contributions"
                    );
                }
            }
        });

        println!("[cybou-workspaced] Connecting to D-Bus session bus...");
        let service = Workspace1Service::new(core);
        let connection = zbus::connection::Builder::session()?
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
