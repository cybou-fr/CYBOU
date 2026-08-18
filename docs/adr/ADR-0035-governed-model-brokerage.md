<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0035: Governed Model Brokerage and External Inference

## Status

Proposed

## Context

Cybou should be able to use small local models, larger local models, specialist models, remote
providers, or multiple critics without making any one implementation the owner of cognition.

## Decision

### Model access is brokered

```text
faculty / planner / agent
        ↓
     Model Broker
        ↓
 ┌──────┼─────────┐
 ▼      ▼         ▼
local  local     remote
fast   strong    provider
```

No core organ calls an arbitrary provider as a hidden fallback.

### Routing is policy-aware

A route may consider capability/modality, model/provider identity/version, sensitivity/privacy
scope, external-boundary policy, latency, cost, compute/battery budget, network availability,
retention/training terms, and calibration history.

### Context crosses ADR-0030

Every model is a named consumer. A remote model is an external-boundary consumer. A local model is
not automatically trusted.

### Remote inference is optional capability, not continuity ownership

Loss of remote providers may reduce reasoning/language/research capacity but MUST NOT erase or
replace identity, biography, intentions, epistemic ownership, authorization policy, or minimum local
security/control state.

### Model output remains non-authoritative

```text
model says X      ≠ X is true
model wants A     ≠ A is authorized
model called tool ≠ outcome is achieved
```

### Multiple models may criticize each other

Agreement can support evaluation, but model consensus is not authority.

### Provider and cost use are attributable

Consequential use should be attributable enough to explain provider/model, route reason, disclosure
boundary, and cost/resource policy without duplicating raw prompt content into Journal.

## Consequences

Cybou can exploit remote inference without becoming cloud-dependent.

## Acceptance gates

| | Gate |
|---|---|
| **MB1** | A remote provider receives no context outside ADR-0030 |
| **MB2** | Disabling remote providers leaves identity/biography/policy intact |
| **MB3** | A local model cannot bypass consumer/sensitivity policy |
| **MB4** | Model/provider selection is attributable |
| **MB5** | Model output cannot directly authorize mutation |
| **MB6** | Provider failure becomes a capability deficit |
| **MB7** | Cost/network policy can refuse a route without corrupting Mind |

## Alternatives Considered

### One mandatory provider

Rejected.

### Permanent local-only inference rule

Rejected: local ownership and governed disclosure are the sovereignty boundary.

### Remote provider with direct memory retrieval

Rejected.

## Related documents

- `ADR-0021-language-models-are-optional-faculties.md`
- `ADR-0022-authorized-action-boundary.md`
- `ADR-0030-transparent-context-delivery.md`
- `ADR-0034-governed-agents-workers-and-tools.md`
