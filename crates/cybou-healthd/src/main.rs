// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-healthd` daemon entrypoint.

use std::{sync::Arc, time::Duration};

#[allow(unused_imports)]
use cybou_healthd::{ComponentHealth, ComponentHealthRecord, HealthCore};
use time::OffsetDateTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-healthd] Initializing capability health engine...");
    let core = Arc::new(HealthCore::new());

    core.recalculate(OffsetDateTime::now_utc());
    println!(
        "[cybou-healthd] Initial overall health: {}",
        core.overall_health()
    );

    // Spawn periodic active probing loop
    let probe_core = core.clone();
    tokio::spawn(async move {
        #[cfg(target_os = "linux")]
        use cybou_fabric::{
            CONTEXT, EPISTEMIC, EVENT, IDENTITY, INTENTION, LIFECYCLE, PERCEPTION, PREDICTOR,
            PRESENCE, SELF, WORKSPACE,
        };

        let mut interval = tokio::time::interval(Duration::from_secs(5));
        #[cfg(target_os = "linux")]
        let session = zbus::Connection::session().await.ok();

        #[cfg(target_os = "linux")]
        let endpoints = [
            ("eventd", EVENT),
            ("identityd", IDENTITY),
            ("intentiond", INTENTION),
            ("predictord", PREDICTOR),
            ("selfd", SELF),
            ("workspaced", WORKSPACE),
            ("perceptiond", PERCEPTION),
            ("epistemicd", EPISTEMIC),
            ("contextd", CONTEXT),
            ("lifecycled", LIFECYCLE),
            ("presenced", PRESENCE),
        ];

        loop {
            interval.tick().await;
            let now = OffsetDateTime::now_utc();

            #[cfg(target_os = "linux")]
            if let Some(ref conn) = session {
                for (id, ep) in &endpoints {
                    let res: Result<bool, _> = conn
                        .call_method(
                            Some(ep.service),
                            ep.object_path,
                            Some(ep.interface),
                            "Ready",
                            &(),
                        )
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r.body().deserialize().map_err(|e| e.to_string()));

                    let health = match res {
                        Ok(true) => ComponentHealth::Healthy,
                        Ok(false) => ComponentHealth::Degraded,
                        Err(_) => ComponentHealth::Unavailable,
                    };

                    probe_core.update_component(
                        *id,
                        ComponentHealthRecord {
                            health,
                            detail: None,
                        },
                        now,
                    );
                }
            }

            probe_core.recalculate(now);
        }
    });

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::HEALTH;
        use cybou_healthd::service::Health1Service;

        println!("[cybou-healthd] Connecting to D-Bus session bus...");
        let service = Health1Service::new(core);
        // Bound, not discarded: dropping the connection would release the well-known name.
        let _connection = zbus::connection::Builder::session()?
            .name(HEALTH.service)?
            .serve_at(HEALTH.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-healthd] Registered '{}' at '{}'",
            HEALTH.service, HEALTH.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-healthd] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-healthd] Running on non-Linux host in headless mode.");
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
