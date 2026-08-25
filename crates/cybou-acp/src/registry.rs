// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Read-only discovery from the public ACP registry.

use std::collections::{BTreeMap, HashSet};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;

/// Canonical upstream registry index published by the ACP project.
pub const UPSTREAM_REGISTRY_URL: &str =
    "https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json";
const MAX_REGISTRY_BYTES: usize = 8 * 1024 * 1024;
const FETCH_TIMEOUT: Duration = Duration::from_secs(15);

/// The upstream registry document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryIndex {
    /// Registry format version.
    pub version: String,
    /// Agent manifests aggregated by the upstream registry.
    pub agents: Vec<RegistryAgent>,
}

/// Metadata for one upstream ACP agent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryAgent {
    /// Stable upstream slug.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Published agent version.
    pub version: String,
    /// Upstream description.
    pub description: String,
    /// Source repository, when supplied.
    #[serde(default)]
    pub repository: Option<String>,
    /// Product website, when supplied.
    #[serde(default)]
    pub website: Option<String>,
    /// Manifest authors.
    #[serde(default)]
    pub authors: Vec<String>,
    /// Manifest license expression or label.
    #[serde(default)]
    pub license: Option<String>,
    /// Upstream icon URL, when supplied.
    #[serde(default)]
    pub icon: Option<String>,
    /// Upstream distribution declarations keyed by distribution type.
    pub distribution: BTreeMap<String, serde_json::Value>,
}

impl RegistryAgent {
    /// Distribution kinds declared by the upstream manifest, in stable order.
    #[must_use]
    pub fn distribution_kinds(&self) -> Vec<&str> {
        self.distribution.keys().map(String::as_str).collect()
    }
}

/// One fetched view of the mutable upstream registry.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySnapshot {
    /// Exact source used for discovery.
    pub source: &'static str,
    /// UTC observation time; it is not a claim that metadata remains current afterwards.
    #[serde(with = "time::serde::rfc3339")]
    pub observed_at: OffsetDateTime,
    /// Validated upstream registry contents.
    pub index: RegistryIndex,
}

impl RegistrySnapshot {
    /// Case-insensitive local search across identity, name and description.
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<&RegistryAgent> {
        let query = query.trim().to_lowercase();
        let mut matches = self
            .index
            .agents
            .iter()
            .filter(|agent| {
                query.is_empty()
                    || agent.id.to_lowercase().contains(&query)
                    || agent.name.to_lowercase().contains(&query)
                    || agent.description.to_lowercase().contains(&query)
            })
            .collect::<Vec<_>>();
        matches.sort_by_key(|agent| (agent.name.to_lowercase(), agent.id.as_str()));
        matches
    }
}

/// Failure to fetch or validate the public registry.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// HTTPS fetch failed or returned a non-success status.
    #[error("upstream ACP registry fetch failed: {0}")]
    Fetch(#[from] reqwest::Error),
    /// The response exceeded the browser's fixed memory ceiling.
    #[error("upstream ACP registry exceeds {MAX_REGISTRY_BYTES} bytes")]
    TooLarge,
    /// The response was not an upstream registry document.
    #[error("invalid upstream ACP registry: {0}")]
    Invalid(String),
}

/// Fetches and validates the public ACP registry without installing from it.
#[derive(Clone, Debug)]
pub struct RegistryBrowser {
    http: reqwest::Client,
}

impl RegistryBrowser {
    /// Build a browser pinned to the canonical HTTPS endpoint and a fixed timeout.
    ///
    /// # Errors
    ///
    /// Returns an error only when the HTTPS client cannot be configured.
    pub fn new() -> Result<Self, RegistryError> {
        Ok(Self {
            http: reqwest::Client::builder()
                .timeout(FETCH_TIMEOUT)
                .https_only(true)
                .build()?,
        })
    }

