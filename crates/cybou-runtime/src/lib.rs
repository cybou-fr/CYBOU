// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Stable XDG state locations and fail-closed predecessor migration.

use std::{
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use thiserror::Error;

/// State-path or legacy-migration failure.
#[derive(Debug, Error)]
pub enum StateError {
    /// No absolute home exists from which to derive an XDG fallback.
    #[error("an absolute HOME is required")]
    MissingHome,
    /// Runtime state must never fall back to a persistent directory.
    #[error("an absolute XDG_RUNTIME_DIR is required")]
    MissingRuntime,
    /// Migration accepts only resolved absolute roots.
    #[error("state root must be absolute: {0}")]
    RelativeRoot(PathBuf),
    /// Source and destination contain the same entry name.
    #[error("state migration collision for {0}")]
    Collision(String),
    /// A filesystem operation failed before migration completed.
    #[error("state migration I/O failure: {0}")]
    Io(#[from] io::Error),
    /// A move failed and at least one preceding move could not be rolled back.
    #[error("state migration failed and rollback was incomplete")]
    IncompleteRollback,
}

fn absolute(value: Option<&OsStr>) -> Option<PathBuf> {
    value.map(PathBuf::from).filter(|path| path.is_absolute())
}

fn home_fallback(home: Option<&OsStr>, suffix: &str) -> Result<PathBuf, StateError> {
    absolute(home)
        .map(|path| path.join(suffix).join("cybou"))
        .ok_or(StateError::MissingHome)
}

/// Resolve `$XDG_STATE_HOME/cybou`, falling back to `$HOME/.local/state/cybou`.
///
/// # Errors
///
/// Returns [`StateError::MissingHome`] when neither input is an absolute path.
pub fn persistent_root_from(
    home: Option<&OsStr>,
    xdg_state_home: Option<&OsStr>,
) -> Result<PathBuf, StateError> {
    absolute(xdg_state_home)
        .map(|path| path.join("cybou"))
        .map_or_else(|| home_fallback(home, ".local/state"), Ok)
}

/// Resolve `$XDG_CACHE_HOME/cybou`, falling back to `$HOME/.cache/cybou`.
///
/// # Errors
///
/// Returns [`StateError::MissingHome`] when neither input is an absolute path.
pub fn cache_root_from(
    home: Option<&OsStr>,
    xdg_cache_home: Option<&OsStr>,
) -> Result<PathBuf, StateError> {
    absolute(xdg_cache_home)
        .map(|path| path.join("cybou"))
        .map_or_else(|| home_fallback(home, ".cache"), Ok)
}

/// Resolve `$XDG_RUNTIME_DIR/cybou` without a persistent fallback.
///
/// # Errors
///
/// Returns [`StateError::MissingRuntime`] unless the input is absolute.
pub fn runtime_root_from(xdg_runtime_dir: Option<&OsStr>) -> Result<PathBuf, StateError> {
    absolute(xdg_runtime_dir)
        .map(|path| path.join("cybou"))
        .ok_or(StateError::MissingRuntime)
}

/// Move every predecessor entry into the canonical root without overwriting any destination.
///
/// All collisions are detected before the first move. A later move failure rolls preceding moves
/// back in reverse order.
///
/// # Errors
///
/// Returns a typed collision, I/O, relative-root, or incomplete-rollback failure.
pub fn migrate_legacy(legacy_root: &Path, persistent_root: &Path) -> Result<(), StateError> {
    for root in [legacy_root, persistent_root] {
        if !root.is_absolute() {
            return Err(StateError::RelativeRoot(root.to_path_buf()));
        }
    }
    if legacy_root == persistent_root || !legacy_root.exists() {
        return Ok(());
    }

    let mut entries = fs::read_dir(legacy_root)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);
    if entries.is_empty() {
        fs::remove_dir(legacy_root)?;
        return Ok(());
    }
    fs::create_dir_all(persistent_root)?;
    for entry in &entries {
        if persistent_root.join(entry.file_name()).exists() {
            return Err(StateError::Collision(
                entry.file_name().to_string_lossy().into_owned(),
            ));
        }
    }

    let mut moved = Vec::new();
    for entry in entries {
        let name = entry.file_name();
        let destination = persistent_root.join(&name);
        if let Err(error) = fs::rename(entry.path(), &destination) {
            let mut rollback_ok = true;
            for moved_name in moved.iter().rev() {
                if fs::rename(
                    persistent_root.join(moved_name),
                    legacy_root.join(moved_name),
                )
                .is_err()
                {
                    rollback_ok = false;
                }
            }
            return if rollback_ok {
                Err(StateError::Io(error))
            } else {
                Err(StateError::IncompleteRollback)
            };
        }
        moved.push(PathBuf::from(name));
    }
    fs::remove_dir(legacy_root)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, fs};

    use tempfile::tempdir;

    use super::{
        StateError, cache_root_from, migrate_legacy, persistent_root_from, runtime_root_from,
    };

    #[test]
    fn xdg_roots_match_the_predecessor_contract() {
        assert_eq!(
            persistent_root_from(Some(OsStr::new("/home/alice")), Some(OsStr::new("/state")))
                .expect("state"),
            std::path::Path::new("/state/cybou")
        );
        assert_eq!(
            persistent_root_from(
                Some(OsStr::new("/home/alice")),
                Some(OsStr::new("relative"))
            )
            .expect("fallback"),
            std::path::Path::new("/home/alice/.local/state/cybou")
        );
        assert_eq!(
            cache_root_from(Some(OsStr::new("/home/alice")), None).expect("cache"),
            std::path::Path::new("/home/alice/.cache/cybou")
        );
        assert_eq!(
            runtime_root_from(Some(OsStr::new("/run/user/1000"))).expect("runtime"),
            std::path::Path::new("/run/user/1000/cybou")
        );
        assert!(matches!(
            runtime_root_from(Some(OsStr::new("relative"))),
            Err(StateError::MissingRuntime)
        ));
    }

    #[test]
    fn migration_preserves_unrelated_target_entries() {
        let root = tempdir().expect("temp root");
        let legacy = root.path().join("legacy");
        let target = root.path().join("state");
        fs::create_dir_all(&legacy).expect("legacy");
        fs::create_dir_all(&target).expect("target");
        fs::write(legacy.join("journal.db"), b"journal").expect("legacy state");
        fs::write(target.join("desktop-layout-version"), b"2\n").expect("unrelated state");

        migrate_legacy(&legacy, &target).expect("migration");
        assert_eq!(
            fs::read(target.join("journal.db")).expect("journal"),
            b"journal"
        );
        assert_eq!(
            fs::read(target.join("desktop-layout-version")).expect("layout"),
            b"2\n"
        );
        assert!(!legacy.exists());
    }

    #[test]
    fn collision_fails_before_moving_any_entry() {
        let root = tempdir().expect("temp root");
        let legacy = root.path().join("legacy");
        let target = root.path().join("state");
        fs::create_dir_all(&legacy).expect("legacy");
        fs::create_dir_all(&target).expect("target");
        fs::write(legacy.join("a-first"), b"untouched").expect("first");
        fs::write(legacy.join("journal.db"), b"old").expect("old");
        fs::write(target.join("journal.db"), b"new").expect("new");

        assert!(matches!(
            migrate_legacy(&legacy, &target),
            Err(StateError::Collision(_))
        ));
        assert_eq!(
            fs::read(legacy.join("a-first")).expect("not moved"),
            b"untouched"
        );
        assert_eq!(
            fs::read(legacy.join("journal.db")).expect("old remains"),
            b"old"
        );
        assert_eq!(
            fs::read(target.join("journal.db")).expect("new remains"),
            b"new"
        );
    }
}
