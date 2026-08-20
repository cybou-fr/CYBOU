// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-lifecycled` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

use cybou_lifecycled::LifecycleCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-lifecycled] Initializing cognitive lifecycle engine...");
    let state_path = env::var("CYBOU_LIFECYCLE_PATH").map_or_else(
        |_| {
            let state_dir = env::var("XDG_STATE_HOME").map_or_else(
                |_| {
                    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
                    PathBuf::from(home).join(".local/state")
                },
                PathBuf::from,
            );
            state_dir.join("cybou/lifecycle.json")
        },
        PathBuf::from,
    );
    let core = Arc::new(LifecycleCore::open(&state_path)?);

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::LIFECYCLE;
        use cybou_lifecycled::service::Lifecycle1Service;

        tokio::spawn(consolidate_when_idle(core.clone()));

        println!("[cybou-lifecycled] Connecting to D-Bus session bus...");
        let service = Lifecycle1Service::new(core);
        // Bound, not discarded: dropping the connection would release the well-known name.
        let _connection = zbus::connection::Builder::session()?
            .name(LIFECYCLE.service)?
            .serve_at(LIFECYCLE.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-lifecycled] Registered '{}' at '{}'",
            LIFECYCLE.service, LIFECYCLE.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-lifecycled] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-lifecycled] Running on non-Linux host in headless mode.");
        let _ = core;
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}

/// How long without a person before the system considers the moment its own.
#[cfg(target_os = "linux")]
const IDLE_BEFORE_CONSOLIDATION: time::Duration = time::Duration::minutes(15);

/// The least time between two full sweeps.
///
/// Idleness alone is not a schedule. On a machine nobody ever touches — a server, which is where
/// this is deployed — "when nobody is present" means always, and a run gated only on idleness
/// would never stop running.
#[cfg(target_os = "linux")]
const BETWEEN_CONSOLIDATIONS: time::Duration = time::Duration::hours(6);

/// Rows per page of the sweep. Small enough that returning to a person costs at most one page.
#[cfg(target_os = "linux")]
const SWEEP_PAGE: u32 = 512;

/// Run the maintenance that belongs to a quiet moment, and stop the instant one ends.
///
/// The only work here is a full re-verification of the chain. The incremental pass trusts a
/// checkpoint and never looks behind it, so a row that rots after it was verified is never
/// questioned again; this is what questions it. Nothing is rewritten — consolidation is not
/// permission to revise a biography — and nothing leaves the machine.
#[cfg(target_os = "linux")]
async fn consolidate_when_idle(core: Arc<LifecycleCore>) {
    use cybou_fabric::EVENT;
    use cybou_lifecycled::LifecycleMode;
    use time::OffsetDateTime;

    let Ok(connection) = zbus::Connection::session().await else {
        println!("[cybou-lifecycled] No session bus; nothing will be consolidated");
        return;
    };

    let mut last_completed: Option<OffsetDateTime> = None;
    let mut interval = tokio::time::interval(std::time::Duration::from_mins(1));

    loop {
        interval.tick().await;
        let now = OffsetDateTime::now_utc();
        let state = core.state();

        if now - state.last_user_activity_at < IDLE_BEFORE_CONSOLIDATION {
            continue;
        }
        if last_completed.is_some_and(|last| now - last < BETWEEN_CONSOLIDATIONS) {
            continue;
        }

        if core.transition(LifecycleMode::Consolidating).is_err() {
            continue;
        }
        println!("[cybou-lifecycled] Consolidating: verifying the chain in full");

        // The activity instant at the start of the run. A person arriving changes it, and that is
        // the signal to stop: the run is interruptible between pages precisely because each page
        // is bounded and the sweep's position is never trusted.
        let began_after = state.last_user_activity_at;
        let mut interrupted = false;

        loop {
            let Some(step) = read_step(&connection, EVENT).await else {
                break;
            };
            if let Some(broken_at) = step.broken_at {
                println!(
                    "[cybou-lifecycled] Consolidation found the chain broken at sequence {broken_at}"
                );
                break;
            }
            if !step.has_more {
                println!(
                    "[cybou-lifecycled] Consolidation verified the whole chain through {}",
                    step.verified_through
                );
                break;
            }
            if core.state().last_user_activity_at != began_after {
                println!("[cybou-lifecycled] Consolidation interrupted: someone is here");
                interrupted = true;
                break;
            }
        }

        let _ = core.transition(LifecycleMode::Awake);
        if !interrupted {
            last_completed = Some(OffsetDateTime::now_utc());
        }
    }
}

/// Ask Event1 to advance the sweep by one page.
#[cfg(target_os = "linux")]
async fn read_step(
    connection: &zbus::Connection,
    endpoint: cybou_fabric::BusEndpoint,
) -> Option<SweepStep> {
    let encoded: Vec<u8> = connection
        .call_method(
            Some(endpoint.service),
            endpoint.object_path,
            Some(endpoint.interface),
            "VerifyFullyStep",
            &(SWEEP_PAGE,),
        )
        .await
        .ok()?
        .body()
        .deserialize()
        .ok()?;
    ciborium::from_reader(encoded.as_slice()).ok()
}

/// Event1's answer for one page of a full sweep.
#[cfg(target_os = "linux")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SweepStep {
    verified_through: u64,
    has_more: bool,
    broken_at: Option<u64>,
}
