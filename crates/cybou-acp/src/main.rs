// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Read-only ACP discovery and handshake probe.

use cybou_acp::RegistryBrowser;

fn usage() -> &'static str {
    "usage: cybou-acp registry [query] [--json]"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    match args
        .next()
        .and_then(|arg| arg.into_string().ok())
        .as_deref()
    {
        Some("registry") => registry(args.collect()).await,
        _ => Err(usage().into()),
    }
}

async fn registry(args: Vec<std::ffi::OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let mut query = String::new();
    let mut json = false;
    for arg in args {
        let arg = arg
            .into_string()
            .map_err(|_| "registry argument is not UTF-8")?;
        if arg == "--json" {
            json = true;
        } else if query.is_empty() {
            query = arg;
        } else {
            return Err(usage().into());
        }
    }

    let snapshot = RegistryBrowser::new()?.fetch().await?;
    let matches = snapshot.search(&query);
    if json {
        println!("{}", serde_json::to_string_pretty(&matches)?);
    } else {
        println!(
            "ACP registry {} observed {} — {} match(es)",
            snapshot.index.version,
            snapshot.observed_at,
            matches.len()
        );
        for agent in matches {
            println!(
                "{}\t{}\t{}\t{}",
                agent.id,
                agent.version,
                agent.name,
                agent.distribution_kinds().join(",")
            );
        }
    }
    Ok(())
}
