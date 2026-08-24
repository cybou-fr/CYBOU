// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-eventd` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

use cybou_crypto::KeyStore;
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

/// Say, at every start, whether one backup would capture both the sealed records and the keys.
///
/// The precondition ADR-0028's whole erasure guarantee rests on, and one that is invisible from
/// inside. Destroying a data key makes a record unreadable in every copy of the database — and only
/// in copies that do not also hold the key. By default the store sits beside the Journal, so the
/// most obvious backup anybody would take captures both, and a restore of it reads exactly what the
/// erasure was meant to reach.
///
/// A warning rather than a different default. Moving the store would leave existing deployments
/// unable to unwrap yesterday's keys, which is the failure this file already learned the hard way
/// and says so above. What an operator needs here is to know, not to be migrated.
fn warn_if_one_backup_would_take_both(keys_dir: &std::path::Path, journal_path: &std::path::Path) {
    if keys_dir.parent() != journal_path.parent() {
        return;
    }
    println!(
        "[cybou-eventd] The key store sits beside the Journal. A backup of {} holds both the sealed records and the keys that open them, which puts it outside the erasure guarantee (ADR-0028 E12). Back them up separately, or set CYBOU_KEYSTORE_PATH.",
        journal_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .display()
    );
}

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

    // The key-encryption key and key domain come from the store, which keeps them across runs.
    // Generating them here made every restart wrap new data keys with a secret that could not
    // unwrap the old ones, so yesterday's sealed payloads became unreadable with nothing recording
    // that anything had been erased.
    match KeyStore::open(&keys_dir).and_then(|store| {
        let (kek, domain) = store.master()?;
        Ok((store, kek, domain))
    }) {
        Ok((key_store, kek, domain)) => {
            println!(
                "[cybou-eventd] KeyStore at {} in domain {} epoch {}",
                keys_dir.display(),
                domain.key_domain_id,
                domain.key_epoch
            );
            warn_if_one_backup_would_take_both(&keys_dir, &journal_path);
            core.set_key_store(key_store, kek, domain);
        }
        Err(error) => {
            // Refuse rather than continue with a fresh secret: a Journal that seals payloads it
            // will never be able to open is worse than one that will not seal them at all.
            eprintln!("[cybou-eventd] Cannot establish key continuity: {error}");
            return Err(error.into());
        }
    }

    // An erasure interrupted between the request and the redaction leaves a person believing
    // something was forgotten that is still there. The request is durable precisely so nobody has
    // to remember, and this is what reads it: before anything else can be served.
    match core.resume_erasures() {
        Ok(0) => {}
        Ok(resumed) => println!("[cybou-eventd] Finished {resumed} interrupted erasure(s)"),
        Err(error) => {
            // Serving a Journal whose erasures did not finish would answer readers with content
            // somebody asked to have destroyed.
            eprintln!("[cybou-eventd] Cannot finish an interrupted erasure: {error}");
            return Err(error.into());
        }
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
