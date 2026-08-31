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

use time::Duration;

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
    // Nothing else is guessed. This used to prefer `/home/demo` whenever that directory existed,
    // which on the deployed host it does — owned by somebody else, unreadable by this service. The
    // Shell answered every `ls` with an I/O error and the File Manager returned 502, because the
    // sandbox had been chosen by what happened to be on disk rather than by anyone.
    //
    // Named by process rather than at random: a gateway restarted in place should find the sandbox
    // it was using, and a second gateway on the same host must not land in it.
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
    /// One shell belonging to a browser session, named by the digest of its token.
    Session {
        /// Which session holds it.
        session: SessionDigest,
        /// Which of that session's shells it is.
        instance: u32,
    },
    /// One shell belonging to the local desktop seat, which carries no session token.
    LocalDesktop {
        /// Which of the seat's shells it is.
        instance: u32,
    },
}

impl ShellOwner {
    /// Whether this owner belongs to the same seat as another, whatever instance either names.
    ///
    /// Ending a session ends every shell it opened, not the one that happened to be numbered zero.
    #[must_use]
    pub fn same_seat_as(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Session { session: a, .. }, Self::Session { session: b, .. }) => a == b,
            (Self::LocalDesktop { .. }, Self::LocalDesktop { .. }) => true,
            _ => false,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_local_desktop_seat_is_not_a_network_session() {
        // The distinction the gateway makes everywhere it asks who is asking. It used to live
        // beside a sandboxed command engine, which is gone; the seat is what it was really for.
        let local = ShellOwner::LocalDesktop { instance: 0 };
        let session = ShellOwner::Session {
            session: crate::access::digest("token"),
            instance: 0,
        };
        assert!(!local.same_seat_as(&session));
        assert!(local.same_seat_as(&ShellOwner::LocalDesktop { instance: 3 }));
    }
}
