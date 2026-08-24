// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Watching what a host is actually doing, so Cybou can notice something is wrong (ADR-0041 S7).
//!
//! Perception records what is stable about a machine and stops there on purpose, so until now
//! nothing observed the host between one restart and the next. Cybou could say what it knew and not
//! what was happening — which makes the S0 gate unreachable, because a system cannot detect a
//! problem it never saw, and every stage after *detect* has nothing to work on.
//!
//! The whole design is one line: **telemetry is not biography.** A Journal accumulating a CPU
//! sample every second would be a telemetry database wearing a life story, and every rule that
//! makes the Journal worth having — erasure, retention, dependency closure, provenance — would be
//! applied to numbers that individually mean nothing and cost something to keep forever.
//!
//! So this organ holds a bounded transient window and concludes from it. A `Reading` has no path
//! into the Journal anywhere in this tree: it is transient by construction rather than by policy.
//! A `SystemInsight` does, and it is a hypothesis with its readings attached — *the machine is
//! under memory pressure, here is what I saw, since when, and how sure I am*.
//!
//! Nothing here needs an accelerator, a model, or a network, which is the point: the detector has
//! to work on the machine and in the situation where a person most needs it, and that is a small
//! instance with the network as the thing under investigation.

pub mod baseline;
pub mod core;
pub mod probe;
pub mod series;
pub mod trend;
pub mod watchlist;

#[cfg(target_os = "linux")]
pub mod service;

pub use core::{STALE_AFTER, TelemetryCore};
pub use series::Series;
