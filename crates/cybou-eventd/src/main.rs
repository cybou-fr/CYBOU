// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-eventd` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

use cybou_crypto::{KeyDomain, KeyStore, Seal};
use cybou_eventd::EventCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let journal_path = env::var("CYBOU_JOURNAL_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let data_dir = env::var("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
                    PathBuf::from(home).join(".local/share")
                });
            data_dir.join("cybou/journal.sqlite3")
        });

    let keys_dir = env::var("CYBOU_KEYSTORE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            journal_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("keys")
        });

    println!("[cybou-eventd] Opening Journal at {}", journal_path.display());
    if let Some(parent) = journal_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let core = Arc::new(EventCore::open(&journal_path)?);

    // Initialize KeyStore if available
    if let Ok(key_store) = KeyStore::open(&keys_dir) {
        if let Ok(kek) = Seal::generate_key() {
            let domain = KeyDomain::generate(1);
            core.set_key_store(key_store, kek, domain);
            println!("[cybou-eventd] KeyStore initialized at {}", keys_dir.display());
        }
    }

    #[cfg(target_os = "linux")]
    {
        use cybou_eventd::service::Event1Service;
        use cybou_fabric::EVENT;

        println!("[cybou-eventd] Connecting to D-Bus session bus...");
        let service = Event1Service::new(core);
        let connection = zbus::connection::Builder::session()?
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
