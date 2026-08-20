// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-predictord` daemon entrypoint.

use std::{env, path::PathBuf, sync::Arc};

use cybou_predictord::PredictorCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[cybou-predictord] Initializing empirical forecasting and calibration engine...");
    let state_path = env::var("CYBOU_PREDICTOR_PATH").map_or_else(
        |_| {
            let state_dir = env::var("XDG_STATE_HOME").map_or_else(
                |_| {
                    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
                    PathBuf::from(home).join(".local/state")
                },
                PathBuf::from,
            );
            state_dir.join("cybou/predictor.json")
        },
        PathBuf::from,
    );

    let core = Arc::new(PredictorCore::open(&state_path)?);

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::PREDICTOR;
        use cybou_predictord::service::Predictor1Service;

        // Until now this organ had a replay routine and a persisted cursor but nothing that ever
        // called them: every forecast it could make came from whatever a caller happened to push
        // in through Observe, and a restart forecast from nothing. It follows the Journal, which
        // is where the observations actually are, and resumes from where its samples stand.
        let follow_core = Arc::clone(&core);
        tokio::spawn(async move {
            let from = follow_core.cursor();
            let caught_up_core = Arc::clone(&follow_core);
            if let Err(error) = cybou_fabric::event_client::follow_contributions_reporting(
                from,
                move |sequence, envelope| {
                    follow_core.ingest_envelope(envelope, sequence);
                },
                move || caught_up_core.mark_caught_up(),
            )
            .await
            {
                println!("[cybou-predictord] Cannot follow Event1: {error}");
            }
        });

        println!("[cybou-predictord] Connecting to D-Bus session bus...");
        let service = Predictor1Service::new(core);
        // Bound, not discarded: dropping the connection would release the well-known name.
        let _connection = zbus::connection::Builder::session()?
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
