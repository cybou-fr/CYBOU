// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Development-only ACP initialization probe; production callers must supply a capsule entrypoint.

use agent_client_protocol::AcpAgentConfig;
use cybou_acp::AcpClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let command = args.next().ok_or("usage: probe-agent command [args...]")?;
    let process = AcpAgentConfig::new(command)
        .args(args.map(|argument| argument.to_string_lossy().into_owned()));
    let handshake = AcpClient::new().initialize(process).await?;
    println!("{}", serde_json::to_string_pretty(&handshake)?);
    Ok(())
}
