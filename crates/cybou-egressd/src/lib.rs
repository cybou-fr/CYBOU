// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The one way out of a capsule (ADR-0042, step ten).
//!
//! A capsule has a network namespace of its own with loopback and no route, and that is not a
//! restriction to be relaxed later — it is the shape the rest of this depends on. What a
//! [`cybou_capsule::grant::NetworkGrant`] permits arrives through here instead: a broker the capsule
//! must ask, which speaks `CONNECT`, decides on the **name**, and does the resolving itself.
//!
//! ## Why not a firewall
//!
//! Because a grant is a DNS identity and a firewall works in addresses, and turning one into the
//! other means owning a policy for how long a resolution is good for, what happens when it changes
//! under you, and what a name means when it answers differently to every caller. Every one of those
//! is a place where being wrong is silent — the rule still loads, the counters still increment, and
//! the capsule reaches somewhere nobody granted.
//!
//! Here there is one resolution, it happens after the decision, and the capsule never performs or
//! supplies one. There is no window between checking a name and using it because there is nothing
//! between them.
//!
//! ## Two checks, and only one of them is about the grant
//!
//! The name is checked against the grant. The addresses it resolved to are checked against a much
//! smaller list of places no grant means — loopback, and link-local, which is where every cloud host
//! serves its own credentials to anything that asks. A name is controlled by whoever runs it, and a
//! broker that checked the name correctly and then connected to `169.254.169.254` would have done
//! everything right and handed over the machine.
//!
//! ## What this deliberately is not
//!
//! It is not a proxy. It does not read the capsule's payload, follow its redirects, or terminate its
//! TLS. After the decision it copies bytes it does not interpret, in both directions, until one side
//! stops — so the capsule's traffic is between the capsule and whatever it was granted, and the
//! broker is not a place where that traffic could be read.

pub mod ask;
pub mod decide;

pub use ask::{Asked, NotUnderstood, read};
pub use decide::{Egress, THE_GRANTED_PORT, decide, may_be_connected_to};
