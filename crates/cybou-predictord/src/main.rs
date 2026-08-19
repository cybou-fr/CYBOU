// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-predictord` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

use cybou_predictord::PredictorCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-predictord] Initializing empirical forecasting and calibration engine...");
    let state_path = env::var("CYBOU_PREDICTOR_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let state_dir = env::var("XDG_STATE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
                    PathBuf::from(home).join(".local/state")
                });
            state_dir.join("cybou/predictor.json")
        });

    let core = Arc::new(PredictorCore::open(&state_path)?);

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::PREDICTOR;
        use cybou_predictord::service::Predictor1Service;

        println!("[cybou-predictord] Connecting to D-Bus session bus...");
        let service = Predictor1Service::new(core);
        let connection = zbus::connection::Builder::session()?
            .name(PREDICTOR.service)?
            .serve_at(PREDICTOR.object_path, service)?
            .build()
            .await?;

        println!(
            "[cybou-predictord] Registered '{}' at '{}'",
            PREDICTOR.service, PREDICTOR.object_path
        );

        tokio::signal::ctrl_c().await?;
        println!("[cybou-predictord] Shutting down.");
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-predictord] Running on non-Linux host in headless mode.");
        let _ = core;
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}
