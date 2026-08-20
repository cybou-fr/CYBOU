// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-identityd` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

use cybou_identityd::{ARCHITECTURE_VERSION, IdentityCore};
use time::OffsetDateTime;

/// How many times a session start is offered to the Journal before giving up out loud.
///
/// With the backoff below this spans several minutes, which covers a slow start of the whole Mind
/// without leaving a process retrying into a Journal that is never coming back.
#[cfg(target_os = "linux")]
const SESSION_START_ATTEMPTS: u32 = 12;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state_path = env::var("CYBOU_IDENTITY_PATH").map_or_else(
        |_| {
            let state_dir = env::var("XDG_STATE_HOME").map_or_else(
                |_| {
                    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
                    PathBuf::from(home).join(".local/state")
                },
                PathBuf::from,
            );
            state_dir.join("cybou/identity.json")
        },
        PathBuf::from,
    );

    println!(
        "[cybou-identityd] Managing identity at {}",
        state_path.display()
    );
    let core = Arc::new(IdentityCore::open(&state_path));

    let now = OffsetDateTime::now_utc();
    let action = core.begin_session(now, ARCHITECTURE_VERSION)?;
    println!("[cybou-identityd] Session initialized with action: {action:?}");

    if let Some(state) = core.current_state() {
        println!(
            "[cybou-identityd] Identity ID: {}, Session: {}, Age: {} days",
            state.identity_id,
            state.session_count,
            state.age_in_days()
        );
    }

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::{IDENTITY, event_client::EventClient};
        use cybou_identityd::service::Identity1Service;

        if let Some(envelope) = core.build_envelope(&action, now, 0) {
            let recording_core = Arc::clone(&core);
            let recorded_id = envelope.message_id;
            tokio::spawn(async move {
                // One attempt, made at the moment the organs are all starting at once, was the
                // only chance this contribution ever got. An Event1 that was not listening yet
                // left the identity holding a session it had counted and the Journal had never
                // heard of — and the count is the very thing a restart is supposed to be able to
                // prove. It keeps trying until the Journal has it, or until the Journal has
                // considered it and said no, which is an answer rather than an absence.
                let mut wait = std::time::Duration::from_secs(1);
                for attempt in 1..=SESSION_START_ATTEMPTS {
                    let outcome = match EventClient::session().await {
                        Ok(client) => client.submit(&envelope).await,
                        Err(error) => Err(error),
                    };
                    match outcome {
                        Ok(res) => {
                            println!(
                                "[cybou-identityd] Submitted session start sequence {} to Event1",
                                res.sequence
                            );
                            recording_core.record_session_start(recorded_id);
                            return;
                        }
                        Err(cybou_fabric::event_client::EventClientError::Rejected(reason)) => {
                            println!(
                                "[cybou-identityd] Event1 refused the session start: {reason}"
                            );
                            return;
                        }
                        Err(error) => {
                            println!(
                                "[cybou-identityd] Could not record the session start (attempt {attempt}): {error}"
                            );
                        }
                    }
                    tokio::time::sleep(wait).await;
                    wait = (wait * 2).min(std::time::Duration::from_secs(30));
                }
                println!(
                    "[cybou-identityd] Gave up recording the session start; this session is not in the biography"
                );
            });
        }

        println!("[cybou-identityd] Connecting to D-Bus session bus...");
        let service = Identity1Service::new(core);
        // Bound, not discarded: dropping the connection would release the well-known name.
        let _connection = zbus::connection::Builder::session()?
            .name(IDENTITY.service)?
            .serve_at(IDENTITY.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-identityd] Registered '{}' at '{}'",
            IDENTITY.service, IDENTITY.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-identityd] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-identityd] Running on non-Linux host in headless mode.");
        let _ = core;
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
