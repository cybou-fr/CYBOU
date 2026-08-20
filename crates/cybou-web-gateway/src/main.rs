// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Loopback-only process entry point for the read-only Cybou web gateway.

use std::{net::SocketAddr, sync::Arc};

#[cfg(target_os = "linux")]
use cybou_web_contracts::SessionMode;
use cybou_web_gateway::{
    PresenceSource, SessionContext, fixture::FixturePresenceSource, router_with_assets_and_session,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let presence: Arc<dyn PresenceSource> = source().await?;
    // Loopback always; the port is settable so a gate can start one of these beside a deployment
    // instead of having to stop the deployment to test the thing that protects it.
    let address: SocketAddr = match std::env::var("CYBOU_GATEWAY_ADDR") {
        Ok(value) => value
            .parse()
            .map_err(|_| format!("CYBOU_GATEWAY_ADDR is not an address: {value}"))?,
        Err(_) => SocketAddr::from(([127, 0, 0, 1], 8787)),
    };
    if !address.ip().is_loopback() {
        return Err(format!("refusing to bind {address}: this gateway is loopback only").into());
    }
    let listener = tokio::net::TcpListener::bind(address).await?;
    let web_root = std::env::var_os("CYBOU_WEB_ROOT").map(std::path::PathBuf::from);
    let session_context = match std::env::var("CYBOU_SESSION_MODE") {
        Ok(value) if value == "public-preview" => SessionContext::public_preview(),
        Ok(value) if value == "local-desktop" => SessionContext::local_desktop(),
        Err(std::env::VarError::NotPresent) => SessionContext::local_desktop(),
        Ok(value) => return Err(format!("unsupported CYBOU_SESSION_MODE: {value}").into()),
        Err(error) => return Err(error.into()),
    };
    #[cfg(target_os = "linux")]
    if session_context.mode == SessionMode::PublicPreview
        && std::env::var_os("CYBOU_GATEWAY_FIXTURE").is_none()
    {
        refuse_publishing_personal_state().await?;
        // Checking once at startup guards the door of a room somebody is already in: a promise
        // recorded a minute after this process came up would be published by it for as long as it
        // kept running. The same question is asked again while it serves.
        tokio::spawn(stop_when_the_journal_turns_personal());
    }

    println!("cybou-web-gateway listening on http://{address}");
    axum::serve(
        listener,
        router_with_assets_and_session(presence, web_root, session_context),
    )
    .await?;
    Ok(())
}

#[cfg_attr(not(target_os = "linux"), allow(clippy::unused_async))]
async fn source() -> Result<Arc<dyn PresenceSource>, Box<dyn std::error::Error>> {
    if std::env::var_os("CYBOU_GATEWAY_FIXTURE").is_some() {
        return Ok(Arc::new(FixturePresenceSource::nominal()));
    }

    #[cfg(target_os = "linux")]
    {
        Ok(Arc::new(
            cybou_web_gateway::presence_zbus::ZbusPresenceSource::connect().await?,
        ))
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err("live Presence1 adapter is available only on Linux; set CYBOU_GATEWAY_FIXTURE=1".into())
    }
}

/// The most a public, unauthenticated surface may publish unless the owner says otherwise.
///
/// `Ordinary` on the frozen scale: no particular exposure concern. Anything above it is about the
/// person — theirs to release — and releasing it to everyone is not a decision a running process
/// should make on their behalf.
#[cfg(target_os = "linux")]
const DEFAULT_PUBLISHABLE_SENSITIVITY: u8 = 0;

/// What this deployment's owner has decided a public surface may publish.
///
/// Raising it is a decision, and it is taken where decisions belong: in the unit that starts the
/// process, visible to whoever reads it, not in a constant somebody would have to recompile. The
/// default refuses everything above ordinary, so an unset value is the strict one.
#[cfg(target_os = "linux")]
fn publishable_sensitivity() -> u8 {
    std::env::var("CYBOU_PUBLISHABLE_SENSITIVITY")
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(DEFAULT_PUBLISHABLE_SENSITIVITY)
}

