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

use std::{
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use cybou_capsule::grant::NetworkGrant;
use cybou_egressd::ask;
use cybou_egressd::decide::{Egress, decide, may_be_connected_to};
use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader,
};
use tokio::net::{TcpStream, UnixListener, UnixStream};
use tokio::sync::Semaphore;

/// The most a request line may be before it is refused unread.
///
/// A request is one line and a few headers. Without a ceiling, a capsule that opens a connection and
/// sends bytes without a newline is a capsule that makes the broker allocate until the host is out
/// of memory — from inside a sandbox whose whole purpose is that it cannot do that.
const THE_MOST_A_REQUEST_MAY_BE: usize = 8 * 1024;

/// A capsule may consume at most this many host-side tasks, file descriptors and outbound sockets.
const MAXIMUM_CONCURRENT_TUNNELS: usize = 64;

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const DNS_AND_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const IDLE_TIMEOUT: Duration = Duration::from_mins(10);

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

    prepare_socket_path(&socket)?;
    let listener = UnixListener::bind(&socket)
        .map_err(|why| format!("could not listen on {}: {why}", socket.display()))?;
    restrict_socket(&socket)?;
    eprintln!(
        "cybou-egressd: listening on {} for {}",
        socket.display(),
        if grant.hosts.is_empty() {
            "no hosts at all".to_owned()
        } else {
            grant.hosts.join(", ")
        }
    );

    let tunnels = Arc::new(Semaphore::new(MAXIMUM_CONCURRENT_TUNNELS));
    loop {
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|why| format!("could not accept: {why}"))?;
        let Ok(permit) = Arc::clone(&tunnels).try_acquire_owned() else {
            tokio::spawn(async move {
                let _ = refuse(stream, 503, "this capsule has too many open tunnels").await;
            });
            continue;
        };
        let grant = grant.clone();
        // One task per connection, and a failure in one is not a failure of the broker. A capsule
        // that can end the broker by sending something malformed has found a way to deny every
        // other capsule its granted egress.
        tokio::spawn(async move {
            let _permit = permit;
            if let Err(why) = Box::pin(tunnel(stream, &grant)).await {
                eprintln!("cybou-egressd: {why}");
            }
        });
    }
}

/// Make the pathname a private broker endpoint, and never remove something merely because it has
/// the name a socket was expected to have.
fn prepare_socket_path(socket: &Path) -> Result<(), String> {
    use std::os::unix::fs::{FileTypeExt, PermissionsExt};

    let parent = socket
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", socket.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|why| format!("could not create {}: {why}", parent.display()))?;
    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
        .map_err(|why| format!("could not restrict {}: {why}", parent.display()))?;

    match std::fs::symlink_metadata(socket) {
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(why) => Err(format!("could not inspect {}: {why}", socket.display())),
        Ok(metadata) if metadata.file_type().is_socket() => {
            match std::os::unix::net::UnixStream::connect(socket) {
                Ok(_) => Err(format!(
                    "refusing to replace active socket {}",
                    socket.display()
                )),
                Err(why) if why.kind() == std::io::ErrorKind::ConnectionRefused => {
                    std::fs::remove_file(socket).map_err(|why| {
                        format!("could not clear stale socket {}: {why}", socket.display())
                    })
                }
                Err(why) => Err(format!(
                    "could not establish that socket {} is stale: {why}",
                    socket.display()
                )),
            }
        }
        Ok(_) => Err(format!(
            "refusing to replace non-socket path {}",
            socket.display()
        )),
    }
}

