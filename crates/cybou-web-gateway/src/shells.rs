// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! One shell per session, rather than one shell per process.
//!
//! A shell has state — where it is standing. Holding a single [`ShellEngine`] for the whole
//! gateway made that state shared: two people signed into two accounts issued `cd` into the same
//! variable, and each of them saw the other move. Nothing leaked that the sandbox would not have
//! handed over anyway, because the jail root is the same either way, but a working directory that
//! answers to somebody else is a statement about who is at the keyboard, and it was false.
//!
//! What is isolated here is the shell's own state. The sandbox root is deliberately still shared:
//! ADR-0040 bounds the Body to read-only builtins over one demonstration root, and giving each
//! session a private root would be a different capability, not a fix to this one.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
};

use cybou_jailfs::JailFs;
use cybou_shelld::ShellEngine;
use time::{Duration, OffsetDateTime};

use crate::access::{SESSION_LIFETIME, SessionDigest};

/// How long an untouched shell is kept before it is forgotten.
///
/// A session's shell cannot outlive the session that owns it, and the desktop's shell is idle
/// whenever nobody is typing. Matching [`SESSION_LIFETIME`] means the two expire together rather
/// than the registry holding a working directory for a session that no longer exists.
pub const SHELL_IDLE_LIFETIME: Duration = SESSION_LIFETIME;

/// Where the shell surface is sandboxed, unless the deployment says otherwise.
///
/// Resolved in one place so the router and anything reasoning about the sandbox agree by
/// construction rather than by two copies of the same rule staying in step.
#[must_use]
pub fn sandbox_root() -> std::path::PathBuf {
    if let Ok(configured) = std::env::var("CYBOU_SHELL_JAIL") {
        return std::path::PathBuf::from(configured);
    }
    let demo = std::path::Path::new("/home/demo");
    if demo.exists() {
        return demo.to_path_buf();
    }
    // Named by process rather than at random: a gateway restarted in place should find the
    // sandbox it was using, and a second gateway on the same host must not land in it.
    std::env::temp_dir().join(format!("cybou_sandbox_{}", std::process::id()))
}

/// Whose shell this is.
///
/// Sessions are named the same way [`Sessions`](crate::access::Sessions) names them — by the
/// digest of the token, never the token — so a shell registry is not a second place in the process
/// holding the value that grants access. The desktop has no token and is one seat by definition,
/// so it gets a name of its own rather than sharing whatever session happened to be first.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ShellOwner {
    /// A browser session, named by the digest of its token.
    Session(SessionDigest),
    /// The local desktop, which is one seat and carries no session token.
    LocalDesktop,
}

/// One owner's shell, and when it was last used.
struct Held {
    engine: Arc<tokio::sync::Mutex<ShellEngine>>,
    last_used: OffsetDateTime,
}

/// The shells this process is currently holding.
///
/// In memory, like the sessions they belong to. A working directory is not something to persist:
/// it means nothing after the process that was standing in it is gone.
pub struct Shells {
    jail: JailFs,
    held: Mutex<HashMap<ShellOwner, Held>>,
}

impl Shells {
    /// A registry handing out shells rooted at one sandbox.
    #[must_use]
    pub fn new(jail: JailFs) -> Self {
        Self {
            jail,
            held: Mutex::new(HashMap::new()),
        }
    }

    /// This owner's shell, creating it if this is their first command.
    ///
    /// Returned as an `Arc` so the caller locks one shell rather than the registry: a long command
    /// in one session must not stop another session from running a short one.
    pub fn for_owner(
        &self,
        owner: &ShellOwner,
        now: OffsetDateTime,
    ) -> Arc<tokio::sync::Mutex<ShellEngine>> {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        // Idle shells are dropped whenever anything is written, so a gateway that has served many
        // sessions does not keep a working directory for each of them.
        held.retain(|_, entry| entry.last_used + SHELL_IDLE_LIFETIME > now);
        if let Some(entry) = held.get_mut(owner) {
            entry.last_used = now;
            return Arc::clone(&entry.engine);
        }
        let engine = Arc::new(tokio::sync::Mutex::new(ShellEngine::new(self.jail.clone())));
        held.insert(
            owner.clone(),
            Held {
                engine: Arc::clone(&engine),
                last_used: now,
            },
        );
        engine
    }

