// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Governed Agents, Workers, Tools, and Model Brokerage (ADR-0034 & ADR-0035).
//!
//! Enforces process boundaries, task-scoped capability grants, brokered MCP/tool access,
//! and policy-aware inference routing without allowing external models or agents to become authority.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Classification of an autonomous execution actor.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActorKind {
    /// Replaceable stateless ability (language parsing, vision, planning).
    Faculty,
    /// Temporary short-lived actor created for one bounded task.
    Worker,
    /// Longer-lived actor responsible for a continuing domain or intention.
    Agent,
}

/// A task-scoped grant defining bounded permissions for a worker or agent.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskScope {
    /// Unique actor identifier.
    pub actor_id: Uuid,
    /// Actor kind.
    pub kind: ActorKind,
    /// Bound causal intention or task ID.
    pub intention_id: Option<Uuid>,
    /// Explicitly allowed capability keys.
    pub capabilities: Vec<String>,
    /// Permitted MCP/tool methods (e.g. `["git.status", "fs.readFile"]`).
    pub tool_grants: Vec<String>,
    /// Permitted network egress destination hosts.
    pub network_destinations: Vec<String>,
    /// Maximum lifetime in seconds before automatic revocation.
    pub ttl_seconds: u32,
    /// Maximum compute time budget in milliseconds.
    pub max_compute_ms: u64,
    /// Whether this actor is permitted to spawn subordinate workers.
    pub delegation_permitted: bool,
    /// Scope grant creation timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub granted_at: OffsetDateTime,
}

/// A structured proposal by an actor to invoke an external MCP tool or method.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallProposal {
    /// Unique tool call invocation identifier.
    pub call_id: Uuid,
    /// Calling actor ID.
    pub actor_id: Uuid,
    /// Target MCP server or capability provider name.
    pub mcp_server: String,
    /// Target method name.
    pub method: String,
    /// Method parameters (JSON or structured key-value).
    pub parameters: Vec<(String, String)>,
    /// Supporting evidence message IDs.
    pub evidence: Vec<Uuid>,
}

/// Broker authorization verdict for an MCP tool invocation.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolCallVerdict {
    /// Authorized within task scope and capability grant.
    Authorized,
    /// Refused by security boundary or policy constraint.
    Rejected {
        /// Reason for rejection.
        reason: String,
    },
}

/// Policy specification for model inference routing per ADR-0035.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InferenceRoute {
    /// Provider name (e.g. "local.llama", "remote.anthropic").
    pub provider: String,
    /// Model identifier.
    pub model_name: String,
    /// Whether the model operates across an external network boundary.
    pub is_remote: bool,
    /// Maximum permitted data sensitivity category allowed to reach this model.
    pub sensitivity_ceiling: String,
    /// Maximum allowable cost per request in millicents (1/1000th USD cent).
    pub cost_budget_millicents: u64,
}

/// Attributable model inference request crossing the named-consumer boundary.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInferenceRequest {
    /// Unique request identifier.
    pub request_id: Uuid,
    /// Selected routing configuration.
    pub route: InferenceRoute,
    /// Consumer/requester component name.
    pub consumer_name: String,
    /// Context delivery evidence IDs attached to this request.
    pub context_evidence: Vec<Uuid>,
    /// Request submission timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub requested_at: OffsetDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_scope_least_privilege_defaults() {
        let actor_id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();

        let scope = TaskScope {
            actor_id,
            kind: ActorKind::Worker,
            intention_id: None,
            capabilities: vec!["fs.read".into()],
            tool_grants: vec!["git.status".into()],
            network_destinations: vec![],
            ttl_seconds: 300,
            max_compute_ms: 5000,
            delegation_permitted: false,
            granted_at: now,
        };

        assert_eq!(scope.kind, ActorKind::Worker);
        assert!(!scope.delegation_permitted);
        assert_eq!(scope.ttl_seconds, 300);
    }
}
