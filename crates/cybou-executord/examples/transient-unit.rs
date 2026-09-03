// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Run one command as a transient systemd unit and report what it did, for host proofs.
//!
//! Usage: `transient-unit <system|user> <absolute command> [args...]`.
//!
//! Prints `ran` when the command exited zero, and the adapter's own refusal otherwise. It exists so
//! the mechanism behind package installation can be exercised against a real service manager
//! without needing anything installable, and without touching the system manager on a machine where
//! nothing may.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("transient-unit is Linux-only");
    std::process::exit(1);
}

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use cybou_executord::transient::{Manager, run, unit_name};

    let mut arguments = std::env::args().skip(1);
    let manager = match arguments.next().as_deref() {
        Some("system") => Manager::System,
        Some("user") => Manager::User,
        _ => return Err("first argument is 'system' or 'user'".into()),
    };
    let argv: Vec<String> = arguments.collect();
    if argv.is_empty() {
        return Err("a command to run is required".into());
    }

    let connection = manager.connect().await?;
    match run(&connection, &unit_name("probe"), &argv, &[]).await {
        Ok(()) => {
            println!("ran");
            Ok(())
        }
        Err(error) => {
            println!("refused: {error}");
            std::process::exit(2);
        }
    }
}
