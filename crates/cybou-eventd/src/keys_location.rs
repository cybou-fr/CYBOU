// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Where the key store goes, and why it is no longer beside the Journal.
//!
//! ADR-0028's erasure guarantee is that destroying a data key makes a record unreadable in every
//! copy of the database. It holds only in copies that do not also hold the key, and the default
//! layout put the store in `keys/` next to `journal.sqlite3` — so `tar czf backup.tgz
//! ~/.local/share/cybou/` captured both, and a restore of it read exactly what the erasure was
//! meant to reach. A test in `tests/restored_backup.rs` demonstrates that rather than warning
//! about it.
//!
//! ## Why this is a choice and not a migration
//!
//! Moving an existing store would leave a deployment unable to unwrap yesterday's keys, and this
//! organ has already learned once what that costs: a period where every restart wrapped new keys
//! with a secret that could not open the old ones, and yesterday's sealed payloads became
//! unreadable with nothing recording that anything had been erased.
//!
//! So nothing moves. A store that already exists keeps being used, wherever it is. Only an
//! installation with no store at all gets the separated location, and it gets it before there is
//! anything to lose.
//!
//! ```text
//! CYBOU_KEYSTORE_PATH set        -> exactly that, always
//! a store already beside the DB  -> that one, and say what it costs
//! nothing anywhere               -> the separated path
//! ```

use std::path::{Path, PathBuf};

/// Where a key store is, or should be created.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeysLocation {
    /// The directory to open.
    pub path: PathBuf,
    /// Why this one.
    pub because: Chose,
}

/// Why a location was chosen.
///
/// Carried rather than inferred from the path, because the interesting case — an old store that is
/// staying where it is — looks identical to a fresh one created there, and only one of them is
/// something an operator should be told about.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Chose {
    /// An operator set `CYBOU_KEYSTORE_PATH`.
    Declared,
    /// A store already exists beside the Journal and is still being used.
    ///
    /// Nothing is moved. Whether this deployment's backups are inside the erasure guarantee is a
    /// question about the backups, and the answer is no by default.
    ExistingBesideTheJournal,
    /// A store already exists at the separated path.
    ExistingSeparated,
    /// Nothing existed anywhere, so the separated path is used.
    FreshAndSeparated,
}

impl Chose {
    /// Whether one backup of the Journal's directory would also capture the keys.
    #[must_use]
    pub const fn one_backup_takes_both(self) -> bool {
        matches!(self, Self::ExistingBesideTheJournal)
    }
}

/// The directory name a key store lives in, under either root.
const STORE_DIRECTORY: &str = "keys";

/// Decide where the key store is.
///
/// `declared` is `CYBOU_KEYSTORE_PATH` if it was set. `separated_root` is the state directory a
/// fresh installation should use — somewhere a backup of the Journal's directory does not reach.
/// `exists` answers whether a directory already holds a store; it is passed in so this decision can
/// be tested without a filesystem, which is the only way it gets tested at all.
#[must_use]
pub fn decide(
    declared: Option<PathBuf>,
    journal_path: &Path,
    separated_root: &Path,
    exists: impl Fn(&Path) -> bool,
) -> KeysLocation {
    if let Some(path) = declared {
        return KeysLocation {
            path,
            because: Chose::Declared,
        };
    }

    let separated = separated_root.join(STORE_DIRECTORY);
    let beside = journal_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(STORE_DIRECTORY);

    // The old location first. A deployment that has one there must keep using it, and must keep
    // using it even if a separated one somehow also exists — two stores is a situation to report,
    // not to resolve by preferring the one with no keys in it.
    if exists(&beside) {
        return KeysLocation {
            path: beside,
            because: Chose::ExistingBesideTheJournal,
        };
    }
    if exists(&separated) {
        return KeysLocation {
            path: separated,
            because: Chose::ExistingSeparated,
        };
    }
    KeysLocation {
        path: separated,
        because: Chose::FreshAndSeparated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn journal() -> PathBuf {
        PathBuf::from("/var/lib/cybou/data/journal.sqlite3")
    }

    fn state() -> PathBuf {
        PathBuf::from("/var/lib/cybou/state")
    }

    fn nothing_exists(_: &Path) -> bool {
        false
    }

    #[test]
    fn a_fresh_installation_keeps_its_keys_out_of_the_journal_directory() {
        // The whole point. A new deployment gets the separated layout before there is anything to
        // lose by having it.
        let chosen = decide(None, &journal(), &state(), nothing_exists);
        assert_eq!(chosen.path, PathBuf::from("/var/lib/cybou/state/keys"));
        assert_eq!(chosen.because, Chose::FreshAndSeparated);
        assert!(!chosen.because.one_backup_takes_both());
        assert_ne!(
            chosen.path.parent(),
            journal().parent(),
            "a backup of the Journal directory would still take the keys"
        );
    }

    #[test]
    fn an_existing_store_beside_the_journal_is_not_moved() {
        // Moving it would leave the deployment unable to unwrap yesterday's keys — the failure this
        // organ already learned once, when every restart wrapped new keys with a secret that could
        // not open the old ones.
        let beside = PathBuf::from("/var/lib/cybou/data/keys");
        let chosen = decide(None, &journal(), &state(), |path| path == beside);
        assert_eq!(chosen.path, beside);
        assert_eq!(chosen.because, Chose::ExistingBesideTheJournal);
        assert!(
            chosen.because.one_backup_takes_both(),
            "an operator on the old layout must still be told what their backups hold"
        );
    }

    #[test]
    fn a_store_beside_the_journal_wins_over_one_that_also_exists_separated() {
        // Two stores is a situation to report, not to resolve by preferring the one that may hold
        // no keys. Choosing the empty one silently would seal tomorrow's payloads with a secret
        // that cannot open yesterday's.
        let chosen = decide(None, &journal(), &state(), |_| true);
        assert_eq!(chosen.path, PathBuf::from("/var/lib/cybou/data/keys"));
        assert_eq!(chosen.because, Chose::ExistingBesideTheJournal);
    }

    #[test]
    fn an_existing_separated_store_is_used_and_is_not_a_warning() {
        let separated = PathBuf::from("/var/lib/cybou/state/keys");
        let chosen = decide(None, &journal(), &state(), |path| path == separated);
        assert_eq!(chosen.path, separated);
        assert_eq!(chosen.because, Chose::ExistingSeparated);
        assert!(!chosen.because.one_backup_takes_both());
    }

    #[test]
    fn a_declared_path_is_taken_exactly_and_asks_no_questions() {
        // An operator who names a path has decided where their keys live, including deciding to put
        // them somewhere this build would not have.
        let declared = PathBuf::from("/mnt/hsm/cybou");
        let chosen = decide(Some(declared.clone()), &journal(), &state(), |_| {
            panic!("a declared path must not be second-guessed against the filesystem")
        });
        assert_eq!(chosen.path, declared);
        assert_eq!(chosen.because, Chose::Declared);
    }

    #[test]
    fn a_journal_at_the_filesystem_root_still_produces_a_path() {
        // `parent()` is None for a bare name, and a key store at `keys` relative to whatever
        // directory systemd started the process in is exactly the class of accident the watchlist
        // parser refuses paths for.
        let chosen = decide(None, Path::new("journal.sqlite3"), &state(), |_| false);
        assert_eq!(chosen.path, PathBuf::from("/var/lib/cybou/state/keys"));
    }
}
