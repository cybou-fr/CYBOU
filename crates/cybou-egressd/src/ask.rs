// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What a capsule asked for, read off the wire.
//!
//! One request shape and no others: `CONNECT host:port HTTP/1.1`. Not because a fuller proxy would
//! be hard, but because every method other than `CONNECT` is one where the broker would be handling
//! the capsule's payload — parsing its headers, following its redirects, deciding what a `Host:`
//! line means when it disagrees with the request line. A tunnel that carries bytes it does not
//! interpret has one decision to get right, and it is the one this file exists for.
//!
//! ## Why a name and never an address
//!
//! A grant names `github.com`. If this accepted `CONNECT 140.82.121.4:443`, the name would be
//! decoration: a capsule would reach anything it could find an address for, and the check above it
//! would be a check on a string nobody had to use. So an address literal is refused here, before any
//! grant is consulted, and the refusal is about grammar rather than about permission.
//!
//! That is also what closes the gap a firewall cannot: the capsule never resolves anything and never
//! supplies an address. There is exactly one resolution, it happens after the decision, and it is
//! the broker's own — so there is no window between checking a name and using it.

use std::fmt;

/// A request this broker understands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Asked {
    /// The host, as the capsule wrote it.
    pub host: String,
    /// The port.
    pub port: u16,
}

impl fmt::Display for Asked {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.host, self.port)
    }
}

/// Why a request was not understood.
///
/// Separate from a request that was understood and refused. A capsule told "malformed" when it was
/// in fact denied learns nothing about its grant, and one told "denied" when it sent nonsense goes
/// looking for a permission problem that is not there.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotUnderstood {
    /// Something other than `CONNECT`.
    NotATunnel(String),
    /// No `host:port`.
    NoTarget,
    /// A port that is not a number, or is zero.
    NotAPort(String),
    /// An address where a name belongs.
    ///
    /// The one refusal here that is about security rather than syntax, and it is here rather than
    /// with the others because a grant cannot answer it: there is no host name to check.
    AnAddressAndNotAName(String),
}

impl fmt::Display for NotUnderstood {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotATunnel(method) => {
                write!(
                    formatter,
                    "this broker only tunnels; {method} is not CONNECT"
                )
            }
            Self::NoTarget => write!(formatter, "no host:port in the request"),
            Self::NotAPort(what) => write!(formatter, "{what} is not a port"),
            Self::AnAddressAndNotAName(literal) => write!(
                formatter,
                "{literal} is an address; a grant names hosts, so an address cannot be checked \
                 against one"
            ),
        }
    }
}

/// Read a request line.
///
/// # Errors
///
/// Returns [`NotUnderstood`] for anything that is not a `CONNECT` to a named host and a port.
pub fn read(line: &str) -> Result<Asked, NotUnderstood> {
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    if !method.eq_ignore_ascii_case("CONNECT") {
        return Err(NotUnderstood::NotATunnel(method.to_owned()));
    }

    let target = parts.next().ok_or(NotUnderstood::NoTarget)?;
    // From the right: an IPv6 literal is full of colons, and splitting from the left would read the
    // first group of one as a host name.
    let (host, port) = target.rsplit_once(':').ok_or(NotUnderstood::NoTarget)?;
    if host.is_empty() {
        return Err(NotUnderstood::NoTarget);
    }

    let port: u16 = port
        .parse()
        .map_err(|_| NotUnderstood::NotAPort(port.to_owned()))?;
    if port == 0 {
        return Err(NotUnderstood::NotAPort(port.to_string()));
    }

    if is_an_address(host) {
        return Err(NotUnderstood::AnAddressAndNotAName(host.to_owned()));
    }

    Ok(Asked {
        host: host.to_owned(),
        port,
    })
}

/// Whether this is an address rather than a name.
///
/// Both families. An IPv6 literal arrives in brackets, which is not part of the address, and a
/// version of this that forgot to remove them would decide every IPv6 literal was a host name — the
/// failure being invisible, because the connection would then be refused by the grant instead and
/// look exactly like a working check.
fn is_an_address(host: &str) -> bool {
    let bare = host
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(host);
    bare.parse::<std::net::IpAddr>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tunnel_to_a_named_host_is_understood() {
        assert_eq!(
            read("CONNECT github.com:443 HTTP/1.1"),
            Ok(Asked {
                host: "github.com".to_owned(),
                port: 443
            })
        );
    }

    #[test]
    fn an_address_is_refused_before_any_grant_is_consulted() {
        // Otherwise the name in a grant is decoration: a capsule reaches whatever it can resolve
        // for itself, and the check above this is a check on a string nobody had to use.
        assert_eq!(
            read("CONNECT 140.82.121.4:443 HTTP/1.1"),
            Err(NotUnderstood::AnAddressAndNotAName(
                "140.82.121.4".to_owned()
            ))
        );
        assert_eq!(
            read("CONNECT [2606:50c0:8000::153]:443 HTTP/1.1"),
            Err(NotUnderstood::AnAddressAndNotAName(
                "[2606:50c0:8000::153]".to_owned()
            ))
        );
        // The brackets are not part of the address. A version that forgot to strip them would call
        // every IPv6 literal a host name, and the mistake would hide behind the grant refusing it.
        assert!(is_an_address("[::1]"));
        assert!(is_an_address("::1"));
        assert!(!is_an_address("github.com"));
    }

    #[test]
    fn a_name_that_merely_looks_numeric_is_still_a_name() {
        // `1.2.3.4.example.com` is a host. So is `999.999.999.999`, which is not an address.
        assert!(read("CONNECT 1.2.3.4.example.com:443 HTTP/1.1").is_ok());
        assert!(read("CONNECT 999.999.999.999:443 HTTP/1.1").is_ok());
    }

    #[test]
    fn only_a_tunnel_is_offered() {
        // Any other method makes the broker handle the capsule's payload — its headers, its
        // redirects, its disagreements between a request line and a Host: line. A tunnel has one
        // decision to get right.
        assert_eq!(
            read("GET http://github.com/ HTTP/1.1"),
            Err(NotUnderstood::NotATunnel("GET".to_owned()))
        );
        assert_eq!(read("",), Err(NotUnderstood::NotATunnel(String::new())));
    }

    #[test]
    fn the_method_is_read_without_regard_to_case() {
        assert!(read("connect github.com:443 HTTP/1.1").is_ok());
    }

    #[test]
    fn a_request_without_a_port_is_not_a_request_with_a_default_one() {
        // Guessing 443 would mean a capsule could omit the port and have the broker choose what it
        // meant. What a tunnel connects to should be what was asked for.
        assert_eq!(
            read("CONNECT github.com HTTP/1.1"),
            Err(NotUnderstood::NoTarget)
        );
        assert_eq!(read("CONNECT :443 HTTP/1.1"), Err(NotUnderstood::NoTarget));
        assert!(matches!(
            read("CONNECT github.com:0 HTTP/1.1"),
            Err(NotUnderstood::NotAPort(_))
        ));
        assert!(matches!(
            read("CONNECT github.com:https HTTP/1.1"),
            Err(NotUnderstood::NotAPort(_))
        ));
    }

    #[test]
    fn a_malformed_request_and_a_denied_one_are_different_answers() {
        // A capsule told "malformed" when it was denied learns nothing about its grant; one told
        // "denied" for nonsense goes looking for a permission problem that is not there.
        let malformed = read("HELLO");
        assert!(matches!(malformed, Err(NotUnderstood::NotATunnel(_))));
    }
}
