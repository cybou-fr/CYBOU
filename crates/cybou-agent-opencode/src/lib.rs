// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The first reproducible ACP agent pack.
//!
//! `OpenCode` receives only a loopback OpenAI-compatible endpoint and a file containing its
//! ephemeral lease token. The provider key remains in the host worker. The capsule boundary, not
//! `OpenCode`'s
//! own configuration, remains the authority for files, processes, network, model class and spend.

use std::path::PathBuf;

use serde_json::json;

/// Upstream ACP registry identity.
pub const AGENT_ID: &str = "opencode";
/// Immutable upstream release selected for this pack.
pub const VERSION: &str = "1.18.23";
/// Installation root visible read-only through the capsule's `/usr` mount.
pub const INSTALL_ROOT: &str = "/usr/local/libexec/cybou/agents/opencode/1.18.23";
/// Capsule workspace config path. It contains no long-lived credential.
pub const CONFIG_INSIDE: &str = "/workspace/.cybou/opencode.json";
/// Capsule-local OpenAI-compatible gateway.
pub const GATEWAY_BASE_URL: &str = "http://127.0.0.1:3130/v1";
/// Ephemeral lease-token file mounted read-only for this capsule.
pub const TOKEN_INSIDE: &str = "/run/cybou/model-token";

/// One immutable upstream artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Archive {
    /// Rust-style architecture name.
    pub architecture: &'static str,
    /// Upstream HTTPS archive.
    pub url: &'static str,
    /// Lowercase SHA-256 from the upstream ACP registry entry.
    pub sha256: &'static str,
}

/// Resolve the archive for a supported Debian architecture.
#[must_use]
pub fn archive(architecture: &str) -> Option<Archive> {
    match architecture {
        "x86_64" => Some(Archive {
            architecture: "x86_64",
            url: "https://github.com/anomalyco/opencode/releases/download/v1.18.23/opencode-linux-x64.tar.gz",
            sha256: "ab7015cd8113e011a461f30a0c2b77d8299a144ff688cb62e93e8802835d7288",
        }),
        "aarch64" => Some(Archive {
            architecture: "aarch64",
            url: "https://github.com/anomalyco/opencode/releases/download/v1.18.23/opencode-linux-arm64.tar.gz",
            sha256: "86d3afaf4e8784f9adab189be2a315c12b27ec40a04b70defbe70595c3cc7c65",
        }),
        _ => None,
    }
}

/// Render the credential-free `OpenCode` configuration for one granted model class.
///
/// The token is referenced by pathname and never serialized into the configuration.
///
/// # Errors
///
/// Returns a serialization error if the JSON value cannot be rendered.
pub fn configuration(model_class: &str) -> Result<String, serde_json::Error> {
    let model = format!("cybou/{model_class}");
    serde_json::to_string_pretty(&json!({
        "$schema": "https://opencode.ai/config.json",
        "model": model,
        "share": "disabled",
        "autoupdate": false,
        "provider": {
            "cybou": {
                "npm": "@ai-sdk/openai-compatible",
                "name": "Cybou lease gateway",
                "options": {
                    "baseURL": GATEWAY_BASE_URL,
                    "apiKey": format!("{{file:{TOKEN_INSIDE}}}")
                },
                "models": {
                    model_class: {
                        "name": format!("Cybou {model_class}")
                    }
                }
            }
        }
    }))
}

/// Exact ACP subprocess command for this pack.
#[must_use]
pub fn command() -> Vec<String> {
    vec![
        "/usr/bin/env".to_owned(),
        format!("OPENCODE_CONFIG={CONFIG_INSIDE}"),
        PathBuf::from(INSTALL_ROOT)
            .join("opencode")
            .to_string_lossy()
            .into_owned(),
        "acp".to_owned(),
        "--cwd".to_owned(),
        "/workspace".to_owned(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_is_pinned_by_version_and_digest() {
        let release = archive("x86_64").expect("supported");
        assert!(release.url.contains(&format!("/v{VERSION}/")));
        assert_eq!(release.sha256.len(), 64);
        assert!(release.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert!(archive("riscv64").is_none());
    }

    #[test]
    fn config_names_only_the_capsule_gateway_and_token_file() {
        let config = configuration("Strong").expect("config");
        assert!(config.contains(GATEWAY_BASE_URL));
        assert!(config.contains(TOKEN_INSIDE));
        assert!(!config.contains("api.openai.com"));
        assert!(!config.contains("sk-"));
        assert!(!config.contains("provider-key"));
    }

    #[test]
    fn a_model_class_is_json_data_and_not_configuration_syntax() {
        let config = configuration("Strong\"},\"evil\":true,").expect("config");
        let parsed: serde_json::Value = serde_json::from_str(&config).expect("valid JSON");
        assert!(parsed.get("evil").is_none());
    }

    #[test]
    fn acp_command_uses_only_read_only_installation_and_workspace_paths() {
        let command = command();
        assert_eq!(command.last().map(String::as_str), Some("/workspace"));
        assert!(command.iter().any(|part| part.ends_with("/opencode")));
        assert!(command.iter().any(|part| part == "acp"));
    }
}