    /// Forget this owner's shell, because the session that owned it has ended.
    pub fn end(&self, owner: &ShellOwner) {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        held.remove(owner);
    }

    /// How many shells are currently held.
    #[must_use]
    pub fn held_shells(&self) -> usize {
        self.held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> (Shells, tempfile::TempDir) {
        let root = tempfile::tempdir().expect("a sandbox root");
        let jail = JailFs::new(root.path()).expect("a jail");
        (Shells::new(jail), root)
    }

    fn session(token: &str) -> ShellOwner {
        ShellOwner::Session(crate::access::digest(token))
    }

    #[tokio::test]
    async fn one_session_does_not_move_another_sessions_shell() {
        // The whole point of the split: `cd` is a statement about where one person is standing.
        let (shells, root) = registry();
        std::fs::create_dir(root.path().join("somewhere")).expect("a directory to enter");
        let now = OffsetDateTime::now_utc();

        let alice = shells.for_owner(&session("alice-token"), now);
        // The output is not what is under test; where the shell now stands is.
        drop(alice.lock().await.execute("cd somewhere"));
        assert_eq!(alice.lock().await.cwd(), "/somewhere");

        let bob = shells.for_owner(&session("bob-token"), now);
        assert_eq!(bob.lock().await.cwd(), "/");
    }

    #[tokio::test]
    async fn the_same_session_keeps_standing_where_it_was() {
        let (shells, root) = registry();
        std::fs::create_dir(root.path().join("somewhere")).expect("a directory to enter");
        let now = OffsetDateTime::now_utc();

        // The output is not what is under test; where the shell now stands is.
        drop(
            shells
                .for_owner(&session("alice-token"), now)
                .lock()
                .await
                .execute("cd somewhere"),
        );
        let again = shells.for_owner(&session("alice-token"), now);
        assert_eq!(again.lock().await.cwd(), "/somewhere");
    }

    #[tokio::test]
    async fn the_desktop_is_not_whichever_session_arrived_first() {
        let (shells, root) = registry();
        std::fs::create_dir(root.path().join("somewhere")).expect("a directory to enter");
        let now = OffsetDateTime::now_utc();

        // The output is not what is under test; where the shell now stands is.
        drop(
            shells
                .for_owner(&session("alice-token"), now)
                .lock()
                .await
                .execute("cd somewhere"),
        );
        let desktop = shells.for_owner(&ShellOwner::LocalDesktop, now);
        assert_eq!(desktop.lock().await.cwd(), "/");
    }

    #[tokio::test]
    async fn ending_a_session_forgets_where_it_was_standing() {
        // A token can be reissued. A shell that survived its session would hand the next holder of
        // that name a working directory somebody else chose.
        let (shells, root) = registry();
        std::fs::create_dir(root.path().join("somewhere")).expect("a directory to enter");
        let now = OffsetDateTime::now_utc();

        // The output is not what is under test; where the shell now stands is.
        drop(
            shells
                .for_owner(&session("alice-token"), now)
                .lock()
                .await
                .execute("cd somewhere"),
        );
        shells.end(&session("alice-token"));

        let fresh = shells.for_owner(&session("alice-token"), now);
        assert_eq!(fresh.lock().await.cwd(), "/");
        assert_eq!(shells.held_shells(), 1);
    }

    #[tokio::test]
    async fn an_idle_shell_does_not_outlive_the_session_it_belongs_to() {
        let (shells, _root) = registry();
        let now = OffsetDateTime::now_utc();
        shells.for_owner(&session("alice-token"), now);
        assert_eq!(shells.held_shells(), 1);

        let much_later = now + SHELL_IDLE_LIFETIME + Duration::seconds(1);
        shells.for_owner(&session("bob-token"), much_later);
        assert_eq!(shells.held_shells(), 1);
    }
}