    /// Fetch the latest public registry snapshot.
    ///
    /// This performs discovery only. Distribution commands and URLs remain inert metadata.
    ///
    /// # Errors
    ///
    /// Returns a typed error for network failures, oversized documents, malformed JSON, duplicate
    /// identities, or incomplete manifests.
    pub async fn fetch(&self) -> Result<RegistrySnapshot, RegistryError> {
        let mut response = self
            .http
            .get(UPSTREAM_REGISTRY_URL)
            .send()
            .await?
            .error_for_status()?;
        if response
            .content_length()
            .is_some_and(|length| length > MAX_REGISTRY_BYTES as u64)
        {
            return Err(RegistryError::TooLarge);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if bytes.len().saturating_add(chunk.len()) > MAX_REGISTRY_BYTES {
                return Err(RegistryError::TooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        let index = Self::parse(&bytes)?;
        Ok(RegistrySnapshot {
            source: UPSTREAM_REGISTRY_URL,
            observed_at: OffsetDateTime::now_utc(),
            index,
        })
    }

    /// Parse and validate registry bytes already obtained from the canonical source.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed JSON, duplicate identities, blank required fields or missing
    /// distribution declarations.
    pub fn parse(bytes: &[u8]) -> Result<RegistryIndex, RegistryError> {
        let index: RegistryIndex = serde_json::from_slice(bytes)
            .map_err(|error| RegistryError::Invalid(error.to_string()))?;
        if index.version.trim().is_empty() {
            return Err(RegistryError::Invalid("blank registry version".to_owned()));
        }
        let mut identities = HashSet::with_capacity(index.agents.len());
        for agent in &index.agents {
            if agent.id.trim().is_empty()
                || agent.name.trim().is_empty()
                || agent.version.trim().is_empty()
                || agent.description.trim().is_empty()
            {
                return Err(RegistryError::Invalid(format!(
                    "agent '{}' has a blank required field",
                    agent.id
                )));
            }
            if agent.distribution.is_empty() {
                return Err(RegistryError::Invalid(format!(
                    "agent '{}' has no distribution",
                    agent.id
                )));
            }
            if !identities.insert(agent.id.as_str()) {
                return Err(RegistryError::Invalid(format!(
                    "duplicate agent identity '{}'",
                    agent.id
                )));
            }
        }
        Ok(index)
    }
}

impl Default for RegistryBrowser {
    fn default() -> Self {
        Self::new().expect("static HTTPS client configuration is valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REGISTRY: &[u8] = br#"{
      "version":"1.0.0",
      "agents":[
        {"id":"z-agent","name":"Zeta","version":"2.0.0","description":"Terminal agent","distribution":{"npx":{"package":"z@2"}}},
        {"id":"a-agent","name":"Alpha","version":"1.0.0","description":"Code repair","distribution":{"binary":{"linux-x86_64":{"cmd":"./alpha"}}}}
      ]
    }"#;

    #[test]
    fn browser_preserves_upstream_manifests_and_sorts_search_results() {
        let index = RegistryBrowser::parse(REGISTRY).expect("valid registry");
        let snapshot = RegistrySnapshot {
            source: UPSTREAM_REGISTRY_URL,
            observed_at: OffsetDateTime::UNIX_EPOCH,
            index,
        };
        let all = snapshot.search("");
        assert_eq!(all[0].id, "a-agent");
        assert_eq!(all[1].distribution_kinds(), vec!["npx"]);
        assert_eq!(snapshot.search("REPAIR")[0].id, "a-agent");
    }

    #[test]
    fn duplicate_upstream_identities_are_refused() {
        let duplicate = REGISTRY.to_vec();
        let mut value: serde_json::Value = serde_json::from_slice(&duplicate).expect("valid JSON");
        let first = value["agents"][0].clone();
        value["agents"].as_array_mut().expect("agents").push(first);
        let encoded = serde_json::to_vec(&value).expect("encoded");
        assert!(matches!(
            RegistryBrowser::parse(&encoded),
            Err(RegistryError::Invalid(message)) if message.contains("duplicate")
        ));
    }

    #[test]
    fn a_manifest_without_a_distribution_is_not_installable_metadata() {
        let raw = br#"{"version":"1","agents":[{"id":"empty","name":"Empty","version":"1","description":"No distribution","distribution":{}}]}"#;
        assert!(matches!(
            RegistryBrowser::parse(raw),
            Err(RegistryError::Invalid(message)) if message.contains("no distribution")
        ));
    }
}
