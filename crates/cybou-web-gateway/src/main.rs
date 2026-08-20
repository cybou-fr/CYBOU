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
    let address = SocketAddr::from(([127, 0, 0, 1], 8787));
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

/// The most a public, unauthenticated surface may publish.
///
/// `Ordinary` on the frozen scale: no particular exposure concern. Anything above it is about the
/// person — theirs to release — and releasing it to everyone is not a decision a running process
/// should make on their behalf.
#[cfg(target_os = "linux")]
const PUBLISHABLE_SENSITIVITY: u8 = 0;

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
    use cybou_fabric::EVENT;

    let connection = zbus::Connection::session().await?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);

    loop {
        let answer: Option<u8> = connection
            .call_method(
                Some(EVENT.service),
                EVENT.object_path,
                Some(EVENT.interface),
                "HighestSensitivity",
                &(),
            )
            .await
            .ok()
            .and_then(|reply| reply.body().deserialize().ok());

        match answer {
            Some(highest) if highest > PUBLISHABLE_SENSITIVITY => {
                return Err(format!(
                    "refusing to serve an unauthenticated public surface: the Journal holds                      contributions at sensitivity {highest}, above the {PUBLISHABLE_SENSITIVITY}                      a public surface may publish. Serve this deployment behind authentication,                      or run it in local-desktop mode."
                )
                .into());
            }
            Some(_) => return Ok(()),
            None if std::time::Instant::now() < deadline => {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            None => {
                return Err("refusing to serve an unauthenticated public surface: Event1 did not                             answer, so what this deployment would publish is unknown"
                    .into());
            }
        }
    }
}
