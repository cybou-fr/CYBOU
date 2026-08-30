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

/// One connection, one pseudoterminal.
///
/// In the library rather than beside the binary's `main`, so a test can drive a real shell over a
/// socket pair. A module private to the binary could only be exercised by starting a process,
/// which tests the same thing through more moving parts.
#[cfg(target_os = "linux")]
pub mod session;

pub use cybou_protocol::terminal::{
    FromGateway, FromOwner, IDLE_TIMEOUT, MAX_BACKLOG_BYTES, MAX_COLUMNS, MAX_FRAME_BYTES,
    MAX_ROWS, Refusal, socket_path, window_is_possible,
};
