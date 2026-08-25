// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Provider facts that admit they were observed at an instant and may stop being true.
//!
//! This crate contains no provider entries. It validates an external snapshot and derives routing
//! eligibility from fresh, source-backed observations. Route order remains operator policy.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use url::Url;

/// Current catalogue wire format.
pub const SCHEMA_VERSION: u16 = 1;

/// A provider's observed reachability through the configured proxy.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Availability {
    /// A bounded probe succeeded.
    Available,
    /// A bounded probe reached a definite failure.
    Unavailable,
    /// The observer could not establish either answer.
    Unknown,
}

/// Whether the observer found a route whose priced usage accrues no charge.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ZeroCostAccess {
    /// A zero-cost route was observed.
    Available,
    /// No zero-cost route was observed under the cited terms.
    Unavailable,
    /// The observer could not establish either answer.
    Unknown,
}

/// A condition a person should see before choosing a provider.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConditionKind {
    /// Provider terms permit some use of supplied data beyond serving the request.
    DataUse,
    /// Access requires a payment method even when the observed route is zero-cost.
    PaymentMethod,
    /// Access is restricted by geography.
    Regional,
    /// Access is subject to a quota material to the choice.
    Quota,
    /// A material condition outside the closed common vocabulary.
    Other,
}

/// One observed claim with an explicit expiry and evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Observation<T> {
    /// Observed answer.
    pub value: T,
    /// When the source was checked, in UTC.
    #[serde(with = "time::serde::rfc3339")]
    pub observed_at: OffsetDateTime,
    /// Last instant at which this observation may be used for routing.
    #[serde(with = "time::serde::rfc3339")]
    pub valid_until: OffsetDateTime,
    /// HTTPS evidence supporting the observation.
    pub evidence: Vec<Url>,
}

impl<T> Observation<T> {
    /// Whether the observation may still govern a route at `now`.
    #[must_use]
    pub fn freshness(&self, now: OffsetDateTime) -> Freshness {
        if now <= self.valid_until {
            Freshness::Current
        } else {
            Freshness::Stale
        }
    }
}

/// Whether an observation is still inside its declared validity window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Freshness {
    /// The observation may govern a route.
    Current,
    /// The observation remains displayable evidence but may not govern a route.
    Stale,
}

/// One source-backed warning.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderCondition {
    /// Closed warning category.
    pub kind: ConditionKind,
    /// Human-readable statement taken from the catalogue source.
    pub summary: String,
    /// When this condition was checked.
    #[serde(with = "time::serde::rfc3339")]
    pub observed_at: OffsetDateTime,
    /// Last instant at which the condition is considered current.
    #[serde(with = "time::serde::rfc3339")]
    pub valid_until: OffsetDateTime,
    /// HTTPS evidence for the warning.
    pub evidence: Vec<Url>,
}

impl ProviderCondition {
    /// Whether the warning is current. A stale warning is retained and labelled stale.
    #[must_use]
    pub fn freshness(&self, now: OffsetDateTime) -> Freshness {
        if now <= self.valid_until {
            Freshness::Current
        } else {
            Freshness::Stale
        }
    }
}

/// All volatile observations about one provider.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderEntry {
    /// Stable operator/catalogue identifier used by route policy.
    pub id: String,
    /// Display label supplied by the catalogue.
    pub name: String,
    /// Reachability observation.
    pub availability: Observation<Availability>,
    /// Zero-cost access observation, independent of reachability.
    pub zero_cost_access: Observation<ZeroCostAccess>,
    /// Material terms and limitations, current or stale.
    #[serde(default)]
    pub conditions: Vec<ProviderCondition>,
}

/// One external catalogue snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderCatalogue {
    /// Wire format version.
    pub schema_version: u16,
    /// Source-owned revision used for audit and cache replacement.
    pub revision: String,
    /// Validated provider observations. This crate supplies none by default.
    pub providers: Vec<ProviderEntry>,
}

impl ProviderCatalogue {
    /// Parse and validate an externally supplied catalogue snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`CatalogueError`] for malformed JSON, unsupported versions, duplicate or malformed
    /// provider identities, impossible timestamps, non-HTTPS evidence, or blank warnings.
    pub fn parse(bytes: &[u8], loaded_at: OffsetDateTime) -> Result<Self, CatalogueError> {
        let catalogue: Self = serde_json::from_slice(bytes)
            .map_err(|error| CatalogueError::Invalid(error.to_string()))?;
        catalogue.validate(loaded_at)?;
        Ok(catalogue)
    }

