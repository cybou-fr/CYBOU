// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Loopback-only process entry point for the read-only Cybou web gateway.

use std::{net::SocketAddr, sync::Arc};

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
