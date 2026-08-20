// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-contextd` daemon entrypoint.

use std::sync::Arc;

#[cfg(target_os = "linux")]
use std::collections::HashMap;

use cybou_contextd::ContextCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-contextd] Initializing Associative Context engine (association != truth)...");

    // The epoch the Journal is already at, asked for before anything is built. It is the one thing
    // here that cannot be derived from the contributions themselves, and starting from zero would
    // make the first check read a long-finished erasure as a fresh one.
    #[cfg(target_os = "linux")]
    let epoch = match cybou_fabric::event_client::EventClient::session().await {
        Ok(client) => client.erasure_epoch().await.unwrap_or(0),
        Err(_) => 0,
    };
    #[cfg(not(target_os = "linux"))]
    let epoch = 0;

    let core = Arc::new(ContextCore::resuming_at_epoch(epoch));

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
                    // Discarding is only half of A7. Everything the erasure did not touch is still
                    // in the Journal and still associates, and an organ that answered from an empty
                    // graph until something new happened would be reporting an absence of context
                    // that is not there. The rebuild is a replay from the start, which is what this
                    // organ does on every start, so it is done by starting again.
                    println!(
                        "[cybou-contextd] Erasure epoch {epoch}: discarding the associative projection and rebuilding it from the surviving Journal"
                    );
                    std::process::exit(1);
                }
            }
        });

        let context_core = core.clone();
        tokio::spawn(async move {
            let mut previous_in_episode: HashMap<uuid::Uuid, ObservedConcept> = HashMap::new();
            let caught_up_core = context_core.clone();
            if let Err(error) = cybou_fabric::event_client::follow_contributions_reporting(
                0,
                move |_sequence, envelope| {
                    activate_from(&context_core, envelope, &mut previous_in_episode);
                },
                move || caught_up_core.mark_caught_up(),
            )
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
    previous_in_episode: &mut std::collections::HashMap<uuid::Uuid, ObservedConcept>,
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

    if let Some(previous) = previous_in_episode.get(&envelope.correlation_id)
        && previous.subject != observation.subject
    {
        // TemporalCooccurrence, deliberately: these two were seen in one episode and nothing more
        // was established. ADR-0029 keeps the origin a closed set exactly so co-occurrence cannot
        // quietly become knowledge.
        //
        // Both ends' classes are applied, because a link is derived from both contributions.
        // Passing only this one's meant the earlier observation's privacy never reached the link
        // it was half of: the association did not exist when that contribution was seen, so there
        // was nothing for it to have tightened.
        for (privacy, sensitivity, retention_class) in [
            (
                previous.privacy,
                previous.sensitivity,
                previous.retention_class,
            ),
            (
                envelope.privacy,
                envelope.sensitivity,
                envelope.retention_class,
            ),
        ] {
            core.associate_with_class(
                previous.subject.clone(),
                observation.subject.clone(),
                envelope.confidence,
                AssociationOrigin::TemporalCooccurrence,
                vec![previous.message_id, envelope.message_id],
                privacy,
                sensitivity,
                retention_class,
            );
        }
    }
    previous_in_episode.insert(
        envelope.correlation_id,
        ObservedConcept {
            subject: observation.subject.clone(),
            message_id: envelope.message_id,
            privacy: envelope.privacy,
            sensitivity: envelope.sensitivity,
            retention_class: envelope.retention_class,
        },
    );
    true
}

/// What an episode remembers about the contribution before this one.
///
/// The classes travel with it because an association inherits from both ends, and the earlier end
/// is gone by the time the link is formed.
#[cfg(target_os = "linux")]
struct ObservedConcept {
    subject: String,
    message_id: uuid::Uuid,
    privacy: u8,
    sensitivity: u8,
    retention_class: u8,
}
