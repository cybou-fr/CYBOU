// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! CYBOU Bounded Body Shell capability daemon.

use cybou_jailfs::JailFs;
use cybou_shelld::ShellEngine;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let default_jail = if std::path::Path::new("/home/demo").exists() {
        PathBuf::from("/home/demo")
    } else {
        PathBuf::from("/var/lib/cybou/shell-jail")
    };
    let jail_path = std::env::var("CYBOU_SHELL_JAIL").map_or(default_jail, PathBuf::from);
    let jail = JailFs::new(jail_path)?;
    let _engine = ShellEngine::new(jail);
    eprintln!("cybou-shelld: running with bounded capabilities...");
    tokio::signal::ctrl_c().await?;
    eprintln!("cybou-shelld: shutting down");
    Ok(())
}
