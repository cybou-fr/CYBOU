// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-healthd` daemon entrypoint.

use std::sync::Arc;

#[cfg(target_os = "linux")]
use std::collections::HashMap;

#[cfg(target_os = "linux")]
use std::time::Duration;

#[allow(unused_imports)]
use cybou_healthd::{ComponentHealth, ComponentHealthRecord, HealthCore};
#[cfg(target_os = "linux")]
use time::OffsetDateTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-healthd] Initializing capability health engine...");
    let core = Arc::new(HealthCore::new());

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::{
            CONTEXT, EPISTEMIC, EVENT, HEALTH, IDENTITY, INTENTION, LIFECYCLE, PERCEPTION,
            PREDICTOR, PRESENCE, SELF, WORKSPACE,
        };
        use cybou_healthd::service::{Health1Service, emit_changed};

        let _ = core.recalculate(OffsetDateTime::now_utc());
        println!(
            "[cybou-healthd] Initial overall health: {}",
            core.overall_health()
        );

        println!("[cybou-healthd] Connecting to D-Bus session bus...");
        let probe_core = core.clone();
        let service = Health1Service::new(core);
        // Bound, not discarded: dropping the connection would release the well-known name.
        let connection = zbus::connection::Builder::session()?
            .name(HEALTH.service)?
            .serve_at(HEALTH.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-healthd] Registered '{}' at '{}'",
            HEALTH.service, HEALTH.object_path
        );

        // The probe loop shares the owning connection: it both dispatches the Ready probes and
        // emits Changed, and a signal sent over a second connection would carry a sender no
        // subscriber matches.
        let probe_connection = connection.clone();
        tokio::spawn(async move {
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

            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                let now = OffsetDateTime::now_utc();

                let mut records = HashMap::new();
                for (id, ep) in &endpoints {
                    let res: Result<bool, _> = probe_connection
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

                    let mut health = match res {
                        Ok(true) => ComponentHealth::Healthy,
                        Ok(false) => ComponentHealth::Degraded,
                        Err(_) => ComponentHealth::Unavailable,
                    };

                    // Answering is not the same as being sound. Event1 replies Ready while its
                    // chain is broken, so a corrupted biography would leave every capability
                    // reading available — the system would report itself healthy about the one
                    // thing it exists to keep. Readiness stays a question about the process; this
                    // asks the separate question about what it holds.
                    if *id == "eventd" && health == ComponentHealth::Healthy {
                        health = journal_integrity(&probe_connection).await;
                    }

                    records.insert(
                        (*id).to_string(),
                        ComponentHealthRecord {
                            health,
                            detail: None,
                        },
                    );
                }

                // One probe round is one observation: applied together so the transition is
                // reported once, and only when an observer could actually see the difference.
                if probe_core.set_components(records, now) {
                    let _ = emit_changed(&probe_connection).await;
                }
            }
        });

        tokio::signal::ctrl_c().await?;
        println!("[cybou-healthd] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-healthd] Running on non-Linux host in headless mode.");
        let _ = core;
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}

/// Judge Event1 by what its verification established, not only by its willingness to answer.
///
/// A chain that has been replayed to the head and found intact is healthy. A chain with a known
/// break is conflicted: the process is alive and its contents contradict themselves, which is a
/// different failure from a process that is gone. A verification that has not reached the head yet
/// says nothing either way, so it says nothing.
#[cfg(target_os = "linux")]
async fn journal_integrity(connection: &zbus::Connection) -> ComponentHealth {
    use cybou_fabric::EVENT;

    let Ok(reply) = connection
        .call_method(
            Some(EVENT.service),
            EVENT.object_path,
            Some(EVENT.interface),
            "Verification",
            &(),
        )
        .await
    else {
        return ComponentHealth::Healthy;
    };
    let Ok(encoded) = reply.body().deserialize::<Vec<u8>>() else {
        return ComponentHealth::Healthy;
    };
    if encoded.is_empty() {
        return ComponentHealth::Healthy;
    }
    let Ok(state) = ciborium::from_reader::<JournalVerification, _>(encoded.as_slice()) else {
        return ComponentHealth::Healthy;
    };

    if state.broken_at.is_some() {
        ComponentHealth::Conflicted
    } else {
        ComponentHealth::Healthy
    }
}

/// What Event1 established about its own chain.
#[cfg(target_os = "linux")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct JournalVerification {
    broken_at: Option<u64>,
}
