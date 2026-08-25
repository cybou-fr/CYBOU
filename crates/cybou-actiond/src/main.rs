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

/// How many contributions to ask for at a time when reading the Journal back.
#[cfg(target_os = "linux")]
const REPLAY_WINDOW: u32 = 512;

/// Read this host's own action history back before answering anything.
///
/// Before, deliberately. A restarted Action1 that served a request while still ignorant of what it
/// had already proposed would be answering as though the host had no past, and the whole reason for
/// writing the lifecycle down is that it does.
///
/// A Journal that cannot be read leaves the owner empty rather than stopping it. The alternative is
/// a host that will not repair itself because it cannot remember, and the reason it acts on itself
/// at all is that nobody may be there to help.
#[cfg(target_os = "linux")]
async fn restore_from_journal(core: &std::sync::Arc<ActionCore>) {
    use cybou_actiond::journal;
    use cybou_fabric::event_client::EventClient;

    let Ok(client) = EventClient::session().await else {
        println!("[cybou-actiond] The Journal is unreachable; starting with no recorded history");
        return;
    };

    let mut envelopes = Vec::new();
    let mut after = 0_u64;
    loop {
        match client.replay(after, REPLAY_WINDOW).await {
            Ok(page) if page.is_empty() => break,
            Ok(page) => {
                after += page.len() as u64;
                let complete = page.len() < REPLAY_WINDOW as usize;
                envelopes.extend(page);
                if complete {
                    break;
                }
            }
            Err(error) => {
                println!("[cybou-actiond] The Journal replay stopped at {after}: {error}");
                break;
            }
        }
    }

    match journal::replay(&envelopes) {
        Ok(records) => {
            let restored = records.len();
            match core.restore(records) {
                Ok(held) => println!(
                    "[cybou-actiond] Restored {restored} decided action(s); holding {held}"
                ),
                Err(error) => println!("[cybou-actiond] Nothing was restored: {error}"),
            }
        }
        // A decision whose proposal is missing is a gap in the record, and it is said rather than
        // patched over. Starting empty is honest; starting with a decision nobody can trace to a
        // request is not.
        Err(error) => println!("[cybou-actiond] The recorded history is not readable: {error}"),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let core = Arc::new(ActionCore::new(policy_from_environment()?));

    #[cfg(target_os = "linux")]
    {
        use cybou_actiond::service::Action1Service;
        use cybou_fabric::ACTION;

        restore_from_journal(&core).await;

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
