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

/// Say, at every start, where the keys are and whether one backup would take them with the records.
///
/// The precondition ADR-0028's whole erasure guarantee rests on, and one that is invisible from
/// inside. Destroying a data key makes a record unreadable in every copy of the database — and only
/// in copies that do not also hold the key.
///
/// A fresh installation now keeps the two apart. An existing store is never moved, because doing
/// that would leave a deployment unable to unwrap yesterday's keys, and this file learned once what
/// that costs. So the old layout is still reachable and is still reported, every start, for as long
/// as somebody is running it.
fn report_where_the_keys_are(keys: &cybou_eventd::KeysLocation, journal_path: &std::path::Path) {
    if !keys.because.one_backup_takes_both() {
        return;
    }
    println!(
        "[cybou-eventd] The key store sits beside the Journal, where this installation created it. A backup of {} holds both the sealed records and the keys that open them, which puts it outside the erasure guarantee (ADR-0028 E12). Back them up separately, or move the store and set CYBOU_KEYSTORE_PATH. Nothing is moved for you: a store this process cannot find is yesterday's payloads becoming unreadable.",
        journal_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .display()
    );
}

/// One of the XDG roots, or the fallback a login shell would have used.
fn xdg_root(variable: &str, fallback: &str) -> PathBuf {
    env::var(variable).map_or_else(
        |_| {
            let home = env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(fallback)
        },
        PathBuf::from,
    )
}

/// Where this installation keeps its Journal, and where it keeps the keys that open it.
///
/// The two roots are deliberately different. ADR-0017 already separates data from state, and this
/// is the first thing for which the distinction is more than tidiness: a backup of the data
/// directory must not carry the keys that undo an erasure of what is in it.
fn where_things_live() -> (PathBuf, cybou_eventd::KeysLocation) {
    let journal_path = env::var("CYBOU_JOURNAL_PATH").map_or_else(
        |_| xdg_root("XDG_DATA_HOME", ".local/share").join("cybou/journal.sqlite3"),
        PathBuf::from,
    );
    let state_dir = xdg_root("XDG_STATE_HOME", ".local/state").join("cybou");

    let keys = cybou_eventd::keys_location::decide(
        env::var("CYBOU_KEYSTORE_PATH").ok().map(PathBuf::from),
        &journal_path,
        &state_dir,
        // `master.json` and nothing else. A directory is a key store when it holds the master
        // secret this organ wraps everything else with; a directory that merely exists, or holds a
        // stray file, is not one, and treating it as one would keep a deployment pointed at a
        // store with no keys in it.
        |path| path.join("master.json").is_file(),
    );
    (journal_path, keys)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (journal_path, keys) = where_things_live();
    let keys_dir = keys.path.clone();

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
            report_where_the_keys_are(&keys, &journal_path);
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
