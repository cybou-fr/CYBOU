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
        use cybou_fabric::{HEALTH, PRESENCE};
        use cybou_presenced::service::{Presence1Service, emit_changed};
        use futures_util::StreamExt;

        println!("[cybou-presenced] Connecting to D-Bus session bus...");
        let service = Presence1Service::new(core);
        // Bound, not discarded: dropping the connection would release the well-known name.
        let connection = zbus::connection::Builder::session()?
            .name(PRESENCE.service)?
            .serve_at(PRESENCE.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-presenced] Registered '{}' at '{}'",
            PRESENCE.service, PRESENCE.object_path
        );

        // Presence1 presents what other owners hold, so it has nothing of its own to change on.
        // Health1 is the only owner behind the compound projection today: re-emit its Changed as
        // Presence1's, so a subscriber waiting on Presence1 is woken by the same facts.
        let signal_connection = connection.clone();
        tokio::spawn(async move {
            let proxy = match zbus::Proxy::new(
                &signal_connection,
                HEALTH.service,
                HEALTH.object_path,
                HEALTH.interface,
            )
            .await
            {
                Ok(proxy) => proxy,
                Err(error) => {
                    println!("[cybou-presenced] Cannot observe Health1: {error}");
                    return;
                }
            };
            let mut changed = match proxy.receive_signal("Changed").await {
                Ok(stream) => stream,
                Err(error) => {
                    println!("[cybou-presenced] Cannot subscribe to Health1 Changed: {error}");
                    return;
                }
            };
            while changed.next().await.is_some() {
                let _ = emit_changed(&signal_connection).await;
            }
        });

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
