// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Whether a capsule may reach what it asked for.
//!
//! Two questions, in this order, and the order is the design.
//!
//! First the name, against the grant. That is the whole reason this exists rather than an address
//! allow-list in a firewall: a grant says `github.com`, and turning that into addresses means owning
//! a policy for how long a resolution is good for, what to do when it changes, and what a name means
//! when it answers with a different address to every caller. Being wrong there is silent.
//!
//! Then the addresses the name resolved to, against a small list of places no grant means. That
//! second check is not about the grant at all — it is about `169.254.169.254` and `127.0.0.1`, which
//! are what a name has to resolve to for a capsule to reach the machine it is running on.

use std::net::IpAddr;

use cybou_capsule::grant::NetworkGrant;

use crate::ask::Asked;

/// What the broker decided.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Egress {
    /// The grant names this host.
    Permitted,
    /// It does not.
    NotGranted {
        /// What was asked for.
        host: String,
    },
    /// The grant names it, but not on this port.
    NotOnThatPort {
        /// What was asked for.
        port: u16,
    },
}

/// The only port a grant covers.
///
/// A grant names hosts and says nothing about ports, and the difference between "reach github.com"
/// and "reach anything listening anywhere on github.com" is the difference between an egress broker
/// and a tunnel. `443` because a grant to reach a host over the public internet means TLS; anything
/// else is a decision somebody should have to write down.
pub const THE_GRANTED_PORT: u16 = 443;

/// Decide on the name.
#[must_use]
pub fn decide(grant: &NetworkGrant, asked: &Asked) -> Egress {
    if !grant.permits(&asked.host) {
        return Egress::NotGranted {
            host: asked.host.clone(),
        };
    }
    if asked.port != THE_GRANTED_PORT {
        return Egress::NotOnThatPort { port: asked.port };
    }
    Egress::Permitted
}

/// Whether an address a granted name resolved to is one a capsule may be connected to.
///
/// Nothing to do with the grant. A name is a thing somebody else controls, and the addresses it
/// answers with include, on every cloud host in existence, `169.254.169.254` — where the machine's
/// own credentials are served to anything that asks. A capsule reaching that has reached the host,
/// through a broker that checked its name correctly and connected it anyway.
///
/// Loopback for the same reason: a name resolving to `127.0.0.1` reaches whatever the *host* is
/// running, which is Cybou.
///
/// Private ranges are permitted. An operator who grants an internal host name means it, and refusing
/// those would make the broker unusable on any network that has one — a rule people work around is a
/// rule that ends up switched off.
#[must_use]
pub fn may_be_connected_to(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            !address.is_loopback()
                && !address.is_link_local()
                && !address.is_unspecified()
                && !address.is_broadcast()
        }
        IpAddr::V6(address) => {
            // `is_unicast_link_local` is not stable, so the prefix is checked directly. fe80::/10.
            let link_local = (address.segments()[0] & 0xffc0) == 0xfe80;
            !address.is_loopback() && !address.is_unspecified() && !link_local
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asked(host: &str, port: u16) -> Asked {
        Asked {
            host: host.to_owned(),
            port,
        }
    }

    #[test]
    fn a_granted_host_on_the_granted_port_is_permitted() {
        let grant = NetworkGrant::to(&["github.com"]);
        assert_eq!(decide(&grant, &asked("github.com", 443)), Egress::Permitted);
    }

    #[test]
    fn a_host_nobody_granted_is_not_reachable_by_asking_nicely() {
        let grant = NetworkGrant::to(&["github.com"]);
        assert_eq!(
            decide(&grant, &asked("evil.example", 443)),
            Egress::NotGranted {
                host: "evil.example".to_owned()
            }
        );
    }

    #[test]
    fn a_grant_with_no_hosts_permits_no_hosts() {
        // The vacuous case, which this repository has now found three times in other places: an
        // empty list that answers "yes" to everything asked of it.
        let grant = NetworkGrant::default();
        assert_eq!(
            decide(&grant, &asked("github.com", 443)),
            Egress::NotGranted {
                host: "github.com".to_owned()
            }
        );
    }

    #[test]
    fn a_granted_host_is_not_granted_on_every_port() {
        // Otherwise a grant to reach github.com is a grant to reach anything listening anywhere on
        // it, and the broker is a tunnel with extra steps.
        let grant = NetworkGrant::to(&["github.com"]);
        assert_eq!(
            decide(&grant, &asked("github.com", 22)),
            Egress::NotOnThatPort { port: 22 }
        );
    }

    #[test]
    fn spelling_is_not_a_permission_boundary() {
        let grant = NetworkGrant::to(&["GitHub.com"]);
        assert_eq!(decide(&grant, &asked("github.com", 443)), Egress::Permitted);
    }

    #[test]
    fn a_subdomain_is_a_different_host() {
        // `permits` compares whole names. A grant to `github.com` that also admitted
        // `evil.github.com` would be a grant to whoever can register a subdomain, and a grant to
        // `github.com.evil.example` would be a grant to anybody at all.
        let grant = NetworkGrant::to(&["github.com"]);
        assert!(matches!(
            decide(&grant, &asked("raw.github.com", 443)),
            Egress::NotGranted { .. }
        ));
        assert!(matches!(
            decide(&grant, &asked("github.com.evil.example", 443)),
            Egress::NotGranted { .. }
        ));
    }

    #[test]
    fn the_addresses_a_capsule_may_never_be_connected_to() {
        // Every one of these is reachable through a name the grant permits, because a name is
        // controlled by whoever runs it and can answer with anything.
        assert!(!may_be_connected_to("127.0.0.1".parse().unwrap()));
        assert!(!may_be_connected_to("::1".parse().unwrap()));
        // The cloud metadata endpoint, which serves the machine's own credentials to anything that
        // asks. This is the one that turns a correct name check into a compromised host.
        assert!(!may_be_connected_to("169.254.169.254".parse().unwrap()));
        assert!(!may_be_connected_to("fe80::1".parse().unwrap()));
        assert!(!may_be_connected_to("0.0.0.0".parse().unwrap()));
        assert!(!may_be_connected_to("::".parse().unwrap()));
    }

    #[test]
    fn an_operator_who_grants_an_internal_host_means_it() {
        // Refusing private ranges would make this unusable on any network that has one, and a rule
        // people work around is a rule that ends up switched off.
        assert!(may_be_connected_to("10.1.2.3".parse().unwrap()));
        assert!(may_be_connected_to("192.168.1.10".parse().unwrap()));
        assert!(may_be_connected_to("140.82.121.4".parse().unwrap()));
    }
}