fn restrict_socket(socket: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(socket, std::fs::Permissions::from_mode(0o600))
        .map_err(|why| format!("could not restrict {}: {why}", socket.display()))
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
    let line = tokio::time::timeout(HANDSHAKE_TIMEOUT, read_handshake(&mut capsule))
        .await
        .map_err(|_| "the proxy handshake timed out".to_owned())??;

    let asked = match ask::read(line.trim_end()) {
        Ok(asked) => asked,
        Err(why) => return refuse(capsule, 400, &why.to_string()).await,
    };

    match decide(grant, &asked) {
        Egress::Permitted => {}
        Egress::NotGranted { host } => {
            return refuse(
                capsule,
                403,
                &format!("this capsule was not granted {host}"),
            )
            .await;
        }
        Egress::NotOnThatPort { port } => {
            return refuse(
                capsule,
                403,
                &format!("this capsule was not granted {} on port {port}", asked.host),
            )
            .await;
        }
    }

    // The one resolution, and it is the broker's. Everything above decided on a name.
    let target = format!("{}:{}", asked.host, asked.port);
    let addresses: Vec<_> =
        tokio::time::timeout(DNS_AND_CONNECT_TIMEOUT, tokio::net::lookup_host(&target))
            .await
            .map_err(|_| format!("resolving {} timed out", asked.host))?
            .map_err(|why| format!("could not resolve {}: {why}", asked.host))?
            .collect();
    if addresses.is_empty() {
        return refuse(capsule, 502, &format!("{} resolved to nothing", asked.host)).await;
    }
    // Every address, not the first one. A name that answers with the metadata endpoint second would
    // otherwise be reached on a retry, and the check would have passed the first time.
    if let Some(refused) = addresses
        .iter()
        .find(|address| !may_be_connected_to(address.ip()))
    {
        return refuse(
            capsule,
            403,
            &format!(
                "{} resolves to {}, which is the host this capsule is running on",
                asked.host,
                refused.ip()
            ),
        )
        .await;
    }

    // Exact interface addresses, checked at the last instant before connect. Private address space
    // remains usable: binding succeeds for this host's 10.0.0.4 and fails for another machine's
    // 10.0.0.20, which is the distinction the policy needs.
    if let Some(local) = addresses
        .iter()
        .find(|address| address_is_on_this_host(address.ip()))
    {
        return refuse(
            capsule,
            403,
            &format!(
                "{} resolves to {}, which is an address of this host",
                asked.host,
                local.ip()
            ),
        )
        .await;
    }

    let mut outside =
        tokio::time::timeout(DNS_AND_CONNECT_TIMEOUT, TcpStream::connect(&*addresses))
            .await
            .map_err(|_| format!("connecting to {target} timed out"))?
            .map_err(|why| format!("could not reach {target}: {why}"))?;

    capsule
        .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
        .await
        .map_err(|why| format!("could not answer: {why}"))?;

    // Bytes, uninterpreted, in both directions. The broker is not a place this traffic could be
    // read: it decided who the capsule may talk to, not what it may say.
    Box::pin(copy_until_idle(&mut capsule, &mut outside))
        .await
        .map_err(|why| format!("{target} ended badly: {why}"))?;
    Ok(())
}

/// Consume a complete CONNECT handshake and return its request line.
///
/// The entire header block is bounded before each read. Bytes after the blank line stay buffered
/// in `capsule` and become the first opaque tunnel bytes; no proxy header can leak into TLS.
async fn read_handshake<R>(capsule: &mut R) -> Result<String, String>
where
    R: AsyncBufRead + Unpin,
{
    let mut total = 0_usize;
    let mut request = None;
    loop {
        let remaining = THE_MOST_A_REQUEST_MAY_BE.saturating_sub(total);
        if remaining == 0 {
            return Err("the proxy handshake is larger than 8 KiB".to_owned());
        }
        let mut line = String::new();
        let read = (&mut *capsule)
            .take(u64::try_from(remaining + 1).unwrap_or(u64::MAX))
            .read_line(&mut line)
            .await
            .map_err(|why| format!("could not read a request: {why}"))?;
        if read == 0 {
            return Err("the proxy handshake ended before its blank line".to_owned());
        }
        total = total.saturating_add(read);
        if total > THE_MOST_A_REQUEST_MAY_BE {
            return Err("the proxy handshake is larger than 8 KiB".to_owned());
        }
        if request.is_none() {
            request = Some(line.trim_end_matches(['\r', '\n']).to_owned());
        }
        if line == "\r\n" || line == "\n" {
            return Ok(request.unwrap_or_default());
        }
    }
}

/// Whether this exact address is assigned to the host now.
///
/// Binding port zero asks the kernel's current interface table and sends no traffic. A deployment
/// with non-local bind enabled may conservatively refuse a remote address; it never opens one.
fn address_is_on_this_host(address: IpAddr) -> bool {
    std::net::TcpListener::bind(SocketAddr::new(address.to_canonical(), 0)).is_ok()
}

