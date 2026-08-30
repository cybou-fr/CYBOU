// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The wire between the gateway and one interactive terminal, and the bounds on it.
//!
//! [ADR-0047](../../../docs/adr/ADR-0047-interactive-terminal-under-the-authenticated-account.md)
//! decides that a person authenticated to the gateway may run programs as their own account. This
//! crate owns the half of that which runs as the person: it allocates the pseudoterminal, spawns
//! their login shell in it, and moves bytes.
//!
//! It decides nothing about who may ask. Authentication happened at the gateway, and the kernel
//! holds the boundary afterwards, because this process is that account and can do exactly what
//! that account can do. There is no filtering here and there is deliberately none: command
//! filtering on a real shell is theatre, and a filter that can be defeated is worse than an absent
//! one because it is believed.
//!
//! What is here instead is bounds. A terminal is the one surface where the far end produces bytes
//! faster than anything consumes them, and where a tab left open is a process left running.

use serde::{Deserialize, Serialize};

/// One connection, one pseudoterminal.
///
/// In the library rather than beside the binary's `main`, so a test can drive a real shell over a
/// socket pair. A module private to the binary could only be exercised by starting a process,
/// which tests the same thing through more moving parts.
#[cfg(target_os = "linux")]
pub mod session;

/// The most bytes one frame may carry, in either direction.
///
/// A paste is the large case in the input direction and a screen redraw in the output one; neither
/// is anywhere near this. What this stops is a peer declaring a length nobody intends to send.
pub const MAX_FRAME_BYTES: usize = 64 * 1024;

/// How many bytes of unread output are held before the session is closed.
///
/// A program can write faster than a browser reads — `yes`, a `find /` over a slow link, a build
/// log — and something has to give. Buffering without a bound gives the far end a way to spend
/// this host's memory from a tab. Dropping bytes silently gives a person a terminal that lies
/// about what it printed. So the session ends, and says that it ended for this reason.
pub const MAX_BACKLOG_BYTES: usize = 4 * 1024 * 1024;

/// How long a session with no input and no output is kept.
///
/// A terminal is idle whenever nobody is typing and nothing is printing, which for a working
/// session is most of the time — so this is long. What it collects is the tab that was closed
/// without the socket noticing.
pub const IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_hours(4);

/// The widest and tallest a terminal may be told it is.
///
/// A window size is two numbers from a browser that reach `TIOCSWINSZ`. Programs allocate from
/// them — a full-screen editor sizes buffers by rows times columns — so an unchecked pair is a way
/// to ask a program on this host to allocate whatever the tab felt like.
pub const MAX_COLUMNS: u16 = 1000;
/// The tallest a terminal may be told it is. See [`MAX_COLUMNS`].
pub const MAX_ROWS: u16 = 1000;

/// What the gateway sends.
///
/// Externally tagged, which is serde's default and is deliberate here rather than inherited. The
/// web contracts in this workspace carry an internal `tag` because they are read as JSON by a
/// browser; this is CBOR between two processes on one host, and an internally tagged enum cannot
/// hold a newtype variant wrapping a sequence at all — which is every frame that carries bytes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FromGateway {
    /// Start the session. Must be the first frame, and may not arrive twice.
    Open {
        /// Terminal width in columns.
        columns: u16,
        /// Terminal height in rows.
        rows: u16,
    },
    /// Keystrokes, on their way to the program.
    Input(Vec<u8>),
    /// The window changed size.
    Resize {
        /// Terminal width in columns.
        columns: u16,
        /// Terminal height in rows.
        rows: u16,
    },
}

/// What the owner sends back.
///
/// Externally tagged, for the reason given on [`FromGateway`].
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FromOwner {
    /// The pseudoterminal exists and the shell is running in it.
    Opened,
    /// Bytes from the program, on their way to a screen.
    Output(Vec<u8>),
    /// The program ended, and how.
    ///
    /// Both halves are optional because a process ends one way or the other and a reader should
    /// not have to read a zero as "no signal".
    Ended {
        /// Exit status, when it exited.
        code: Option<i32>,
        /// Terminating signal, when it was signalled.
        signal: Option<i32>,
    },
    /// The session will not continue, and this is why.
    Refused(Refusal),
}

/// Why a session stopped or never started.
///
/// Typed rather than a string, because a person is looking at a blank terminal and the difference
/// between *this host has no terminal for you* and *you were producing output faster than it could
/// be read* is the difference between asking an operator for access and scrolling back.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Refusal {
    /// A frame arrived that this stage of the session has no meaning for.
    ///
    /// The first frame must be `Open` and no later frame may be: a second `Open` on a live session
    /// is either a confused peer or an attempt to get a second shell out of one connection.
    OutOfOrder,
    /// A frame was longer than [`MAX_FRAME_BYTES`], or declared a length that was.
    FrameTooLarge,
    /// A window size outside [`MAX_COLUMNS`] by [`MAX_ROWS`], or zero in either direction.
    ImpossibleWindow,
    /// Unread output passed [`MAX_BACKLOG_BYTES`].
    ///
    /// Not a failure of the program, which is doing what it was asked. The session is what could
    /// not keep up.
    OutputOutranTheReader,
    /// Nothing was typed and nothing was printed for [`IDLE_TIMEOUT`].
    Idle,
    /// The pseudoterminal could not be allocated, or the shell could not be started.
    CouldNotStart,
}