    /// An installation with no observed providers.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            revision: "empty".to_owned(),
            providers: Vec::new(),
        }
    }

    /// Find one provider observation by its stable identifier.
    #[must_use]
    pub fn provider(&self, id: &str) -> Option<&ProviderEntry> {
        self.providers.iter().find(|provider| provider.id == id)
    }

    /// Resolve an explicit preferred route and operator-ordered alternatives.
    ///
    /// The return type distinguishes the preferred route from a named fallback. Callers cannot
    /// accidentally report an alternative as the route that was originally requested.
    #[must_use]
    pub fn resolve(
        &self,
        preferred: &str,
        alternatives: &[String],
        require_zero_cost: bool,
        now: OffsetDateTime,
    ) -> Resolution {
        let preferred_eligibility = self.eligibility(preferred, require_zero_cost, now);
        if preferred_eligibility == Eligibility::Eligible {
            return Resolution::Preferred {
                provider: preferred.to_owned(),
            };
        }

        let alternative = alternatives.iter().find(|candidate| {
            candidate.as_str() != preferred
                && self.eligibility(candidate, require_zero_cost, now) == Eligibility::Eligible
        });
        match alternative {
            Some(alternative) => Resolution::NamedAlternative {
                preferred: preferred.to_owned(),
                preferred_eligibility,
                alternative: alternative.clone(),
            },
            None => Resolution::Absent {
                preferred: preferred.to_owned(),
                preferred_eligibility,
            },
        }
    }

    /// Why one named provider may or may not currently serve a route.
    #[must_use]
    pub fn eligibility(
        &self,
        provider: &str,
        require_zero_cost: bool,
        now: OffsetDateTime,
    ) -> Eligibility {
        let Some(entry) = self.provider(provider) else {
            return Eligibility::NotObserved;
        };
        if entry.availability.freshness(now) == Freshness::Stale {
            return Eligibility::AvailabilityStale;
        }
        match entry.availability.value {
            Availability::Unavailable => return Eligibility::Unavailable,
            Availability::Unknown => return Eligibility::AvailabilityUnknown,
            Availability::Available => {}
        }
        if require_zero_cost {
            if entry.zero_cost_access.freshness(now) == Freshness::Stale {
                return Eligibility::ZeroCostObservationStale;
            }
            match entry.zero_cost_access.value {
                ZeroCostAccess::Unavailable => return Eligibility::NotZeroCost,
                ZeroCostAccess::Unknown => return Eligibility::ZeroCostUnknown,
                ZeroCostAccess::Available => {}
            }
        }
        Eligibility::Eligible
    }

    fn validate(&self, loaded_at: OffsetDateTime) -> Result<(), CatalogueError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(CatalogueError::UnsupportedVersion(self.schema_version));
        }
        if self.revision.trim().is_empty() {
            return Err(CatalogueError::Invalid(
                "blank catalogue revision".to_owned(),
            ));
        }
        let mut ids = HashSet::with_capacity(self.providers.len());
        for provider in &self.providers {
            if !valid_id(&provider.id) || provider.name.trim().is_empty() {
                return Err(CatalogueError::Invalid(format!(
                    "provider '{}' has an invalid identity or blank name",
                    provider.id
                )));
            }
            if !ids.insert(provider.id.as_str()) {
                return Err(CatalogueError::Invalid(format!(
                    "duplicate provider '{}'",
                    provider.id
                )));
            }
            validate_observation(&provider.availability, loaded_at, &provider.id)?;
            validate_observation(&provider.zero_cost_access, loaded_at, &provider.id)?;
            for condition in &provider.conditions {
                if condition.summary.trim().is_empty() {
                    return Err(CatalogueError::Invalid(format!(
                        "provider '{}' has a blank condition",
                        provider.id
                    )));
                }
                validate_times_and_evidence(
                    condition.observed_at,
                    condition.valid_until,
                    &condition.evidence,
                    loaded_at,
                    &provider.id,
                )?;
            }
        }
        Ok(())
    }
}

impl Default for ProviderCatalogue {
    fn default() -> Self {
        Self::empty()
    }
}

/// Why a provider is or is not eligible now.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Eligibility {
    /// Fresh observations satisfy the requested route conditions.
    Eligible,
    /// The catalogue contains no entry for this provider.
    NotObserved,
    /// Availability evidence expired.
    AvailabilityStale,
    /// A fresh probe found the provider unreachable.
    Unavailable,
    /// A fresh probe could not establish availability.
    AvailabilityUnknown,
    /// The zero-cost observation expired.
    ZeroCostObservationStale,
    /// A fresh observation found no zero-cost route.
    NotZeroCost,
    /// A fresh observation could not establish whether a zero-cost route exists.
    ZeroCostUnknown,
}

/// Result of resolving only explicitly named routes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Resolution {
    /// The requested provider is eligible.
    Preferred {
        /// Provider identifier.
        provider: String,
    },
    /// The requested provider was ineligible and an explicit alternative is eligible.
    NamedAlternative {
        /// Originally requested provider.
        preferred: String,
        /// Why it was not selected.
        preferred_eligibility: Eligibility,
        /// Operator-named alternative, in configured order.
        alternative: String,
    },
    /// Neither the preferred provider nor any named alternative is eligible.
    Absent {
        /// Originally requested provider.
        preferred: String,
        /// Why it was not selected.
        preferred_eligibility: Eligibility,
    },
}

