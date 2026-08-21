// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Loopback-only process entry point for the read-only Cybou web gateway.

use std::{net::SocketAddr, sync::Arc};

#[cfg(target_os = "linux")]
use cybou_web_contracts::SessionMode;
use cybou_web_gateway::{
    DisclosureSink, PresenceSource, SessionContext, access::CredentialVerifier,
    fixture::FixturePresenceSource, router_recording_disclosures,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Two sources, not one source and a flag. The filtered one is what a stranger reads; the full
    // one is reachable only by signing in, and only if this deployment can authenticate anybody.
    let permitted = publishable_sensitivity_or_default();
    let presence: Arc<dyn PresenceSource> = source(Some(permitted)).await?;
    let verifier = credential_verifier();
    let privileged: Option<Arc<dyn PresenceSource>> = if verifier.is_some() {
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
        // and the unfiltered one is reachable only by signing in. A route added later cannot
        // forget to filter, because filtering is not something a route does.
        println!("public projection withholds anything above sensitivity {permitted}");
    }

    println!("cybou-web-gateway listening on http://{address}");
    if verifier.is_some() {
        println!("sign-in is available; a reader who signs in is served unfiltered");
    } else {
        println!("sign-in is unavailable; every reader is served the public projection");
    }
    axum::serve(
        listener,
        router_recording_disclosures(
            presence,
            privileged,
            verifier,
            disclosure_sink(),
            web_root,
            session_context,
        ),
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

/// What can say whether a Linux account accepts a secret, if anything on this host can.
///
/// The gateway asks; it never checks. Checking needs the shadow database, which needs privilege
/// this process must not have, so the question goes to `cybou-authd` over a socket only this user
/// can open. An unset socket path means this deployment cannot authenticate anybody, which is a
/// valid way to run a public demo and is said at startup rather than left to be discovered.
#[cfg(target_os = "linux")]
fn credential_verifier() -> Option<Arc<dyn CredentialVerifier>> {
    let path = std::env::var_os("CYBOU_AUTH_SOCKET")?;
    Some(Arc::new(
        cybou_web_gateway::auth_socket::HelperVerifier::at(std::path::PathBuf::from(path)),
    ))
}

#[cfg(not(target_os = "linux"))]
fn credential_verifier() -> Option<Arc<dyn CredentialVerifier>> {
    // The helper is a unix socket and a PAM stack; there is neither here.
    None
}

/// Where this deployment records what it supplied to whom.
///
/// `None` on a fixture-backed or non-Linux gateway: there is no Journal behind it, and a sink that
/// silently discarded records would make a deployment that cannot say who read what look exactly
/// like one that can.
#[cfg(target_os = "linux")]
fn disclosure_sink() -> Option<Arc<dyn DisclosureSink>> {
    if std::env::var_os("CYBOU_GATEWAY_FIXTURE").is_some() {
        return None;
    }
    Some(Arc::new(cybou_web_gateway::auth_socket::JournalSink))
}

#[cfg(not(target_os = "linux"))]
fn disclosure_sink() -> Option<Arc<dyn DisclosureSink>> {
    None
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
