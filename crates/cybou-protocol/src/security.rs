// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Autonomous Security and Operations Control Plane (ADR-0036).
//!
//! Enforces deterministic security policies, tiered autonomous response capabilities,
//! and monitored worker constraints that survive loss of AI model availability.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Autonomy tiers governing unattended operational intervention per ADR-0036.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AutonomyTier {
    /// L0: Passive telemetry observation and incident logging.
    L0Observe,
    /// L1: Immediate reversible containment (e.g. rate-limiting, blocking an egress IP).
    L1Restrict,
    /// L2: Automated bounded operational remediation with guaranteed rollback.
    L2ReversibleRemediation,
    /// L3: High-impact or destructive intervention requiring explicit user authority.
    L3HighImpact,
}

/// Security and operational domains under continuous surveillance.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SecurityDomain {
    /// Network perimeter, firewall rules, and open listening ports.
    NetworkFirewall,
    /// Host processes, persistence mechanisms, and systemd units.
    EndpointProcess,
    /// Package manager dependencies, binary hashes, and system files.
    PackageIntegrity,
    /// Secret keys, certificates, SSH grants, and credential access.
    CredentialAccess,
    /// Agent and worker egress destinations and tool call invocations.
    AgentWorkerEgress,
    /// Filesystem usage, disk health, and backup integrity.
    StorageHealth,
}

/// A durable standing policy rule authorizing unattended defensive action.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StandingPolicyRule {
    /// Unique policy rule identifier.
    pub rule_id: String,
    /// Target domain.
    pub domain: SecurityDomain,
    /// Maximum autonomy tier permitted without user confirmation.
    pub max_autonomy_tier: AutonomyTier,
    /// Whether explicit standing authorization grant is required in policy.
    pub requires_standing_authorization: bool,
    /// Rate limit: maximum interventions permitted per hour.
    pub rate_limit_per_hour: u32,
    /// Human-readable policy description.
    pub description: String,
}

/// A security incident record tracking containment and root-cause resolution.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityIncident {
    /// Unique incident identifier.
    pub incident_id: Uuid,
    /// Security domain involved.
    pub domain: SecurityDomain,
    /// Assessed severity description.
    pub severity: String,
    /// Evidence message IDs establishing the anomalous observation.
    pub observed_evidence: Vec<Uuid>,
    /// Whether immediate reversible containment has been applied.
    pub containment_applied: bool,
    /// Whether the true underlying root cause has been verified and settled.
    pub root_cause_settled: bool,
    /// Incident detection timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub detected_at: OffsetDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standing_policy_tier_bounds() {
        let rule = StandingPolicyRule {
            rule_id: "auto-block-bruteforce-ssh".into(),
            domain: SecurityDomain::NetworkFirewall,
            max_autonomy_tier: AutonomyTier::L1Restrict,
            requires_standing_authorization: true,
            rate_limit_per_hour: 10,
            description: "Automatically drop external IP after 5 failed SSH attempts".into(),
        };

        assert_eq!(rule.max_autonomy_tier, AutonomyTier::L1Restrict);
        assert!(rule.requires_standing_authorization);
    }
}
