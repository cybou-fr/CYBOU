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

### Amendment: the request vocabulary is typed, and `NoModel` is answered per task (2026-08-23)

The first version of these types named a provider and a model as strings, carried a sensitivity
ceiling as prose, and had no way to say what was being asked, what came back, or which artifact
answered. Nothing consumed them, because no runtime existed to consume them. They are replaced by
`protocol::model` rather than migrated.

Four decisions are worth naming, because each closes a way this boundary is usually lost:

**A task is a closed set, versioned in its name.** An open `String` task would be a way to add an
input shape, an output shape and a `NoModel` answer all at once, without anybody reviewing any of
them.

**Every task answers for its own absence.** `ModelTask::without_a_model` is total, and returns one
of exactly two things: something deterministic already does this, or the feature is *absent*. Absent
is not degraded and not a stub returning something plausible — a semantic search that quietly falls
back to matching filenames answers a different question than the one asked. ADR-0021 says `NoModel`
is a configuration; this is where that stops being an intention.

**No output can assert or command.** There is no `ModelOutput` variant carrying a truth value, a
permission, a path, or a command to run. MB5 says model output cannot directly authorize mutation;
making the field absent is stronger than checking for it, because a check can be forgotten at one
call site and a missing field cannot.

**Attribution is by digest, not by name.** A family and a revision record what somebody intended to
install; only `artifact_sha256` — with the template version beside it, since the same weights under
a different template are a different thing to have asked — says what actually answered. MB4 is
otherwise satisfiable by a worker that loaded a different file than the manifest named.

A request also names the disclosure its input was drawn from, and the field is not optional. A model
is a named consumer under ADR-0030; a request that could omit this would be a way to hand a model
context nobody recorded handing it, which is MB1 defeated by an ergonomic default.

### Amendment: external agents get a second surface, not a wider one (2026-08-24)

The typed vocabulary above is right for Mind and wrong for an external agent under
[ADR-0042](ADR-0042-agent-capsule-platform.md), which speaks chat completions and always will.

Widening `ModelBroker1` to carry arbitrary prompts would destroy the property this ADR exists for: a
broker whose request is a string cannot match a task to a route, cannot refuse one capability while
permitting another, and cannot say which answer was degraded.

So [ADR-0043](ADR-0043-model-gateway-for-external-agents.md) adds a second surface beside this one.
Two request shapes, one set of provider workers, one policy, one cost ledger. What an agent receives
is an ephemeral token scoped to its capsule, its model class and its budget — never a provider
credential.

Nothing in this ADR changes.

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
