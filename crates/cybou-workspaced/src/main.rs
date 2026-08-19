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
        use cybou_fabric::{EVENT, WORKSPACE, event_client::EventClient};
        use cybou_protocol::canonical::CanonicalEnvelope;
        use cybou_workspaced::service::Workspace1Service;
        use futures_util::StreamExt;

        // Seed from the newest contributions, not the oldest. `replay(0, …)` returns the
        // beginning of the Journal, so a workspace seeded that way opens attention on whatever
        // the system did first and, with a two-minute half-life, weighs it at nothing.
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

        // A workspace that only ever sees its seed is not global: it would keep deliberating over
        // the same contributions while the system moved on. Event1 announces every acceptance, so
        // follow that instead of polling for it.
        let stream_core = core.clone();
        tokio::spawn(async move {
            let Ok(connection) = zbus::Connection::session().await else {
                println!("[cybou-workspaced] Cannot observe Event1: no session bus");
                return;
            };
            let proxy = match zbus::Proxy::new(
                &connection,
                EVENT.service,
                EVENT.object_path,
                EVENT.interface,
            )
            .await
            {
                Ok(proxy) => proxy,
                Err(error) => {
                    println!("[cybou-workspaced] Cannot observe Event1: {error}");
                    return;
                }
            };
            let mut accepted = match proxy.receive_signal("Accepted").await {
                Ok(stream) => stream,
                Err(error) => {
                    println!("[cybou-workspaced] Cannot subscribe to Event1 Accepted: {error}");
                    return;
                }
            };
            println!("[cybou-workspaced] Following Event1 acceptances");
            while let Some(message) = accepted.next().await {
                let Ok((encoded, _sequence)) = message.body().deserialize::<(Vec<u8>, u64)>()
                else {
                    continue;
                };
                if let Ok(envelope) =
                    ciborium::from_reader::<CanonicalEnvelope, _>(encoded.as_slice())
                {
                    stream_core.accept(envelope);
                }
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
