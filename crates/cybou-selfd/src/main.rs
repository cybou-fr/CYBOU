// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-selfd` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

use cybou_identityd::IdentityCore;
use cybou_selfd::SelfCore;

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

    println!("[cybou-selfd] Initializing self organ...");
    let identity_core = IdentityCore::open(&state_path);
    let (age_in_days, sessions, arch) = if let Ok(Some(state)) = identity_core.load_state() {
        (
            state.age_in_days(),
            state.session_count,
            state.architecture_version,
        )
    } else {
        (0, 1, "debian-rust-1.0".to_string())
    };

    let core = Arc::new(SelfCore::new(age_in_days, sessions, arch));

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::SELF;
        use cybou_selfd::service::Self1Service;

        println!("[cybou-selfd] Connecting to D-Bus session bus...");
        let service = Self1Service::new(core);
        let connection = zbus::connection::Builder::session()?
            .name(SELF.service)?
            .serve_at(SELF.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-selfd] Registered '{}' at '{}'",
            SELF.service, SELF.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-selfd] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!(
            "[cybou-selfd] Running on non-Linux host in headless mode (sessions: {}).",
            sessions
        );
        let _ = core;
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