/// Refuse to serve an unauthenticated public surface over a Journal that holds personal state.
///
/// This exists because the alternative was remembering. The deployment is public and
/// unauthenticated by an explicit decision, taken while the Journal held facts about a machine and
/// nothing about a person. That decision stays right exactly as long as that stays true, and
/// nothing was watching for the moment it stopped being true.
///
/// So the moment is watched here instead. It costs nothing while the Journal holds machine facts,
/// and the first time a person puts something of their own in, the public surface stops rather
/// than quietly publishing it.
///
/// A Journal that cannot be reached is not a Journal known to be safe, so it refuses too, after
/// giving Event1 time to come up.
#[cfg(target_os = "linux")]
async fn refuse_publishing_personal_state() -> Result<(), Box<dyn std::error::Error>> {
    let permitted = publishable_sensitivity();
    let connection = zbus::Connection::session().await?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);

    loop {
        let answer = highest_sensitivity(&connection).await;

        match answer {
            Some(highest) if highest > permitted => {
                return Err(format!(
                    "refusing to serve an unauthenticated public surface: the Journal holds contributions at sensitivity {highest}, above the {permitted} this deployment permits. Serve it behind authentication, run it in local-desktop mode, or raise CYBOU_PUBLISHABLE_SENSITIVITY if publishing that is a decision you are making."
                )
                .into());
            }
            Some(_) => return Ok(()),
            None if std::time::Instant::now() < deadline => {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            None => {
                return Err("refusing to serve an unauthenticated public surface: what this deployment would publish could not be established"
                    .into());
            }
        }
    }
}

/// How often a running surface re-asks what it would be publishing.
#[cfg(target_os = "linux")]
const RECHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(15);

/// How many consecutive unestablished answers a running surface tolerates before stopping.
#[cfg(target_os = "linux")]
const UNKNOWN_TOLERANCE: u32 = 4;

/// Ask Event1 what the most exposing thing in the Journal is.
///
/// `None` covers both a call that failed and an answer of -1, which is how Event1 says it could
/// not establish one. To a caller they are the same fact: what would be published is unknown, and
/// not knowing is not permission.
#[cfg(target_os = "linux")]
async fn highest_sensitivity(connection: &zbus::Connection) -> Option<u8> {
    use cybou_fabric::EVENT;

    let answer: i16 = connection
        .call_method(
            Some(EVENT.service),
            EVENT.object_path,
            Some(EVENT.interface),
            "HighestSensitivity",
            &(),
        )
        .await
        .ok()?
        .body()
        .deserialize()
        .ok()?;
    u8::try_from(answer).ok()
}

/// Stop serving once the Journal holds more than this deployment permits.
///
/// Exiting rather than refusing individual requests: a public surface that has started publishing
/// personal state should stop being a public surface, not answer some routes and not others. The
/// unit restarts it, the startup check refuses, and it lands in failed with the reason in the
/// journal — the same place an operator would look, saying the same thing.
///
/// A Journal that stops answering does not stop the surface. That would turn every transient
/// hiccup in one owner into an outage of another, and the startup check already refuses to begin
/// on an unknown Journal; this one only reacts to an answer it actually received.
#[cfg(target_os = "linux")]
async fn stop_when_the_journal_turns_personal() {
    let permitted = publishable_sensitivity();
    let Ok(connection) = zbus::Connection::session().await else {
        return;
    };
    let mut interval = tokio::time::interval(RECHECK_INTERVAL);
    let mut unknown_for: u32 = 0;

    loop {
        interval.tick().await;

        // A Journal that does not answer for a moment is a hiccup, not a disclosure: an outage in
        // one owner should not take another down. A Journal that keeps not answering is different,
        // because a surface that cannot learn what it is publishing is publishing blind.
        let answer = highest_sensitivity(&connection).await;
        if answer.is_none() {
            unknown_for = unknown_for.saturating_add(1);
            if unknown_for >= UNKNOWN_TOLERANCE {
                eprintln!(
                    "what this deployment is publishing has been unestablished for {unknown_for} checks; a public surface stops rather than publishing blind"
                );
                std::process::exit(1);
            }
            continue;
        }
        unknown_for = 0;

        if let Some(highest) = answer
            && highest > permitted
        {
            eprintln!(
                "the Journal now holds contributions at sensitivity {highest}, above the {permitted} this deployment permits; a public surface stops rather than publishing them"
            );
            std::process::exit(1);
        }
    }
}
