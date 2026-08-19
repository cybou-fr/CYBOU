// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-eventd` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

use cybou_crypto::{KeyDomain, KeyStore, Seal};
use cybou_eventd::EventCore;
#[cfg(target_os = "linux")]
use time::OffsetDateTime;

/// How many rows one verification pass replays. Bounded so a long Journal is caught up over
/// several passes instead of blocking one.
#[cfg(target_os = "linux")]
const VERIFICATION_PAGE: u64 = 512;

/// How often a pass runs.
#[cfg(target_os = "linux")]
const VERIFICATION_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let journal_path = env::var("CYBOU_JOURNAL_PATH").map_or_else(
        |_| {
            let data_dir = env::var("XDG_DATA_HOME").map_or_else(
                |_| {
                    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
                    PathBuf::from(home).join(".local/share")
                },
                PathBuf::from,
            );
            data_dir.join("cybou/journal.sqlite3")
        },
        PathBuf::from,
    );

    let keys_dir = env::var("CYBOU_KEYSTORE_PATH").map_or_else(
        |_| {
            journal_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("keys")
        },
        PathBuf::from,
    );

    println!(
        "[cybou-eventd] Opening Journal at {}",
        journal_path.display()
    );
    if let Some(parent) = journal_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let core = Arc::new(EventCore::open(&journal_path)?);

    // Initialize KeyStore if available
    if let Ok(key_store) = KeyStore::open(&keys_dir)
        && let Ok(kek) = Seal::generate_key()
    {
        let domain = KeyDomain::generate(1);
        core.set_key_store(key_store, kek, domain);
        println!(
            "[cybou-eventd] KeyStore initialized at {}",
            keys_dir.display()
        );
    }

    #[cfg(target_os = "linux")]
    {
        use cybou_eventd::service::Event1Service;
        use cybou_fabric::EVENT;

        // Verify the chain in bounded passes rather than on demand. Verification is linear in
        // the length of the Journal, so answering it inside a request would make every reader pay
        // for the whole biography; a background pass catches up once and then tracks the tail.
        let verifier_core = core.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(VERIFICATION_INTERVAL);
            loop {
                interval.tick().await;
                let state = verifier_core.verify_page(VERIFICATION_PAGE, OffsetDateTime::now_utc());
                if let Some(state) = state
                    && let Some(broken_at) = state.broken_at
                {
                    println!(
                        "[cybou-eventd] Journal chain broken at sequence {broken_at}; verified through {}",
                        state.verified_through
                    );
                }
            }
        });

        println!("[cybou-eventd] Connecting to D-Bus session bus...");
        let service = Event1Service::new(core);
        // Bound, not discarded: dropping the connection would release the well-known name.
        let _connection = zbus::connection::Builder::session()?
            .name(EVENT.service)?
            .serve_at(EVENT.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-eventd] Registered '{}' at '{}'",
            EVENT.service, EVENT.object_path
        );

        // Run until terminated
        tokio::signal::ctrl_c().await?;
        println!("[cybou-eventd] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-eventd] Running on non-Linux host in headless mode.");
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
