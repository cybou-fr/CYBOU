// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Minimal ACP v1 peer used only by the adversarial handshake gate.

use std::io::{BufRead, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let wrong_version = std::env::args().any(|argument| argument == "--wrong-version");
    let mut request = String::new();
    std::io::stdin().lock().read_line(&mut request)?;
    let request: serde_json::Value = serde_json::from_str(&request)?;
    if request["jsonrpc"] != "2.0"
        || request["method"] != "initialize"
        || request["params"]["protocolVersion"] != 1
    {
        return Err("client did not send an ACP v1 initialize request".into());
    }
    let response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": request["id"],
        "result": {
            "protocolVersion": if wrong_version { 99 } else { 1 },
            "agentCapabilities": {"loadSession": true},
            "authMethods": [{"id": "fake-login", "name": "Fake login"}],
            "agentInfo": {"name": "cybou-fake-agent", "title": "Cybou Fake Agent", "version": "1.0.0"}
        }
    });
    writeln!(std::io::stdout().lock(), "{response}")?;
    Ok(())
}
