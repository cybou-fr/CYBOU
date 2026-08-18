// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Loopback-only process entry point for the read-only Cybou web gateway.

use std::{net::SocketAddr, sync::Arc};

use cybou_web_gateway::{PresenceSource, fixture::FixturePresenceSource, router_with_assets};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let presence: Arc<dyn PresenceSource> = source().await?;
    let address = SocketAddr::from(([127, 0, 0, 1], 8787));
    let listener = tokio::net::TcpListener::bind(address).await?;
    let web_root = std::env::var_os("CYBOU_WEB_ROOT").map(std::path::PathBuf::from);
    println!("cybou-web-gateway listening on http://{address}");
    axum::serve(listener, router_with_assets(presence, web_root)).await?;
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
