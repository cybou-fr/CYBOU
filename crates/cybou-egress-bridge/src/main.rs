// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Compatibility transport between ordinary HTTP proxy clients and a pathname Unix socket.
//!
//! This process runs inside the capsule. It does not parse CONNECT, resolve DNS, read a grant, or
//! decide anything: `cybou-egressd` does all four after these bytes reach it. Killing this bridge
//! removes convenience and grants nothing, because the network namespace still has no route.

use std::{net::Ipv4Addr, path::PathBuf};

use tokio::net::{TcpListener, TcpStream, UnixStream};

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    match serve().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("cybou-egress-bridge: {why}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn serve() -> Result<(), String> {
    let (port, socket) = parse(std::env::args().skip(1))?;
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))
        .await
        .map_err(|why| format!("could not listen on 127.0.0.1:{port}: {why}"))?;
    loop {
        let (client, _) = listener
            .accept()
            .await
            .map_err(|why| format!("could not accept: {why}"))?;
        let socket = socket.clone();
        tokio::spawn(async move {
            if let Err(why) = forward(client, &socket).await {
                eprintln!("cybou-egress-bridge: {why}");
            }
        });
    }
}

fn parse(arguments: impl Iterator<Item = String>) -> Result<(u16, PathBuf), String> {
    let mut port = None;
    let mut socket = None;
    let mut arguments = arguments;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--port" => {
                let value = arguments.next().ok_or("--port wants a number")?;
                let parsed = value
                    .parse::<u16>()
                    .map_err(|_| format!("{value} is not a port"))?;
                if parsed == 0 {
                    return Err("zero is not a port".to_owned());
                }
                port = Some(parsed);
            }
            "--socket" => {
                socket = Some(PathBuf::from(
                    arguments.next().ok_or("--socket wants a path")?,
                ));
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok((
        port.ok_or("--port must say where to listen")?,
        socket.ok_or("--socket must name the broker")?,
    ))
}

async fn forward(mut client: TcpStream, socket: &PathBuf) -> Result<(), String> {
    let mut broker = UnixStream::connect(socket)
        .await
        .map_err(|why| format!("could not reach broker {}: {why}", socket.display()))?;
    tokio::io::copy_bidirectional(&mut client, &mut broker)
        .await
        .map_err(|why| format!("transport ended badly: {why}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_of(arguments: &[&str]) -> Result<(u16, PathBuf), String> {
        parse(arguments.iter().map(|argument| (*argument).to_owned()))
    }

    #[test]
    fn the_bridge_is_told_only_transport_plumbing() {
        assert_eq!(
            parse_of(&["--port", "3128", "--socket", "/run/cybou/egress.sock"]),
            Ok((3128, PathBuf::from("/run/cybou/egress.sock")))
        );
    }

    #[test]
    fn policy_shaped_arguments_are_refused() {
        assert!(parse_of(&["--host", "github.com"]).is_err());
        assert!(parse_of(&["--grant", "anything"]).is_err());
    }
}
