// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The gateway's half of an interactive terminal: a WebSocket at one end, an owner's socket at the
//! other, and nothing of its own in between.
//!
//! This is the first bidirectional surface the gateway has, and it is deliberately the thinnest
//! thing that can carry one. It does not parse a frame, does not know what a keystroke means, and
//! does not decide what may run — [ADR-0047](../../../../docs/adr/ADR-0047-interactive-terminal-under-the-authenticated-account.md)
//! puts that boundary at the account and the kernel. What the gateway supplies is the one fact
//! neither end can establish for itself: which Linux account is at the keyboard.
//!
//! It cannot spawn a shell and must not be able to. It runs as `cybou`; the owner it connects to
//! runs as the person.

use std::path::PathBuf;

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::HeaderMap,
    response::Response,
};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

use crate::state::GatewayState;

/// The most bytes one frame from the browser may carry.
///
/// The same bound the owner applies, deliberately. A gateway that accepted a larger frame than the
/// owner would have a gap in the boundary exactly the width of the disagreement, and the refusal
/// would arrive from a process the browser cannot see.
const MAX_FRAME_BYTES: usize = cybou_protocol::terminal::MAX_FRAME_BYTES;

/// Where per-account terminal owners are addressed, when a deployment has enabled any.
///
/// Absent means no terminal exists on this host. That is a supported configuration and the ordinary
/// one: the unit ships disabled, and enabling it is an act naming an account.
#[must_use]
pub fn owner_directory() -> Option<PathBuf> {
    std::env::var_os("CYBOU_PTY_SOCKET_DIR").map(PathBuf::from)
}

/// Which socket this account's terminal would be at, if this deployment has any.
///
/// Separate from reading the environment so the rule can be checked without one. `None` is not a
/// path this function declined to build: it is a host on which no terminal exists, which is the
/// state every deployment is in until an operator enables one.
#[must_use]
pub fn socket_for(directory: Option<&std::path::Path>, uid: u32) -> Option<PathBuf> {
    directory.map(|directory| cybou_protocol::terminal::socket_path(directory, uid))
}

/// `GET /api/v1/terminal`
///
/// Refuses before upgrading rather than after. A socket that opened and then closed would look to
/// a browser like a terminal that crashed, and the difference between *this host has no terminal
/// for you* and *your terminal died* is what a person needs in order to know whether to ask an
/// operator for access.
///
/// # Errors
///
/// Refuses with `403` when the request holds no seat with a Linux account behind it, and `503`
/// when this deployment has no terminal owner for that account — which is every deployment until
/// an operator enables one, and is therefore an ordinary answer rather than a fault.
pub async fn terminal_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Result<Response, (axum::http::StatusCode, axum::Json<crate::state::ErrorBody>)> {
    // A seat, and specifically one with a numeric identity behind it. The local desktop seat holds
    // no PAM account, so there is no account for a terminal to be: this refuses rather than
    // guessing one, because guessing would mean choosing whose shell somebody gets.
    let uid = state
        .session_for(&headers)
        .and_then(|session| session.uid)
        .ok_or((
            axum::http::StatusCode::FORBIDDEN,
            axum::Json(crate::state::ErrorBody {
                schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
                error: "terminalRequiresAnAuthenticatedAccount",
                retryable: false,
            }),
        ))?;

    let socket = socket_for(owner_directory().as_deref(), uid).ok_or((
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        axum::Json(crate::state::ErrorBody {
            schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
            error: "terminalUnavailable",
            // Not retryable by the browser. What is missing is an operator enabling
            // `cybou-ptyd@` for this account, which no amount of asking again produces.
            retryable: false,
        }),
    ))?;

    // Connected before the upgrade, so a host with no owner for this account is a refusal a person
    // can read rather than a WebSocket that opens and immediately closes.
    let owner = tokio::net::UnixStream::connect(&socket)
        .await
        .map_err(|_| {
            (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                axum::Json(crate::state::ErrorBody {
                    schema_version: cybou_web_contracts::WEB_SCHEMA_V1,
                    error: "terminalUnavailable",
                    retryable: false,
                }),
            )
        })?;

    Ok(upgrade.on_upgrade(move |socket| carry(socket, owner)))
}

