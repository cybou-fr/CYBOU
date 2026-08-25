// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-actiond` daemon entrypoint.

use std::sync::Arc;

use cybou_actiond::ActionCore;
use cybou_remediation::{Operation, StandingPolicy};

fn policy_from_environment() -> Result<StandingPolicy, String> {
    let mut policy = StandingPolicy::nothing_pre_authorized();
    for verb in std::env::var("CYBOU_PREAUTHORIZED_ACTIONS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|verb| !verb.is_empty())
    {
        let operation = match verb {
            "service.status" => Operation::InspectServiceStatus,
            "package.cache.clean" => Operation::CleanPackageCache,
            "service.restart" => Operation::RestartService,
            _ => {
                return Err(format!(
                    "standing policy names unsupported operation '{verb}'"
                ));
            }
        };
        policy.pre_authorized.push(operation);
    }
    Ok(policy)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let core = Arc::new(ActionCore::new(policy_from_environment()?));

    #[cfg(target_os = "linux")]
    {
        use cybou_actiond::service::Action1Service;
        use cybou_fabric::ACTION;

        let builder = if std::env::var_os("CYBOU_ACTION_SYSTEM_BUS").is_some() {
            zbus::connection::Builder::system()?
        } else {
            zbus::connection::Builder::session()?
        };
        let _connection = builder
            .name(ACTION.service)?
            .serve_at(ACTION.object_path, Action1Service::new(core))?
            .build()
            .await?;
        println!("[cybou-actiond] Registered {}", ACTION.service);
        tokio::signal::ctrl_c().await?;
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = core;
        tokio::signal::ctrl_c().await?;
    }
    Ok(())
}
