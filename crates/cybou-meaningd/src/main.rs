// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-meaningd` daemon entrypoint.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-meaningd] Initializing meaning boundary (interpretation != truth)...");

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::MEANING;
        use cybou_meaningd::service::Meaning1Service;

        // Whether the Journal is reachable is checked on an interval rather than inferred from
        // the last time somebody spoke. An organ asked how it is doing should answer about now,
        // and on a quiet system nobody speaks for hours.
        let journal_reachable = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let watched = std::sync::Arc::clone(&journal_reachable);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                let reachable = match cybou_fabric::event_client::EventClient::session().await {
                    Ok(client) => client.count().await.is_ok(),
                    Err(_) => false,
                };
                watched.store(reachable, std::sync::atomic::Ordering::Release);
            }
        });

        println!("[cybou-meaningd] Connecting to D-Bus session bus...");
        let service = Meaning1Service::new(journal_reachable);
        // Bound, not discarded: dropping the connection would release the well-known name.
        let _connection = zbus::connection::Builder::session()?
            .name(MEANING.service)?
            .serve_at(MEANING.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-meaningd] Registered '{}' at '{}'",
            MEANING.service, MEANING.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-meaningd] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-meaningd] Running on non-Linux host in headless mode.");
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
