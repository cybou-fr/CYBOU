// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-contextd` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

#[cfg(target_os = "linux")]
use std::collections::HashMap;

use cybou_contextd::ContextCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-contextd] Initializing Associative Context engine (association != truth)...");
    let state_path = env::var("CYBOU_CONTEXT_PATH").map_or_else(
        |_| {
            let state_dir = env::var("XDG_STATE_HOME").map_or_else(
                |_| {
                    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
                    PathBuf::from(home).join(".local/state")
                },
                PathBuf::from,
            );
            state_dir.join("cybou/context.json")
        },
        PathBuf::from,
    );

    let core = Arc::new(ContextCore::open(&state_path)?);

    #[cfg(target_os = "linux")]
    {
        use cybou_contextd::service::Context1Service;
        use cybou_fabric::CONTEXT;

        // Association is built from the biography, so this follows the same primitive every other
        // derived organ follows: subscribe, catch up to the head in pages, then stay live. The
        // graph is derived state and is rebuilt from the Journal rather than remembered.
        // ADR-0029 A7: the projection is invalid once the Journal has erased anything it was
        // derived from. Checked before following, so a restart after an erasure rebuilds rather
        // than resuming on associations whose evidence is gone.
        let epoch_core = core.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                let Ok(client) = cybou_fabric::event_client::EventClient::session().await else {
                    continue;
                };
                let Ok(epoch) = client.erasure_epoch().await else {
                    continue;
                };
                if epoch_core.invalidate_for_epoch(epoch) {
                    println!(
                        "[cybou-contextd] Erasure epoch {epoch}: associative projection discarded"
                    );
                }
            }
        });

        let context_core = core.clone();
        tokio::spawn(async move {
            let mut previous_in_episode: HashMap<uuid::Uuid, (String, uuid::Uuid)> = HashMap::new();
            if let Err(error) =
                cybou_fabric::event_client::follow_contributions(0, move |_sequence, envelope| {
                    activate_from(&context_core, envelope, &mut previous_in_episode);
                })
                .await
            {
                println!("[cybou-contextd] Cannot follow Event1: {error}");
            }
        });

        println!("[cybou-contextd] Connecting to D-Bus session bus...");
        let service = Context1Service::new(core);
        // Bound, not discarded: dropping the connection would release the well-known name.
        let _connection = zbus::connection::Builder::session()?
            .name(CONTEXT.service)?
            .serve_at(CONTEXT.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-contextd] Registered '{}' at '{}'",
            CONTEXT.service, CONTEXT.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-contextd] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-contextd] Running on non-Linux host in headless mode.");
        let _ = core;
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}

/// Activate the concept an envelope observed, and link it to the previous subject of its episode.
///
/// Returns whether anything was activated, which is false for a contribution that carries no
/// readable observation — a payload that does not decode names no subject, and a concept has to be
/// about something.
#[cfg(target_os = "linux")]
fn activate_from(
    core: &ContextCore,
    envelope: &cybou_protocol::canonical::CanonicalEnvelope,
    previous_in_episode: &mut std::collections::HashMap<uuid::Uuid, (String, uuid::Uuid)>,
) -> bool {
    use cybou_contextd::AssociationOrigin;
    use cybou_protocol::observation::ObservationV1;
    use time::OffsetDateTime;

    // The subject of the observation, not the organ that reported it: an association between
    // concepts is only meaningful if both ends name something observed.
    let Ok(observation) = ciborium::from_reader::<ObservationV1, _>(envelope.payload.as_slice())
    else {
        return false;
    };
    if observation.subject.is_empty() {
        return false;
    }

    let now =
        OffsetDateTime::from_unix_timestamp_nanos(i128::from(envelope.wall_time_ms) * 1_000_000)
            .unwrap_or_else(|_| OffsetDateTime::now_utc());

    core.activate(
        observation.subject.clone(),
        envelope.confidence,
        format!("observed by {}", envelope.origin_organ),
        now,
    );

    if let Some((previous, previous_message)) = previous_in_episode.get(&envelope.correlation_id)
        && previous != &observation.subject
    {
        // TemporalCooccurrence, deliberately: these two were seen in one episode and nothing more
        // was established. ADR-0029 keeps the origin a closed set exactly so co-occurrence cannot
        // quietly become knowledge.
        core.associate(
            previous.clone(),
            observation.subject.clone(),
            envelope.confidence,
            AssociationOrigin::TemporalCooccurrence,
            vec![*previous_message, envelope.message_id],
        );
    }
    previous_in_episode.insert(
        envelope.correlation_id,
        (observation.subject.clone(), envelope.message_id),
    );
    true
}
