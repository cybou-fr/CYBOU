// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Loopback-only process entry point for the read-only Cybou web gateway.

use std::{net::SocketAddr, sync::Arc};

#[cfg(target_os = "linux")]
use cybou_web_contracts::SessionMode;
use cybou_web_gateway::{
    PresenceSource, SessionContext, fixture::FixturePresenceSource, router_with_privileged_access,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Two sources, not one source and a flag. The filtered one is what a stranger reads; the full
    // one is reachable only through the credential below, and only if this deployment has one.
    let permitted = publishable_sensitivity_or_default();
    let presence: Arc<dyn PresenceSource> = source(Some(permitted)).await?;
    let credential = access_credential()?;
    let privileged: Option<Arc<dyn PresenceSource>> = if credential.is_some() {
        Some(source(None).await?)
    } else {
        None
    };
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
        // What used to happen here was a refusal to start at all whenever the Journal held
        // anything above ordinary. That was the right instinct and the wrong instrument: it took
        // the whole surface down over rows it would not have shown anyway, and the pressure to get
        // the surface back is what produced a raised threshold that outlived its reason and
        // published a person's words.
        //
        // What replaces it is not another watcher. A watcher comparing what is served against what
        // exists cannot tell "nothing needed withholding" from "withholding is broken", because a
        // contribution above the line need not produce anything a projection would carry — and a
        // check that fires when nothing is wrong gets removed by whoever it wakes at night.
        //
        // The guarantee is structural instead: there is one filtered source, every route reads it,
        // and the unfiltered one is reachable only through the credential. A route added later
        // cannot forget to filter, because filtering is not something a route does.
        println!("public projection withholds anything above sensitivity {permitted}");
    }

    println!("cybou-web-gateway listening on http://{address}");
    if credential.is_some() {
        println!("a credential is configured; readers presenting it are served unfiltered");
    } else {
        println!("no credential is configured; every reader is served the public projection");
    }
    axum::serve(
        listener,
        router_with_privileged_access(presence, privileged, credential, web_root, session_context),
    )
    .await?;
    Ok(())
}

/// A source that passes on nothing above `permitted`, or everything when `permitted` is `None`.
#[cfg_attr(not(target_os = "linux"), allow(clippy::unused_async))]
async fn source(
    permitted: Option<u8>,
) -> Result<Arc<dyn PresenceSource>, Box<dyn std::error::Error>> {
    if std::env::var_os("CYBOU_GATEWAY_FIXTURE").is_some() {
        return Ok(Arc::new(FixturePresenceSource::nominal()));
    }

    #[cfg(target_os = "linux")]
    {
        Ok(Arc::new(
            cybou_web_gateway::presence_zbus::ZbusPresenceSource::connect_permitting(
                permitted.unwrap_or(u8::MAX),
            )
            .await?,
        ))
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = permitted;
        Err("live Presence1 adapter is available only on Linux; set CYBOU_GATEWAY_FIXTURE=1".into())
    }
}

/// The credential that entitles a reader to the unfiltered projection, if one is configured.
///
/// Read from a file rather than the environment: a unit file is world-readable and a process
/// environment is readable by anyone who can see the process, and a secret in either is a secret
/// in the clear. An unset path means this deployment has nobody to entitle, which is a valid
/// configuration and the one a public demo runs with.
///
/// A configured path that cannot be read is an error rather than a shrug. Falling back to "no
/// credential" would silently turn a deployment that meant to have privileged access into one that
/// serves everyone the same thing, and it would look exactly like it was working.
fn access_credential() -> Result<Option<Arc<str>>, Box<dyn std::error::Error>> {
    let Some(path) = std::env::var_os("CYBOU_ACCESS_CREDENTIAL_FILE") else {
        return Ok(None);
    };
    let raw = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read the access credential file: {error}"))?;
    let credential = raw.trim();
    if credential.is_empty() {
        return Err(
            "the access credential file is empty; remove the setting or put a credential in it"
                .into(),
        );
    }
    Ok(Some(Arc::from(credential)))
}

/// What a public surface may publish, on every platform.
///
/// The Linux build reads the owner's decision; elsewhere there is no Journal to classify, so the
/// filter has nothing to do and the strict default stands.
fn publishable_sensitivity_or_default() -> u8 {
    #[cfg(target_os = "linux")]
    {
        publishable_sensitivity()
    }
    #[cfg(not(target_os = "linux"))]
    {
        0
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
