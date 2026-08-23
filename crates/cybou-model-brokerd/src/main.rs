// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-model-brokerd` daemon entrypoint.

use std::sync::Arc;

use cybou_model_brokerd::BrokerCore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // No worker is registered, because no inference runtime is implemented. This is the supported
    // `NoModel` configuration of ADR-0021 and not a stub: the faculty comes up, answers what it can
    // answer, and tells every caller what happens instead of a model. Registering a backend here is
    // the whole of what installing one will mean.
    let core = Arc::new(BrokerCore::new());

    println!(
        "[cybou-model-brokerd] Model brokerage faculty starting with {} model(s) installed",
        usize::from(core.has_a_model())
    );

    #[cfg(target_os = "linux")]
    {
        use cybou_fabric::MODEL_BROKER;
        use cybou_model_brokerd::service::ModelBroker1Service;

        let connection = zbus::connection::Builder::session()?
            .name(MODEL_BROKER.service)?
            .serve_at(
                MODEL_BROKER.object_path,
                ModelBroker1Service::new(core.clone()),
            )?
            .build()
            .await?;
        println!(
            "[cybou-model-brokerd] Serving {} at {}",
            MODEL_BROKER.interface, MODEL_BROKER.object_path
        );
        std::future::pending::<()>().await;
        drop(connection);
    }

    #[cfg(not(target_os = "linux"))]
    {
        println!("[cybou-model-brokerd] D-Bus is Linux-only; nothing to serve here.");
    }

    Ok(())
}