impl Refusal {
    /// One line for the person looking at the terminal.
    #[must_use]
    pub const fn explain(self) -> &'static str {
        match self {
            Self::OutOfOrder => "the session received a frame it had no meaning for",
            Self::FrameTooLarge => "a frame was larger than this session accepts",
            Self::ImpossibleWindow => "the terminal was given a size it cannot have",
            Self::OutputOutranTheReader => {
                "output arrived faster than it could be read, and the session was closed rather \
                 than held"
            }
            Self::Idle => "the session was idle and was closed",
            Self::CouldNotStart => "no terminal could be started for this account",
        }
    }
}

/// Whether a window size is one a terminal may be told it has.
///
/// Zero is rejected in both directions rather than clamped up. A zero-column terminal is not a
/// small terminal; it is a browser that has not measured itself yet, and sizing a program's buffers
/// from it produces division by zero in software much older than this.
#[must_use]
pub const fn window_is_possible(columns: u16, rows: u16) -> bool {
    columns >= 1 && columns <= MAX_COLUMNS && rows >= 1 && rows <= MAX_ROWS
}

/// Where this account's terminal socket lives, under a runtime directory.
///
/// Named by numeric UID rather than by account name, the way the host filesystem owner already is:
/// a name is a lookup that can change answers, and the gateway knows the UID it authenticated.
#[must_use]
pub fn socket_path(runtime_directory: &std::path::Path, uid: u32) -> std::path::PathBuf {
    runtime_directory.join(uid.to_string()).join("owner.sock")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_terminal_cannot_be_given_a_size_it_cannot_have() {
        assert!(window_is_possible(80, 24));
        assert!(window_is_possible(1, 1));
        assert!(window_is_possible(MAX_COLUMNS, MAX_ROWS));

        // Not clamped. A browser that has not measured itself yet reports zero, and a program
        // sizing its buffers from a zero-column terminal divides by it.
        assert!(!window_is_possible(0, 24));
        assert!(!window_is_possible(80, 0));
        assert!(!window_is_possible(MAX_COLUMNS + 1, 24));
        assert!(!window_is_possible(80, MAX_ROWS + 1));
    }

    #[test]
    fn every_refusal_says_something_a_person_can_act_on() {
        for refusal in [
            Refusal::OutOfOrder,
            Refusal::FrameTooLarge,
            Refusal::ImpossibleWindow,
            Refusal::OutputOutranTheReader,
            Refusal::Idle,
            Refusal::CouldNotStart,
        ] {
            let explanation = refusal.explain();
            assert!(!explanation.is_empty());
            // A blank terminal with a message that names the enum variant is the terminal telling
            // a person about its own source code.
            assert!(
                !explanation.contains("Refusal"),
                "{refusal:?} explains itself by naming itself"
            );
        }
    }

    #[test]
    fn a_socket_is_named_by_the_uid_the_gateway_authenticated() {
        let path = socket_path(std::path::Path::new("/run/cybou-pty"), 1000);
        assert_eq!(path, std::path::Path::new("/run/cybou-pty/1000/owner.sock"));
    }

    #[test]
    fn frames_survive_the_wire_they_are_carried_on() {
        for frame in [
            FromGateway::Open {
                columns: 80,
                rows: 24,
            },
            // Deliberately not UTF-8. A terminal carries bytes: a keystroke is not text, and a
            // frame type that could only hold a `String` would drop exactly the input that matters
            // — arrow keys, Ctrl-C, a paste of a binary file somebody did not mean to paste.
            FromGateway::Input(vec![0x1b, b'[', b'A', 0x03, 0xff]),
            FromGateway::Resize {
                columns: 200,
                rows: 50,
            },
        ] {
            let mut encoded = Vec::new();
            ciborium::into_writer(&frame, &mut encoded).expect("encode");
            let decoded: FromGateway = ciborium::from_reader(encoded.as_slice()).expect("decode");
            assert_eq!(decoded, frame);
        }

        for frame in [
            FromOwner::Opened,
            FromOwner::Output(vec![0xff, 0xfe, 0x00]),
            FromOwner::Ended {
                code: Some(0),
                signal: None,
            },
            FromOwner::Ended {
                code: None,
                signal: Some(9),
            },
            FromOwner::Refused(Refusal::Idle),
        ] {
            let mut encoded = Vec::new();
            ciborium::into_writer(&frame, &mut encoded).expect("encode");
            let decoded: FromOwner = ciborium::from_reader(encoded.as_slice()).expect("decode");
            assert_eq!(decoded, frame);
        }
    }

    #[test]
    fn an_ending_says_which_kind_of_ending_it_was() {
        // Zero is a real exit status and nine is a real signal. A shape that carried one integer
        // would make "exited cleanly" and "was killed by SIGHUP" arithmetic on the same field.
        let exited = FromOwner::Ended {
            code: Some(0),
            signal: None,
        };
        let killed = FromOwner::Ended {
            code: None,
            signal: Some(9),
        };
        assert_ne!(exited, killed);
    }
}
