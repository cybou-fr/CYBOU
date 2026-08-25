// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Opaque transport from an agent's loopback HTTP client to its host model-gateway socket.
//!
//! This process runs inside the capsule and owns no policy or credential. The host gateway validates
//! the ephemeral bearer token and enforces lease identity, model class, lifetime and spend.

use std::{net::Ipv4Addr, path::PathBuf};

use tokio::net::{TcpListener, TcpStream, UnixStream};

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    match serve().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("cybou-model-bridge: {why}");
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
                eprintln!("cybou-model-bridge: {why}");
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
        socket.ok_or("--socket must name the gateway")?,
    ))
}

async fn forward(mut client: TcpStream, socket: &PathBuf) -> Result<(), String> {
    let mut gateway = UnixStream::connect(socket)
        .await
        .map_err(|why| format!("could not reach gateway {}: {why}", socket.display()))?;
    tokio::io::copy_bidirectional(&mut client, &mut gateway)
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
            parse_of(&["--port", "3130", "--socket", "/run/cybou/model.sock"]),
            Ok((3130, PathBuf::from("/run/cybou/model.sock")))
        );
    }

    #[test]
    fn authority_shaped_arguments_are_refused() {
        assert!(parse_of(&["--token", "secret"]).is_err());
        assert!(parse_of(&["--model", "Strong"]).is_err());
    }
}
