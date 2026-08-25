// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Render the credential-free pack configuration for gates and deployment tooling.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_class = std::env::args()
        .nth(1)
        .ok_or("usage: render-config MODEL_CLASS")?;
    println!("{}", cybou_agent_opencode::configuration(&model_class)?);
    Ok(())
}
