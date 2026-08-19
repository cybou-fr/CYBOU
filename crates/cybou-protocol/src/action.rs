// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Authorized Action and Governed Remediation Boundary (ADR-0022 & ADR-0036).
//!
//! Separates action proposals, automated criticism checks, policy authorization,
//! and typed execution from unchecked model output.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Assessed risk category of an action proposal.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RiskLevel {
    /// Read-only inspection or reversible cosmetic change.
    Low,
    /// Service restart or bounded operational mutation.
    Medium,
    /// Package upgrade, configuration overwrite, or network firewall change.
    High,
    /// Irreversible deletion, disk formatting, or system shutdown.
    Critical,
}

/// A structured proposal for mutating external system state.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionProposal {
    /// Unique proposal identifier.
    pub proposal_id: Uuid,
    /// Causal intention or problem that gave rise to this proposal.
    pub cause_id: Option<Uuid>,
    /// High-level communicative intent.
    pub intent: String,
    /// Typed operation verb (e.g. "service.restart", "package.install", "firewall.block").
    pub operation: String,
    /// Target resource identifier.
    pub target_resource: String,
    /// Operation parameters and payload.
    pub parameters: Vec<(String, String)>,
    /// Evaluated risk level.
    pub risk_level: RiskLevel,
    /// Whether the action has an automated rollback mechanism.
    pub reversible: bool,
    /// Proposal creation timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub proposed_at: OffsetDateTime,
}

/// Result of an automated criticism rule evaluating a proposal before authorization.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CriticismCheck {
    /// Evaluation rule identifier.
    pub rule_id: String,
    /// Human-readable rule description.
    pub description: String,
    /// Whether the check passed without objection.
    pub passed: bool,
    /// Diagnostic objection or risk detail if check failed.
    pub objection: Option<String>,
}

/// Authorization verdict reached by the Mind governance engine.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthorizationVerdict {
    /// Pre-authorized for immediate execution under standing policy.
    Granted,
    /// Safe but exceeds automated authority; requires explicit user approval.
    RequiresUserConfirmation {
        /// User confirmation prompt.
        prompt: String,
    },
    /// Refused by policy, criticism check, or security boundary.
    Denied {
        /// Reason for refusal.
        reason: String,
    },
}

/// Final authorization decision record.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationDecision {
    /// Proposal being authorized.
    pub proposal_id: Uuid,
    /// Verdict.
    pub verdict: AuthorizationVerdict,
    /// Specific capability grants checked.
    pub checked_capabilities: Vec<String>,
    /// Instant when authorization decision was finalized.
    #[serde(with = "time::serde::rfc3339")]
    pub decided_at: OffsetDateTime,
}

/// Lifecycle status of an authorized execution attempt.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionStatus {
    /// Scheduled for execution.
    Pending,
    /// Currently being executed by the typed broker.
    Running,
    /// Completed successfully with observed outcome.
    Succeeded {
        /// Evidence message ID of the observed outcome.
        outcome_evidence_id: Uuid,
    },
    /// Failed during execution.
    Failed {
        /// Error message describing failure.
        error_message: String,
    },
    /// Failed and automated rollback was applied.
    RolledBack {
        /// Evidence message ID of the rollback outcome.
        rollback_evidence_id: Uuid,
    },
}

/// Execution attempt record binding proposal, authorization, and outcome evidence.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionAttempt {
    /// Unique execution attempt identifier.
    pub attempt_id: Uuid,
    /// Proposal that was executed.
    pub proposal_id: Uuid,
    /// Current status.
    pub status: ExecutionStatus,
    /// Execution start timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub started_at: OffsetDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_proposal_and_criticism_lifecycle() {
        let proposal = ActionProposal {
            proposal_id: Uuid::new_v4(),
            cause_id: None,
            intent: "Recover unresponsive database".into(),
            operation: "service.restart".into(),
            target_resource: "systemd:postgresql.service".into(),
            parameters: vec![("mode".into(), "graceful".into())],
            risk_level: RiskLevel::Medium,
            reversible: true,
            proposed_at: OffsetDateTime::now_utc(),
        };

        assert_eq!(proposal.risk_level, RiskLevel::Medium);
        assert!(proposal.reversible);

        let check = CriticismCheck {
            rule_id: "no-destructive-during-active-session".into(),
            description: "Verify no active transactions before restart".into(),
            passed: true,
            objection: None,
        };
        assert!(check.passed);
    }
}
