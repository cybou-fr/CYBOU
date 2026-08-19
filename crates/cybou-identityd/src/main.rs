// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-identityd` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

use cybou_identityd::{ARCHITECTURE_VERSION, IdentityCore};
use time::OffsetDateTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state_path = env::var("CYBOU_IDENTITY_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let state_dir = env::var("XDG_STATE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
                    PathBuf::from(home).join(".local/state")
                });
            state_dir.join("cybou/identity.json")
        });

    println!("[cybou-identityd] Managing identity at {}", state_path.display());
    let core = Arc::new(IdentityCore::open(&state_path));

    let now = OffsetDateTime::now_utc();
    let action = core.begin_session(now, ARCHITECTURE_VERSION)?;
    println!("[cybou-identityd] Session initialized with action: {:?}", action);

    if let Some(state) = core.current_state() {
        println!(
            "[cybou-identityd] Identity ID: {}, Session: {}, Age: {} days",
            state.identity_id,
            state.session_count,
            state.age_in_days()
        );
    }

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::IDENTITY;
        use cybou_identityd::service::Identity1Service;

        println!("[cybou-identityd] Connecting to D-Bus session bus...");
        let service = Identity1Service::new(core);
        let connection = zbus::connection::Builder::session()?
            .name(IDENTITY.service)?
            .serve_at(IDENTITY.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-identityd] Registered '{}' at '{}'",
            IDENTITY.service, IDENTITY.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-identityd] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-identityd] Running on non-Linux host in headless mode.");
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
