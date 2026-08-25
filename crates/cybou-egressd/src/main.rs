// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The broker, listening.
//!
//! On a Unix socket rather than a port, because a port is reachable by everything on the host and a
//! socket is reachable by whatever the socket was given to. A capsule's network namespace has no
//! route to the host's loopback, so a broker listening on `127.0.0.1` would be reachable by every
//! other process on the machine and by no capsule at all — which is the wrong answer twice.
//!
//! One grant per broker. A single broker serving several capsules would have to work out which one
//! was on the other end of a connection, and the only thing it could work that out from is what the
//! capsule told it.

use std::path::PathBuf;

use cybou_capsule::grant::NetworkGrant;
use cybou_egressd::ask;
use cybou_egressd::decide::{Egress, decide, may_be_connected_to};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpStream, UnixListener, UnixStream};

/// The most a request line may be before it is refused unread.
///
/// A request is one line and a few headers. Without a ceiling, a capsule that opens a connection and
/// sends bytes without a newline is a capsule that makes the broker allocate until the host is out
/// of memory — from inside a sandbox whose whole purpose is that it cannot do that.
const THE_MOST_A_REQUEST_MAY_BE: u64 = 8 * 1024;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match serve().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("cybou-egressd: {why}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn serve() -> Result<(), String> {
    let (socket, grant) = parse(std::env::args().skip(1))?;

    // A stale socket from a broker that was killed rather than stopped. Removing it is safe in a way
    // that removing a path named on a command line usually is not: it has to be a socket, and it has
    // to be one nothing is listening on, or the bind below fails and this returns instead.
    if socket.exists() {
        std::fs::remove_file(&socket)
            .map_err(|why| format!("could not clear {}: {why}", socket.display()))?;
    }
    let listener = UnixListener::bind(&socket)
        .map_err(|why| format!("could not listen on {}: {why}", socket.display()))?;
    eprintln!(
        "cybou-egressd: listening on {} for {}",
        socket.display(),
        if grant.hosts.is_empty() {
            "no hosts at all".to_owned()
        } else {
            grant.hosts.join(", ")
        }
    );

    loop {
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|why| format!("could not accept: {why}"))?;
        let grant = grant.clone();
        // One task per connection, and a failure in one is not a failure of the broker. A capsule
        // that can end the broker by sending something malformed has found a way to deny every
        // other capsule its granted egress.
        tokio::spawn(async move {
            if let Err(why) = tunnel(stream, &grant).await {
                eprintln!("cybou-egressd: {why}");
            }
        });
    }
}

/// Read the argument list.
fn parse(arguments: impl Iterator<Item = String>) -> Result<(PathBuf, NetworkGrant), String> {
    let mut socket = None;
    let mut hosts = Vec::new();
    let mut arguments = arguments;

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--socket" => {
                socket = Some(PathBuf::from(
                    arguments.next().ok_or("--socket wants a path")?,
                ));
            }
            "--host" => hosts.push(arguments.next().ok_or("--host wants a name")?),
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    let socket = socket.ok_or("--socket must say where to listen")?;
    // A broker with no hosts is permitted and is not a mistake: it is what a capsule with no network
    // grant gets, and it refuses everything. Started by hand it is probably an error, which is why
    // the line above says so out loud rather than looking like a broker that is working.
    Ok((socket, NetworkGrant { hosts }))
}

