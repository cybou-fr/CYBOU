// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

use cybou_jailfs::JailFs;
use cybou_shelld::ShellEngine;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let jail_path = std::env::var("CYBOU_SHELL_JAIL")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/var/lib/cybou/shell-jail"));
    let jail = JailFs::new(jail_path);
    let _engine = ShellEngine::new(jail);
    eprintln!("cybou-shelld: running with bounded capabilities...");
    tokio::signal::ctrl_c().await?;
    eprintln!("cybou-shelld: shutting down");
    Ok(())
}
