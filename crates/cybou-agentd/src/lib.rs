// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The owner of one agent session.
//!
//! Everything an agent session needs already existed and nothing owned it. A capsule could be
//! compiled and started, a lease could be minted, a model gateway could be run, an agent pack could
//! be installed — and each of those was reached by a different caller, which is how the same launch
//! came to be described twice with different bounds. This crate is the single owner: one selection
//! becomes one lease, and every runtime piece is derived from that one object.
//!
//! ## It owns the session; it does not enforce it
//!
//! The distinction matters more here than anywhere else in this repository, because a coordinator is
//! exactly the shape of component that quietly becomes a boundary.
//!
//! ```text
//! what ends the capsule    the kernel, through RuntimeMaxSec on its unit
//! what ends the model      the lease clock, checked at the gateway on every request
//! what this crate does     writes it down, starts it, tears down what is left
//! ```
//!
//! If this process dies mid-session, the capsule still ends at its deadline and the gateway still
//! refuses once the lease is over. That is deliberate and it is the reason the session state here is
//! a *report* rather than a permission. `crate::session` cannot be consulted to find out whether
//! something is allowed, and nothing in this crate is asked before an agent acts.
//!
//! ## Two halves, on purpose
//!
//! [`plan`] is pure: given one launch it produces every path, unit name and file body that launch
//! implies, and the ordered teardown that undoes it. [`runtime`] is pure: it produces the exact
//! commands that carry a plan out, as data. [`session`] is pure: it tracks what has happened to a
//! session and refuses transitions that would misreport it. The binary does the part that touches a
//! filesystem and a service manager, and it does nothing these modules have not already described —
//! so what a session did is answerable by reading rather than by running.

pub mod capacity;
pub mod discovery;
pub mod plan;
pub mod profiles;
pub mod registry;
pub mod runtime;

#[cfg(target_os = "linux")]
pub mod service;
pub mod session;
pub mod view;

pub use capacity::{HostCapacity, NotAdmitted, Reserved, admits};
pub use discovery::{CannotRead, LaunchFiles, read_launch, read_lease, read_session};
pub use plan::{CannotPlan, Ceilings, Launch, SessionPlan, TeardownStep};
pub use profiles::{CannotOffer, OfferedProfile, ProfileCatalogue, Wanted};
pub use registry::{Found, LiveSession, Recovered, SessionRegistry, recover};
pub use runtime::HostPrograms;
pub use session::{Session, SessionEnd, SessionState};
pub use view::{Ledger, SessionView, SpendView, Standing};