/// Serve one connection.
async fn tunnel(capsule: UnixStream, grant: &NetworkGrant) -> Result<(), String> {
    let mut capsule = BufReader::new(capsule);
    let mut line = String::new();
    // Bounded before it is read, not after: the ceiling exists so a capsule cannot make the broker
    // allocate, and a check applied to what was already read would be too late to do that.
    let mut bounded = BufReader::new((&mut capsule).take(THE_MOST_A_REQUEST_MAY_BE));
    bounded
        .read_line(&mut line)
        .await
        .map_err(|why| format!("could not read a request: {why}"))?;

    let asked = match ask::read(line.trim_end()) {
        Ok(asked) => asked,
        Err(why) => return refuse(capsule.into_inner(), 400, &why.to_string()).await,
    };

    match decide(grant, &asked) {
        Egress::Permitted => {}
        Egress::NotGranted { host } => {
            return refuse(
                capsule.into_inner(),
                403,
                &format!("this capsule was not granted {host}"),
            )
            .await;
        }
        Egress::NotOnThatPort { port } => {
            return refuse(
                capsule.into_inner(),
                403,
                &format!("this capsule was not granted {} on port {port}", asked.host),
            )
            .await;
        }
    }

    // The one resolution, and it is the broker's. Everything above decided on a name.
    let target = format!("{}:{}", asked.host, asked.port);
    let addresses: Vec<_> = tokio::net::lookup_host(&target)
        .await
        .map_err(|why| format!("could not resolve {}: {why}", asked.host))?
        .collect();
    if addresses.is_empty() {
        return refuse(
            capsule.into_inner(),
            502,
            &format!("{} resolved to nothing", asked.host),
        )
        .await;
    }
    // Every address, not the first one. A name that answers with the metadata endpoint second would
    // otherwise be reached on a retry, and the check would have passed the first time.
    if let Some(refused) = addresses
        .iter()
        .find(|address| !may_be_connected_to(address.ip()))
    {
        return refuse(
            capsule.into_inner(),
            403,
            &format!(
                "{} resolves to {}, which is the host this capsule is running on",
                asked.host,
                refused.ip()
            ),
        )
        .await;
    }

    let mut outside = TcpStream::connect(&*addresses)
        .await
        .map_err(|why| format!("could not reach {target}: {why}"))?;

    let mut capsule = capsule.into_inner();
    capsule
        .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
        .await
        .map_err(|why| format!("could not answer: {why}"))?;

    // Bytes, uninterpreted, in both directions. The broker is not a place this traffic could be
    // read: it decided who the capsule may talk to, not what it may say.
    tokio::io::copy_bidirectional(&mut capsule, &mut outside)
        .await
        .map_err(|why| format!("{target} ended badly: {why}"))?;
    Ok(())
}

/// Say no, in a way the capsule's own client will understand.
///
/// With the reason. A capsule told only that it was refused cannot tell a grant it does not have
/// from a host that does not exist, and the agent inside it will retry the wrong thing forever.
async fn refuse(mut capsule: UnixStream, status: u16, why: &str) -> Result<(), String> {
    eprintln!("cybou-egressd: refused: {why}");
    let answer = format!(
        "HTTP/1.1 {status} Forbidden\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{why}",
        why.len()
    );
    capsule
        .write_all(answer.as_bytes())
        .await
        .map_err(|error| format!("could not refuse: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_of(arguments: &[&str]) -> Result<(PathBuf, NetworkGrant), String> {
        parse(arguments.iter().map(|argument| (*argument).to_owned()))
    }

    #[test]
    fn a_broker_is_told_where_to_listen_and_what_to_permit() {
        let (socket, grant) =
            parse_of(&["--socket", "/run/x.sock", "--host", "github.com"]).expect("parses");
        assert_eq!(socket, PathBuf::from("/run/x.sock"));
        assert_eq!(grant.hosts, vec!["github.com".to_owned()]);
    }

    #[test]
    fn a_broker_with_no_hosts_is_a_broker_that_refuses_everything() {
        // Not an error. It is what a capsule with no network grant gets, and a version that refused
        // to start would make "no network" the one configuration that needs special handling.
        let (_, grant) = parse_of(&["--socket", "/run/x.sock"]).expect("parses");
        assert!(grant.hosts.is_empty());
    }

    #[test]
    fn a_broker_that_does_not_know_where_to_listen_does_not_start() {
        assert!(parse_of(&["--host", "github.com"]).is_err());
    }

    #[test]
    fn an_unknown_argument_is_refused_rather_than_ignored() {
        // A flag from a newer builder, guessed at, is a guess about a boundary.
        assert!(parse_of(&["--socket", "/run/x.sock", "--allow-everything"]).is_err());
    }
}
