// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-contextd` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

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

        // Nothing has ever fed this organ: its graph could only change if someone called its
        // mutation methods over the bus, and nobody does. Association is built from the
        // biography, so follow the same acceptance signal the workspace follows.
        tokio::spawn(follow_acceptances(core.clone()));

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

/// Build associative context from the biography as it is written.
///
/// Association is not knowledge. What this follows is what occurred, and what it records is that
/// two subjects occurred in one episode — nothing stronger.
#[cfg(target_os = "linux")]
async fn follow_acceptances(core: Arc<ContextCore>) {
    use std::collections::HashMap;

    use cybou_contextd::AssociationOrigin;
    use cybou_fabric::EVENT;
    use cybou_protocol::{canonical::CanonicalEnvelope, observation::ObservationV1};
    use futures_util::StreamExt;
    use time::OffsetDateTime;
    use uuid::Uuid;

    let context_core = core;
    let Ok(connection) = zbus::Connection::session().await else {
        println!("[cybou-contextd] Cannot observe Event1: no session bus");
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
            println!("[cybou-contextd] Cannot observe Event1: {error}");
            return;
        }
    };
    let mut accepted = match proxy.receive_signal("Accepted").await {
        Ok(stream) => stream,
        Err(error) => {
            println!("[cybou-contextd] Cannot subscribe to Event1 Accepted: {error}");
            return;
        }
    };
    println!("[cybou-contextd] Following Event1 acceptances");

    // The previous subject seen in each episode, so two subjects that occurred in the
    // same episode can be linked. Only the previous one: a chain of pairs, not a clique,
    // because "these two followed one another" is a weaker and more defensible claim than
    // "all of these belong together".
    let mut previous_in_episode: HashMap<Uuid, (String, Uuid)> = HashMap::new();

    while let Some(message) = accepted.next().await {
        let Ok((encoded, _sequence)) = message.body().deserialize::<(Vec<u8>, u64)>() else {
            continue;
        };
        let Ok(envelope) = ciborium::from_reader::<CanonicalEnvelope, _>(encoded.as_slice()) else {
            continue;
        };
        // The subject of the observation, not the organ that reported it: an association
        // between concepts is only meaningful if both ends name something observed.
        let Ok(observation) =
            ciborium::from_reader::<ObservationV1, _>(envelope.payload.as_slice())
        else {
            continue;
        };
        if observation.subject.is_empty() {
            continue;
        }

        let now = OffsetDateTime::from_unix_timestamp_nanos(
            i128::from(envelope.wall_time_ms) * 1_000_000,
        )
        .unwrap_or_else(|_| OffsetDateTime::now_utc());

        context_core.activate(
            observation.subject.clone(),
            envelope.confidence,
            format!("observed by {}", envelope.origin_organ),
            now,
        );

        if let Some((previous, previous_message)) =
            previous_in_episode.get(&envelope.correlation_id)
            && previous != &observation.subject
        {
            // TemporalCooccurrence, deliberately: these two were seen in one episode and
            // nothing more was established. ADR-0029 keeps the origin a closed set exactly
            // so co-occurrence cannot quietly become knowledge.
            context_core.associate(
                previous.clone(),
                observation.subject.clone(),
                envelope.confidence,
                AssociationOrigin::TemporalCooccurrence,
                vec![*previous_message, envelope.message_id],
            );
        }
        previous_in_episode.insert(
            envelope.correlation_id,
            (observation.subject, envelope.message_id),
        );
    }
}
