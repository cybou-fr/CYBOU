// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Run one prompt turn against an ACP agent and print what came back, as JSON.
//!
//! Exists so the session gate tests this crate's own client rather than a JSON-RPC exchange somebody
//! wrote out by hand in a shell script. A gate against a hand-written exchange tests the shell
//! script, and passes forever after the code stops agreeing with it.
//!
//! Not a launch surface, and deliberately awkward to mistake for one: it takes a bare command and
//! runs it wherever it is told. `cybou-agentd` is what puts an agent inside a capsule first.

use std::path::PathBuf;
use std::time::Duration;

use agent_client_protocol::AcpAgentConfig;
use cybou_acp::AcpSession;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let workspace = arguments
        .next()
        .ok_or("usage: acp-turn <workspace> <prompt> <program> [argument …]")?;
    let prompt = arguments.next().ok_or("no prompt was given")?;
    let program = arguments.next().ok_or("no agent command was given")?;

    let mut process = AcpAgentConfig::new(program);
    for argument in arguments {
        process = process.arg(argument);
    }

    let seconds: u64 = std::env::var("CYBOU_ACP_TURN_SECONDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(30);
    let turn = AcpSession::within(Duration::from_secs(seconds))
        .one_turn(process, PathBuf::from(workspace), &prompt)
        .await?;

    println!("{}", serde_json::to_string_pretty(&turn)?);
    Ok(())
}
