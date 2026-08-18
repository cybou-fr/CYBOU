<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0036: Autonomous Security and Operations Control Plane

## Status

Proposed

## Context

An agent-native operating system cannot require a person to supervise every process, network
connection, worker, credential, or remediation. Cybou's target is continuous operation while
keeping AI confidence separate from security authority.

## Decision

### Cybou maintains a continuous governed control loop

```text
Observe
  ↓
Assess
  ↓
Predict
  ↓
Decide
  ↓
Authorize
  ↓
Act
  ↓
Verify
  ↓
Learn
  └──────────↺
```

### Security and operations state are first-class concerns

The future control plane may govern firewall/network exposure, network egress/VPN/perimeter,
endpoint processes/persistence, service/package/configuration integrity, storage/backup health,
SSH/access, credentials, models, agents/workers, MCP/tool usage, and remote nodes.

Exact owners require later owner/wire decisions.

### Desired state and observed state stay separate

Policy is desired state. Perception is evidence. Remediation follows policy/evidence/authorization.

### Autonomy uses risk tiers

```text
L0 Observe
L1 Restrict
L2 Reversible remediation
L3 High-impact or destructive
```

L1/L2 may be permitted by standing policy. L3 requires stronger explicit authorization by default.

### Standing authorization is explicit policy

Unattended operation depends on durable standing policy, not inferred preference.

### Deterministic enforcement survives model loss

Firewall enforcement, credential boundaries, capability checks, tool/MCP restrictions, and
authorization policy must remain enforceable with AI models unavailable.

### Restriction may precede diagnosis

Urgent reversible containment may be authorized before full causal diagnosis, but containment is not
reported as root-cause resolution.

### Post-action verification is mandatory

Command/API success does not prove environmental outcome.

### Agents and workers are monitored subjects

Capability requests, denied attempts, tool/MCP use, network destinations, privilege expansion, and
policy conflicts may be part of security observation.

## Consequences

Cybou becomes a self-maintaining and self-defending environment rather than a passive assistant.

## Acceptance gates

| | Gate |
|---|---|
| **S1** | Standing policy can permit a bounded unattended defensive action |
| **S2** | The same action is refused without standing authorization |
| **S3** | Model unavailability does not disable baseline enforcement |
| **S4** | Prompt text cannot bypass firewall/tool/credential policy |
| **S5** | Reversible containment is not reported as final diagnosis |
| **S6** | Command success is not final until consequence is observed |
| **S7** | High-impact actions default to stronger authorization |
| **S8** | Actor/tool use is attributable to actor and grant |
| **S9** | Recovery/rollback failure remains explicit |

## Relationship to distributed operation

M7 may prove distributed continuity. A later milestone extends this control plane across nodes and a
network perimeter. Replication of Mind state and security authority remain separate questions.

## Alternatives Considered

### Alert-only security

Rejected as the long-term target.

### Unrestricted autonomous root agent

Rejected.

### Security entirely implemented by an LLM

Rejected.

## Related documents

- `ADR-0018-privacy-classification-and-replication.md`
- `ADR-0022-authorized-action-boundary.md`
- `ADR-0034-governed-agents-workers-and-tools.md`
- `ADR-0035-governed-model-brokerage.md`