/// Invalid external catalogue data.
#[derive(Debug, Error)]
pub enum CatalogueError {
    /// The source uses a format this build does not understand.
    #[error("unsupported provider catalogue schema {0}")]
    UnsupportedVersion(u16),
    /// The document is malformed or internally inconsistent.
    #[error("invalid provider catalogue: {0}")]
    Invalid(String),
}

fn validate_observation<T>(
    observation: &Observation<T>,
    loaded_at: OffsetDateTime,
    provider: &str,
) -> Result<(), CatalogueError> {
    validate_times_and_evidence(
        observation.observed_at,
        observation.valid_until,
        &observation.evidence,
        loaded_at,
        provider,
    )
}

fn validate_times_and_evidence(
    observed_at: OffsetDateTime,
    valid_until: OffsetDateTime,
    evidence: &[Url],
    loaded_at: OffsetDateTime,
    provider: &str,
) -> Result<(), CatalogueError> {
    if observed_at.offset() != time::UtcOffset::UTC
        || valid_until.offset() != time::UtcOffset::UTC
        || observed_at > loaded_at
        || valid_until < observed_at
    {
        return Err(CatalogueError::Invalid(format!(
            "provider '{provider}' has impossible observation times"
        )));
    }
    if evidence.is_empty()
        || evidence.iter().any(|source| {
            source.scheme() != "https"
                || source.host_str().is_none()
                || !source.username().is_empty()
                || source.password().is_some()
        })
    {
        return Err(CatalogueError::Invalid(format!(
            "provider '{provider}' has missing or non-HTTPS evidence"
        )));
    }
    Ok(())
}

fn valid_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_lowercase()
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: OffsetDateTime = OffsetDateTime::UNIX_EPOCH;

    fn document() -> Vec<u8> {
        include_bytes!("../../../fixtures/provider-catalogue-v1.example.json").to_vec()
    }

    #[test]
    fn unavailable_preference_becomes_only_a_named_configured_alternative() {
        let catalogue = ProviderCatalogue::parse(&document(), NOW).expect("valid example");
        assert_eq!(
            catalogue.resolve(
                "preferred.invalid",
                &["alternative.invalid".to_owned()],
                true,
                NOW,
            ),
            Resolution::NamedAlternative {
                preferred: "preferred.invalid".to_owned(),
                preferred_eligibility: Eligibility::Unavailable,
                alternative: "alternative.invalid".to_owned(),
            }
        );
        assert_eq!(catalogue.providers[1].conditions.len(), 1);
    }

    #[test]
    fn stale_observation_remains_visible_but_cannot_select_a_route() {
        let catalogue = ProviderCatalogue::parse(&document(), NOW).expect("valid example");
        let later = NOW + time::Duration::hours(2);
        assert_eq!(
            catalogue.providers[1].availability.freshness(later),
            Freshness::Stale
        );
        assert_eq!(
            catalogue.resolve("alternative.invalid", &[], false, later),
            Resolution::Absent {
                preferred: "alternative.invalid".to_owned(),
                preferred_eligibility: Eligibility::AvailabilityStale,
            }
        );
    }

    #[test]
    fn observations_without_https_evidence_are_refused() {
        let mut value: serde_json::Value = serde_json::from_slice(&document()).expect("JSON");
        value["providers"][0]["availability"]["evidence"] = serde_json::json!([]);
        let encoded = serde_json::to_vec(&value).expect("encoded");
        assert!(matches!(
            ProviderCatalogue::parse(&encoded, NOW),
            Err(CatalogueError::Invalid(message)) if message.contains("evidence")
        ));
    }

    #[test]
    fn observations_from_the_future_are_not_loaded_as_current_facts() {
        let mut value: serde_json::Value = serde_json::from_slice(&document()).expect("JSON");
        value["providers"][0]["availability"]["observedAt"] =
            serde_json::json!("1970-01-01T00:00:01Z");
        let encoded = serde_json::to_vec(&value).expect("encoded");
        assert!(matches!(
            ProviderCatalogue::parse(&encoded, NOW),
            Err(CatalogueError::Invalid(message)) if message.contains("times")
        ));
    }

    #[test]
    fn the_compiled_default_claims_nothing_about_any_provider() {
        let catalogue = ProviderCatalogue::default();
        assert!(catalogue.providers.is_empty());
        assert_eq!(
            catalogue.resolve("anything", &[], false, NOW),
            Resolution::Absent {
                preferred: "anything".to_owned(),
                preferred_eligibility: Eligibility::NotObserved,
            }
        );
    }
}