/// Move bytes between one browser and one owner until either stops.
///
/// The frames are the owner's, unchanged. The gateway re-frames rather than re-encodes: a
/// WebSocket message already has a length, and the owner's socket needs one, so the only difference
/// between the two ends is four bytes of prefix. Decoding here would put a second parser on the
/// path with nothing to add and its own opinions about malformed input.
async fn carry(browser: WebSocket, owner: tokio::net::UnixStream) {
    let (mut to_browser, mut from_browser) = {
        use futures_util::StreamExt as _;
        browser.split()
    };
    let (mut from_owner, mut to_owner) = tokio::io::split(owner);

    let browser_to_owner = async move {
        use futures_util::StreamExt as _;
        while let Some(Ok(message)) = from_browser.next().await {
            let payload = match message {
                Message::Binary(bytes) => bytes,
                // A close ends the session, and so does text: a terminal frame is CBOR and never
                // text, so a browser sending it has the wrong idea about this socket, and
                // forwarding it would hand the owner something it will refuse anyway.
                Message::Text(_) | Message::Close(_) => break,
                Message::Ping(_) | Message::Pong(_) => continue,
            };
            if payload.len() > MAX_FRAME_BYTES {
                break;
            }
            let Ok(length) = u32::try_from(payload.len()) else {
                break;
            };
            if to_owner.write_all(&length.to_be_bytes()).await.is_err()
                || to_owner.write_all(&payload).await.is_err()
            {
                break;
            }
        }
        // Closing this half ends the owner's session, which is what shutting a tab should do.
        let _ = to_owner.shutdown().await;
    };

    let owner_to_browser = async move {
        use futures_util::SinkExt as _;
        loop {
            let mut length = [0_u8; 4];
            if from_owner.read_exact(&mut length).await.is_err() {
                break;
            }
            let length = u32::from_be_bytes(length) as usize;
            if length > MAX_FRAME_BYTES {
                break;
            }
            let mut body = vec![0_u8; length];
            if from_owner.read_exact(&mut body).await.is_err() {
                break;
            }
            if to_browser.send(Message::Binary(body.into())).await.is_err() {
                break;
            }
        }
        let _ = to_browser.close().await;
    };

    // Either end going away ends the other. A session outliving its browser is a shell nobody is
    // attached to; a browser outliving its session is a terminal that has stopped answering and
    // does not say so.
    tokio::select! {
        () = browser_to_owner => {}
        () = owner_to_browser => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gateway_bounds_a_frame_exactly_where_the_owner_does() {
        // Read from the protocol rather than restated, so the two cannot drift. A gateway that
        // accepted more than the owner would forward a frame whose refusal arrives from a process
        // the browser cannot see.
        assert_eq!(MAX_FRAME_BYTES, cybou_protocol::terminal::MAX_FRAME_BYTES);
    }

    #[test]
    fn no_configured_directory_means_no_terminal_rather_than_a_guessed_one() {
        // Guessing a path would mean the gateway deciding a deployment had enabled something it
        // had not, and finding out by connecting to whatever happened to be at that path.
        assert!(socket_for(None, 1000).is_none());
    }

    #[test]
    fn a_terminal_is_addressed_by_the_uid_the_gateway_authenticated() {
        // The numeric identity the privileged helper established, never a name from the request.
        // A name is a lookup, and a lookup is a place where the answer can change between
        // authenticating somebody and connecting to their owner.
        assert_eq!(
            socket_for(Some(std::path::Path::new("/run/cybou-pty")), 1000),
            Some(PathBuf::from("/run/cybou-pty/1000/owner.sock"))
        );
    }
}
