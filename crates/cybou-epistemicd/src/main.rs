// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-epistemicd` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

use cybou_epistemicd::EpistemicCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "[cybou-epistemicd] Initializing Epistemic projection engine (observation != knowledge)..."
    );
    let state_path = env::var("CYBOU_EPISTEMIC_PATH").map_or_else(
        |_| {
            let state_dir = env::var("XDG_STATE_HOME").map_or_else(
                |_| {
                    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
                    PathBuf::from(home).join(".local/state")
                },
                PathBuf::from,
            );
            state_dir.join("cybou/epistemic.json")
        },
        PathBuf::from,
    );

    let core = Arc::new(EpistemicCore::open(&state_path)?);

    #[cfg(target_os = "linux")]
    {
        use cybou_epistemicd::service::Epistemic1Service;
        use cybou_fabric::{EPISTEMIC, event_client::EventClient};

        // Perform initial catch-up replay from Event1
        let replay_core = core.clone();
        tokio::spawn(async move {
            if let Ok(client) = EventClient::session().await {
                let start_cursor = replay_core.cursor();
                if let Ok(envelopes) = client.replay(start_cursor, 500).await {
                    let mut seq = start_cursor;
                    let batch: Vec<_> = envelopes
                        .into_iter()
                        .map(|e| {
                            seq += 1;
                            (seq, e)
                        })
                        .collect();
                    replay_core.replay_batch(&batch);
                    println!(
                        "[cybou-epistemicd] Replayed {} events up to cursor {}",
                        batch.len(),
                        replay_core.cursor()
                    );
                }
            }
        });

        // The replay above catches up once and then stops. Every contribution accepted afterwards
        // would go unseen, which is why a system observing four new subjects still held exactly
        // one belief: the beliefs were formed at startup and nothing revised them since.
        tokio::spawn(follow_acceptances(core.clone()));

        println!("[cybou-epistemicd] Connecting to D-Bus session bus...");
        let service = Epistemic1Service::new(core);
        // Bound, not discarded: dropping the connection would release the well-known name.
        let _connection = zbus::connection::Builder::session()?
            .name(EPISTEMIC.service)?
            .serve_at(EPISTEMIC.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-epistemicd] Registered '{}' at '{}'",
            EPISTEMIC.service, EPISTEMIC.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-epistemicd] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-epistemicd] Running on non-Linux host in headless mode.");
        let _ = core;
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}

/// Form beliefs from contributions as they are accepted.
///
/// A projection that is only rebuilt at startup describes the system as it was when the process
/// began, which is a different claim from what the system currently believes.
#[cfg(target_os = "linux")]
async fn follow_acceptances(core: Arc<EpistemicCore>) {
    use cybou_fabric::EVENT;
    use cybou_protocol::canonical::CanonicalEnvelope;
    use futures_util::StreamExt;

    let Ok(connection) = zbus::Connection::session().await else {
        println!("[cybou-epistemicd] Cannot observe Event1: no session bus");
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
            println!("[cybou-epistemicd] Cannot observe Event1: {error}");
            return;
        }
    };
    let mut accepted = match proxy.receive_signal("Accepted").await {
        Ok(stream) => stream,
        Err(error) => {
            println!("[cybou-epistemicd] Cannot subscribe to Event1 Accepted: {error}");
            return;
        }
    };
    println!("[cybou-epistemicd] Following Event1 acceptances");

    while let Some(message) = accepted.next().await {
        let Ok((encoded, sequence)) = message.body().deserialize::<(Vec<u8>, u64)>() else {
            continue;
        };
        if let Ok(envelope) = ciborium::from_reader::<CanonicalEnvelope, _>(encoded.as_slice()) {
            core.ingest_envelope(&envelope, sequence);
        }
    }
}
