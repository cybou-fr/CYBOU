// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What a restored backup can and cannot read after an erasure (ADR-0028 E11, E12).
//!
//! ADR-0028 says the thing plainly: a backup taken before an erasure still holds the ciphertext, and
//! only a destroyed key reaches it. Every other test in this tree checks the live database, which is
//! the copy the erasure ran against — it can prove the row was redacted and cannot prove anything
//! about a copy nobody controlled.
//!
//! So this one takes an actual backup. `journal.sqlite3` is copied byte for byte before the erasure,
//! the erasure runs, and the copy is opened afterwards. Nothing is simulated: the file on disk after
//! the copy is what a restore would produce.
//!
//! ## The trap the default layout sets
//!
//! The guarantee holds only because the key is somewhere the backup did not reach. `cybou-eventd`
//! puts the key store in `keys/` **beside** `journal.sqlite3`, so the most obvious backup anybody
//! would take — `tar czf backup.tgz ~/.local/share/cybou/` — captures both, and a restore of it can
//! read everything the erasure was supposed to make unreadable.
//!
//! That is not a defect in the crypto and it cannot be fixed by the crypto. It is a fact about what
//! a deployment must exclude, and the second test here holds it as a fact rather than leaving it to
//! be discovered. A guarantee whose precondition is undocumented and untested is a guarantee that
//! will be reported as holding on the day it does not.

use std::fs;

use cybou_crypto::{KeyStore, SEAL_NONCE_BYTES, Seal, SealedPayload};
use cybou_eventd::EventCore;
use cybou_protocol::{Kind, admission::ErasureReason, canonical::CanonicalEnvelope};
use uuid::Uuid;

/// Something private, sealed, in a journal with its keys in a directory of their own.
fn sealed_contribution() -> CanonicalEnvelope {
    CanonicalEnvelope {
        schema_version: 4,
        message_id: Uuid::from_u128(0x5ea1_ed01),
        correlation_id: Uuid::from_u128(1),
        causation_id: Uuid::nil(),
        origin_organ: "testd".to_owned(),
        origin_node: String::new(),
        kind: Kind::Observation as u16,
        wall_time_ms: 1_787_000_000_000,
        monotonic_time: 0,
        logical_clock: 1,
        confidence: 1.0,
        evidence: Vec::new(),
        payload: b"the thing a person asked to be forgotten".to_vec(),
        privacy: 1,
        capability_scope: String::new(),
        sealed: true,
        key_domain_id: Uuid::nil(),
        key_epoch: 0,
        retention_class: 2,
        retention_policy_version: 0,
        retain_until_ms: 0,
        sensitivity: 1,
    }
}

/// A journal and a key store, in directories a backup can be told apart by.
fn journal_with_keys(
    root: &std::path::Path,
) -> (std::path::PathBuf, std::path::PathBuf, EventCore) {
    let journal = root.join("data/journal.sqlite3");
    let keys = root.join("secrets");
    fs::create_dir_all(journal.parent().expect("a parent")).expect("data dir");

    let core = EventCore::open(&journal).expect("a journal");
    let store = KeyStore::open(&keys).expect("a key store");
    let (kek, domain) = store.master().expect("a master key");
    core.set_key_store(store, kek, domain);
    (journal, keys, core)
}

/// Copy a journal the way a filesystem backup would take it.
///
/// The main file and its siblings. `SQLite` in WAL mode holds recent writes in
/// `journal.sqlite3-wal`
/// until a checkpoint moves them, so `cp journal.sqlite3 backup/` on a running system produces a
/// file that opens cleanly and is missing the most recent contributions — which is worse than a
/// backup that fails, because it restores and looks right. The first draft of this test did exactly
/// that and could not find the row it had just written.
fn copy_journal(from: &std::path::Path, to: &std::path::Path) {
    fs::create_dir_all(to.parent().expect("a parent")).expect("backup dir");
    let name = from.file_name().expect("a file name").to_string_lossy();
    let dir = from.parent().expect("a parent");
    for entry in fs::read_dir(dir).expect("the journal directory is readable") {
        let entry = entry.expect("an entry");
        let entry_name = entry.file_name().to_string_lossy().into_owned();
        if !entry_name.starts_with(name.as_ref()) {
            continue;
        }
        let target = to.with_file_name(entry_name.replace(name.as_ref(), &to_name(to)));
        fs::copy(entry.path(), target).expect("copied");
    }
}

/// The base file name a copy is written under.
fn to_name(to: &std::path::Path) -> String {
    to.file_name()
        .expect("a file name")
        .to_string_lossy()
        .into_owned()
}

/// What the stored bytes of one contribution are, in a database file opened as a journal.
///
/// This *is* the restore. Putting the copied file back and starting the daemon on it is what a
/// person recovering from a backup does, and it is what this does.
fn stored_payload(journal: &std::path::Path, message_id: &Uuid) -> Vec<u8> {
    let restored = EventCore::open(journal).expect("the copy opens as a journal");
    restored
        .find_by_message_id(message_id)
        .expect("the row is in the copy")
        .payload
}

