// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Offering to do something about what the host found, and refusing to do it (ADR-0022).
//!
//! The vertical this belongs to is `observe -> understand -> remember -> diagnose -> explain ->
//! propose -> authorize -> act -> observe outcome`. Everything up to *explain* exists. This crate is
//! *propose* and *authorize*, and it deliberately stops there: **nothing here executes anything.**
//!
//! There is an executor, and `cybou-remediationd` drives it: it asks [`initiative`] whether this host
//! may act, proposes here, hands the permit on, waits out [`TOO_SOON_AFTER`], and reports what the
//! host saw afterwards. Until it existed the only thing that had ever run the loop was an example
//! written for a gate, so a host left to itself reached *explain* and stopped.
//!
//! Nothing about that changes what this crate is. It still proposes and authorizes and executes
//! nothing, and the driver still cannot authorize itself — every proposal it makes comes through
//! [`authorize`] and the operator's standing policy, so on a host where nobody permitted anything it
//! proposes and is refused.
//!
//! That is the point of building it now. The natural shape of this code — build a proposal, hand it
//! to an executor — has no place for the decision to live, so the decision gets made by whoever
//! wired the two together, on a working system, under pressure to make it work. Written first, the
//! executor arrives to find the gate already closed.
//!
//! Three things are structural rather than promised:
//!
//! - **A proposer cannot choose its own risk.** Risk and reversibility are functions of the
//!   [`operation::Operation`], and the operation set is closed. Something arguing for its own
//!   proposal is the wrong party to assess it.
//! - **Nothing is granted on an unconfigured machine.** `Granted` is reachable only through a
//!   standing policy a person set, and the default policy grants nothing.
//! - **A policy cannot grant what the operation table forbids.** Otherwise the forbidden list is
//!   advisory, which is the same as absent.
//! - **An action does not get to say whether it worked.** [`outcome`] concludes that from findings
//!   taken before and after by the telemetry organ, which did not carry the action out and has no
//!   notion that one was carried out. What the executor said is recorded beside it, as a claim, and
//!   the two disagreeing is a first-class value rather than something a reader must work out.

pub mod authorize;
pub mod initiative;
pub mod operation;
pub mod outcome;
pub mod propose;

pub use authorize::{StandingPolicy, authorize, criticise, criticise_request, permits_unattended};
pub use initiative::{Initiative, Settled, Tried, initiative, wait_for};
pub use operation::{ALL_OPERATIONS, Operation};
pub use outcome::{Reobservation, TOO_SOON_AFTER, observe_outcome};
pub use propose::{propose, remedies_for};
