// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What an agent is granted, and where its autonomy ends (ADR-0042).
//!
//! Written before anything that can create a capsule, for the same reason the authorization gate was
//! written before the executor and the outcome layer before both. The natural order is to build the
//! sandbox and then decide what may happen inside it, and code written that way arrives with the
//! decision already dissolved into whichever call site needed it. Written first, the sandbox arrives
//! to find the answer is not its to give.
//!
//! **Nothing here can create, enter, or enforce anything.** This crate decides; it depends on the
//! protocol and on no runtime. `scripts/validate-organ-layering.py` fails if that changes.
//!
//! ## The one distinction the whole design rests on
//!
//! ```text
//! autonomous inside its capsule  ≠  autonomous on the host
//! ```
//!
//! A grant is given once, for a capsule. Inside it an agent reads, writes, compiles, runs tests,
//! installs dependencies, starts servers and calls its model without being asked anything. That is
//! not laxity; it is the only shape that works. A person asked to approve `npm install` for the
//! fifteenth time has stopped reading, and an interface that keeps asking has not made a weaker
//! promise — it has made the grant meaningless.
//!
//! ## Three answers, not two
//!
//! The interesting part is that *refused* and *needs a decision* are different, and collapsing them
//! is how a boundary becomes either a nuisance or a hole.
//!
//! ```text
//! Verdict::Allowed          inside the grant; nobody is asked
//! Verdict::CrossesBoundary  outside the capsule, and answerable — becomes an ActionProposal
//! Verdict::Refused          outside the capsule and never answerable, whatever anybody says
//! ```
//!
//! `Refused` is small on purpose. It holds the things no profile may grant because granting them
//! would end the capsule as a concept: reaching another capsule, reaching Cybou's own state,
//! reaching the key store. Everything else that is outside the capsule is a proposal, because a
//! person is entitled to say yes to it.
//!
//! ## What this deliberately does not do
//!
//! It does not enforce. A capsule holds because the kernel holds it — namespaces, cgroups, seccomp,
//! Landlock or `AppArmor`. This is the description of what was granted, used to decide what to ask
//! and what to record. A design in which this module were the enforcement would be a boundary made
//! of whatever the agent could be persuaded to report about itself.

pub mod backend;
pub mod compile;
pub mod end;
pub mod grant;
pub mod lease;
pub mod profile;
pub mod reach;
pub mod spec;
pub mod supervise;
pub mod verdict;

pub use backend::{BackendError, Bubblewrap, CapsuleBackend, CapsuleRuntimeBindings};
pub use compile::{CannotCompile, compile};
pub use end::end;
pub use grant::{CapsuleGrant, ModelGrant, NetworkGrant, ResourceBudget, Workspace};
pub use lease::{Ended, Lease, decide_under_lease};
pub use profile::{CannotIssueLease, CapabilityProfile, LeaseRequest, ProfileId, issue_lease};
pub use reach::Reach;
pub use spec::{KernelCapsuleSpec, ModelChannel};
pub use supervise::{under_budget, unit_name};
pub use verdict::{Refusal, Verdict, decide};