/// The stored bytes, split back into the shape the sealer produced.
///
/// The writer stores nonce and ciphertext concatenated. Splitting them here rather than reaching
/// for a helper keeps the test honest about what is on disk: an attacker with a backup has these
/// bytes and nothing else.
fn as_sealed(stored: &[u8]) -> SealedPayload {
    let (nonce, ciphertext) = stored.split_at(SEAL_NONCE_BYTES);
    SealedPayload {
        nonce: nonce.to_vec(),
        ciphertext: ciphertext.to_vec(),
    }
}

#[test]
fn a_restored_backup_cannot_read_what_the_erasure_destroyed_the_key_for() {
    let root = tempfile::tempdir().expect("temp dir");
    let (journal, keys, core) = journal_with_keys(root.path());

    let private = sealed_contribution();
    core.submit(&private, None).expect("accepted and sealed");

    // The backup. Taken before the erasure, of the database and nothing else — which is the
    // separation the whole guarantee rests on.
    let backup = root.path().join("backup/journal.sqlite3");
    copy_journal(&journal, &backup);

    // What the backup holds is ciphertext, not the words.
    let carried = stored_payload(&backup, &private.message_id);
    assert!(
        !carried.is_empty(),
        "the backup holds the row, or this test is about nothing"
    );
    assert!(
        !String::from_utf8_lossy(&carried).contains("asked to be forgotten"),
        "a sealed payload reached the database in the clear"
    );

    // Now the person asks to be forgotten.
    core.request_erasure(&private.message_id, ErasureReason::UserRequested)
        .expect("the erasure runs");

    // The key is gone from the store, which is the only place it ever was.
    let store = KeyStore::open(&keys).expect("the store reopens");
    assert!(
        !store.has_key_for(&private.message_id),
        "the erasure left the key behind, so nothing about a backup is guaranteed"
    );

    // And so the copy nobody controlled cannot be read, however intact it is.
    let (kek, _) = store.master().expect("a master key");
    assert!(
        store.key_for(&private.message_id, &kek).is_none(),
        "a destroyed key was recovered from the store"
    );

    // Stated the other way, which is the way ADR-0028 states it: the ciphertext survives the
    // erasure entirely, and is unreadable because the key does not.
    let still_there = stored_payload(&backup, &private.message_id);
    assert_eq!(
        still_there, carried,
        "the backup is untouched by an erasure that ran elsewhere, as any real backup would be"
    );
    assert!(
        Seal::unseal(&as_sealed(&still_there), &[0u8; 32]).is_err(),
        "sealed bytes opened with a key that is not the key"
    );
}

#[test]
fn a_backup_that_also_took_the_keys_is_outside_the_guarantee() {
    // The precondition, held as a fact rather than left to be discovered. `cybou-eventd` puts the
    // key store beside the journal by default, so `tar czf backup.tgz ~/.local/share/cybou/`
    // captures both — and a restore of that reads everything the erasure was meant to reach.
    //
    // Nothing in the crypto can prevent this, and pretending otherwise is worse than saying it: a
    // guarantee whose precondition is untested is one that will be reported as holding on the day
    // it does not.
    let root = tempfile::tempdir().expect("temp dir");
    let (journal, keys, core) = journal_with_keys(root.path());

    let private = sealed_contribution();
    core.submit(&private, None).expect("accepted and sealed");

    // A backup of everything, the way a person backing up a directory would take it.
    let backup_keys = root.path().join("backup-everything/secrets");
    fs::create_dir_all(&backup_keys).expect("backup dir");
    for entry in fs::read_dir(&keys).expect("the key store is readable") {
        let entry = entry.expect("an entry");
        if entry.path().is_file() {
            fs::copy(entry.path(), backup_keys.join(entry.file_name())).expect("copied");
        }
    }
    let backup_journal = root.path().join("backup-everything/journal.sqlite3");
    copy_journal(&journal, &backup_journal);

    core.request_erasure(&private.message_id, ErasureReason::UserRequested)
        .expect("the erasure runs");

    // The live store forgot it.
    let live = KeyStore::open(&keys).expect("the store reopens");
    assert!(!live.has_key_for(&private.message_id));

    // The copy did not, and that is the whole point of saying which rotations are inside the
    // guarantee rather than asserting that erasure reaches everywhere.
    let copied = KeyStore::open(&backup_keys).expect("the copied store opens");
    assert!(
        copied.has_key_for(&private.message_id),
        "this test is meant to demonstrate the hole, and found none"
    );

    let (kek, _) = copied.master().expect("a master key");
    let key = copied
        .key_for(&private.message_id, &kek)
        .expect("the key is in the copy");
    let carried = stored_payload(&backup_journal, &private.message_id);
    let read_back = Seal::unseal(&as_sealed(&carried), &key).expect("the copy decrypts");
    assert_eq!(
        read_back, private.payload,
        "a backup holding the keys reads exactly what the erasure destroyed"
    );
}
