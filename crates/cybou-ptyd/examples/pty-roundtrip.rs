// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Drive a live terminal owner over its socket and read back what the shell printed.
//!
//! The owner's own tests exercise the framing and the bounds. What they cannot do is prove that a
//! pseudoterminal was allocated, that a shell started in it as the account that owns the socket,
//! and that what the shell wrote came back — which is the whole of what a terminal is, and the part
//! that had never been checked outside a browser.
//!
//! Run by `scripts/test-terminal-gate.sh` as the account the owner runs as. `CYBOU_PTY_GATE_SOCKET`
//! is the socket to talk to.

use std::time::Duration;

use cybou_protocol::terminal::{FromGateway, FromOwner};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::UnixStream;

async fn send(
    stream: &mut UnixStream,
    frame: &FromGateway,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut body = Vec::new();
    ciborium::into_writer(frame, &mut body)?;
    let length = u32::try_from(body.len())?;
    stream.write_all(&length.to_be_bytes()).await?;
    stream.write_all(&body).await?;
    stream.flush().await?;
    Ok(())
}

async fn receive(stream: &mut UnixStream) -> Result<FromOwner, Box<dyn std::error::Error>> {
    let mut length = [0_u8; 4];
    stream.read_exact(&mut length).await?;
    let length = u32::from_be_bytes(length) as usize;
    let mut body = vec![0_u8; length];
    stream.read_exact(&mut body).await?;
    Ok(ciborium::from_reader(body.as_slice())?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket =
        std::env::var("CYBOU_PTY_GATE_SOCKET").map_err(|_| "CYBOU_PTY_GATE_SOCKET is required")?;
    let mut stream = UnixStream::connect(&socket).await?;

    send(
        &mut stream,
        &FromGateway::Open {
            columns: 80,
            rows: 24,
        },
    )
    .await?;
    match receive(&mut stream).await? {
        FromOwner::Opened => {}
        other => return Err(format!("expected the terminal to open, got {other:?}").into()),
    }

    // A command whose output could not have come from anywhere else. `id -u` answers with the uid
    // the shell is running as, which is the claim ADR-0047 makes and the one worth checking: a
    // terminal that ran as somebody else would be the failure that matters.
    send(&mut stream, &FromGateway::Input(b"id -u; exit\n".to_vec())).await?;

    let expected = unsafe_free_uid()?;
    let mut seen = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(format!(
                "the terminal never printed the uid; it said {:?}",
                String::from_utf8_lossy(&seen)
            )
            .into());
        }
        match tokio::time::timeout(remaining, receive(&mut stream)).await {
            Ok(Ok(FromOwner::Output(bytes))) => {
                seen.extend_from_slice(&bytes);
                if String::from_utf8_lossy(&seen).contains(&expected) {
                    break;
                }
            }
            Ok(Ok(FromOwner::Ended { code, signal })) => {
                return Err(format!(
                    "the shell ended (code {code:?}, signal {signal:?}) before printing the uid: {:?}",
                    String::from_utf8_lossy(&seen)
                )
                .into());
            }
            Ok(Ok(other)) => return Err(format!("unexpected frame: {other:?}").into()),
            Ok(Err(error)) => return Err(error),
            Err(_) => continue,
        }
    }

    println!("the terminal ran a shell as uid {expected} and gave its output back");
    Ok(())
}

/// This process's own user id, read from `/proc` rather than through `libc`.
///
/// The workspace forbids `unsafe`, and `getuid` is behind it. `/proc/self/status` reports the same
/// number and is what the executor reads for the same reason.
fn unsafe_free_uid() -> Result<String, Box<dyn std::error::Error>> {
    let status = std::fs::read_to_string("/proc/self/status")?;
    status
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .and_then(|rest| rest.split_whitespace().next())
        .map(str::to_owned)
        .ok_or_else(|| "no Uid line in /proc/self/status".into())
}