/// Copy opaque bytes in both directions, resetting one idle deadline on activity either way.
async fn copy_until_idle<A, B>(capsule: &mut A, outside: &mut B) -> std::io::Result<()>
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    let mut from_capsule = [0_u8; 16 * 1024];
    let mut from_outside = [0_u8; 16 * 1024];
    let mut capsule_open = true;
    let mut outside_open = true;
    while capsule_open || outside_open {
        let moved = tokio::time::timeout(IDLE_TIMEOUT, async {
            tokio::select! {
                read = capsule.read(&mut from_capsule), if capsule_open => {
                    let read = read?;
                    if read == 0 {
                        capsule_open = false;
                        outside.shutdown().await?;
                    } else {
                        outside.write_all(&from_capsule[..read]).await?;
                    }
                    Ok::<(), std::io::Error>(())
                }
                read = outside.read(&mut from_outside), if outside_open => {
                    let read = read?;
                    if read == 0 {
                        outside_open = false;
                        capsule.shutdown().await?;
                    } else {
                        capsule.write_all(&from_outside[..read]).await?;
                    }
                    Ok::<(), std::io::Error>(())
                }
            }
        })
        .await;
        match moved {
            Ok(result) => result?,
            Err(_) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "idle tunnel",
                ));
            }
        }
    }
    Ok(())
}

/// Say no, in a way the capsule's own client will understand.
///
/// With the reason. A capsule told only that it was refused cannot tell a grant it does not have
/// from a host that does not exist, and the agent inside it will retry the wrong thing forever.
async fn refuse<W>(mut capsule: W, status: u16, why: &str) -> Result<(), String>
where
    W: AsyncWrite + Unpin,
{
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
    use std::os::unix::fs::PermissionsExt;

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

    #[tokio::test]
    async fn a_complete_proxy_handshake_is_consumed_and_payload_is_not() {
        let (client, server) = tokio::io::duplex(1024);
        let mut server = BufReader::new(server);
        tokio::spawn(async move {
            let mut client = client;
            client
                .write_all(
                    b"CONNECT github.com:443 HTTP/1.1\r\nHost: github.com:443\r\nProxy-Connection: keep-alive\r\n\r\nTLS",
                )
                .await
                .expect("write handshake");
        });
        assert_eq!(
            read_handshake(&mut server).await.expect("handshake"),
            "CONNECT github.com:443 HTTP/1.1"
        );
        let mut payload = [0_u8; 3];
        server.read_exact(&mut payload).await.expect("payload");
        assert_eq!(&payload, b"TLS");
    }

    #[tokio::test]
    async fn an_unbounded_handshake_is_refused_at_eight_kib() {
        let (mut client, server) = tokio::io::duplex(THE_MOST_A_REQUEST_MAY_BE + 1);
        let mut server = BufReader::new(server);
        tokio::spawn(async move {
            client
                .write_all(&vec![b'x'; THE_MOST_A_REQUEST_MAY_BE + 1])
                .await
                .expect("write oversized request");
        });
        assert!(read_handshake(&mut server).await.is_err());
    }

    #[test]
    fn only_a_socket_may_be_replaced() {
        let root = std::env::temp_dir().join(format!("cybou-egressd-{}", std::process::id()));
        let socket = root.join("egress.sock");
        std::fs::create_dir_all(&root).expect("runtime directory");
        std::fs::write(&socket, b"do not erase").expect("regular file");
        assert!(prepare_socket_path(&socket).is_err());
        assert_eq!(
            std::fs::read(&socket).expect("still there"),
            b"do not erase"
        );
        std::fs::remove_file(&socket).expect("remove fixture");
        let listener = std::os::unix::net::UnixListener::bind(&socket).expect("socket");
        assert!(prepare_socket_path(&socket).is_err(), "active socket");
        drop(listener);
        prepare_socket_path(&socket).expect("stale socket may be removed");
        assert!(!socket.exists());
        assert_eq!(
            std::fs::metadata(&root)
                .expect("runtime directory")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        std::fs::remove_dir(&root).expect("remove fixture directory");
    }

    #[test]
    fn the_kernel_answers_whether_an_address_belongs_to_this_host() {
        assert!(address_is_on_this_host("127.0.0.1".parse().unwrap()));
        assert!(!address_is_on_this_host("192.0.2.1".parse().unwrap()));
    }
}
