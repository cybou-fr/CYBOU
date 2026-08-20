// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-perceptiond` daemon entrypoint.

use std::{collections::HashMap, sync::Arc, time::Duration};

use cybou_perception::{LinuxHostSource, LinuxSystemSource};
use cybou_perceptiond::PerceptionCore;
use time::OffsetDateTime;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-perceptiond] Initializing Linux system perception engine...");
    let source = LinuxSystemSource::new_standard(300);
    let core = Arc::new(PerceptionCore::new(source));

    let now = OffsetDateTime::now_utc();
    let initial_env = core.acquire_once(now, 0);
    println!(
        "[cybou-perceptiond] Initial perception health: {}",
        core.health()
    );

    // Spawn periodic background acquisition task submitting to Event1
    let sampling_core = core.clone();
    tokio::spawn(async move {
        let host_source = LinuxHostSource::new_standard(300);
        // The last value contributed for each subject. A host fact that has not changed is not
        // news, and re-recording it every minute would bury the moments when one actually did.
        let mut contributed: HashMap<&'static str, String> = HashMap::new();
        let mut interval = tokio::time::interval(Duration::from_mins(1));
        let mut monotonic = 1u64;

        #[cfg(target_os = "linux")]
        let event_client = cybou_fabric::event_client::EventClient::session()
            .await
            .ok();

        loop {
            interval.tick().await;
            let now = OffsetDateTime::now_utc();
            if let Some(envelope) = sampling_core.acquire_once(now, monotonic) {
                monotonic += 1;
                let _ = &envelope;
                #[cfg(target_os = "linux")]
                if let Some(ref client) = event_client
                    && let Ok(res) = client.submit(&envelope).await
                {
                    println!(
                        "[cybou-perceptiond] Submitted observation sequence {} to Event1",
                        res.sequence
                    );
                }
            }

            // One sweep of host facts is one episode: they were observed together, and sharing a
            // correlation identity is what lets anything downstream see that.
            let episode = Uuid::new_v4();
            for observation in host_source.acquire(now) {
                let subject = observation.subject;
                // What is contributed is decided by the value the source read, not by how it
                // happens to be encoded on the wire: skipping anything that was not text meant a
                // count or a size was acquired every sweep and never contributed once.
                let value = observation.value.display();
                let Ok(observation) = observation.into_protocol() else {
                    continue;
                };
                if contributed.get(subject) == Some(&value) {
                    continue;
                }
                let Some(envelope) =
                    PerceptionCore::envelope_for(&observation, episode, now, monotonic)
                else {
                    continue;
                };
                monotonic += 1;
                let _ = &envelope;
                #[cfg(target_os = "linux")]
                if let Some(ref client) = event_client
                    && client.submit(&envelope).await.is_ok()
                {
                    println!("[cybou-perceptiond] Observed {subject} = {value}");
                    contributed.insert(subject, value);
                }
                #[cfg(not(target_os = "linux"))]
                contributed.insert(subject, value);
            }
        }
    });

    let _ = initial_env;

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::PERCEPTION;
        use cybou_perceptiond::service::Perception1Service;

        println!("[cybou-perceptiond] Connecting to D-Bus session bus...");
        let service = Perception1Service::new(core);
        // Bound, not discarded: dropping the connection would release the well-known name.
        let _connection = zbus::connection::Builder::session()?
            .name(PERCEPTION.service)?
            .serve_at(PERCEPTION.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-perceptiond] Registered '{}' at '{}'",
            PERCEPTION.service, PERCEPTION.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-perceptiond] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-perceptiond] Running on non-Linux host in headless mode.");
        let _ = core;
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
