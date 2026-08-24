<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0034: Governed Agents, Workers, and Tool Use

## Status

Proposed

## Context

Agent-native computing introduces actors that can call powerful tools. Prompt injection, model
error, compromised dependencies, and malicious external content must be expected. The security
contract cannot be "the model was instructed not to do that".

## Decision

### Mind is persistent; execution actors are replaceable

```text
Faculty
  replaceable ability: language, vision, planning, code analysis

Worker
  temporary actor created for one bounded task

Agent
  longer-lived actor responsible for a continuing domain or intention

Mind
  persistent owner of continuity, evidence relationships, policy boundaries,
  and durable cognitive state
```

```text
agent ≠ Mind
worker ≠ Mind
faculty ≠ authority
```

### Every managed actor has an identity and lifecycle

A worker/agent has explicit runtime identity and attributable lifecycle state. Its identity must be
sufficient to attribute context delivery, tool use, grants, network activity, outcomes, and failures.

### Workers receive task-scoped grants

A worker envelope may include task/intention, allowed context, capabilities, MCP/tool methods,
resource scope, network destinations, credential handles, TTL, compute budget, retention permission,
and delegation permission.

The default is no authority beyond what the task explicitly grants.

### Agents do not own the state of their domain

A DevelopmentAgent may serve software work. An InfrastructureAgent may serve system health. Their
hidden context is not canonical domain state.

### Managed tool access is brokered

```text
Agent / Worker
      ↓
Tool / MCP Broker
      ↓
policy + capability grant
      ↓
server / method / resource
```

Raw model-to-MCP or model-to-tool access is not the intended managed path.

### Tool discovery is not authorization

```text
tool exists        ≠ actor may invoke it
MCP server online  ≠ actor may call every method
credential exists  ≠ actor may receive it
```

### Prompt content is untrusted input

Text/data consumed by an actor cannot grant capabilities.

### Actor actions cross ADR-0022

A task grant may allow proposals or bounded action capabilities. Mutations return through
observation/outcome.

### Delegation is explicit

An actor may not spawn another privileged actor or delegate its grant unless policy explicitly
permits delegation. Derived grants can only narrow.

### Amendment: the unit of grant is a capsule, and the agents are other people's (2026-08-24)

This ADR decides that actors have identities, receive task-scoped grants, and cross ADR-0022 to
affect anything. It leaves two questions open, and
[ADR-0042](ADR-0042-agent-capsule-platform.md) settles both.

**Where a grant applies.** Per command is unworkable — a person asked to approve `npm install` for
the fifteenth time is a person who has stopped reading. The grant is a capsule: one profile, granted
once, inside which the agent is free, and at whose boundary a request becomes an `ActionProposal`.

```text
autonomous inside its capsule  ≠  autonomous on the host
```

**Who writes the agents.** Not this project. Cybou hosts agents that already exist and speak ACP,
and contributes the layer beneath them — sandbox, leases, secrets isolation, model routing,
behaviour observation, causal history. A Cybou operator agent for one machine is a later and
different thing.

One correction to how this ADR could be read: *prompt content is untrusted input* is necessary and
is not the enforcement. A capsule holds because the kernel holds it. Cognition explains a
containment; it never is one.

## Consequences

AI workers can be powerful without being root-equivalent.

Prompt injection becomes primarily a capability-boundary problem.

## Acceptance gates

| | Gate |
|---|---|
| **G1** | Replacing a worker does not lose the task's durable Mind history |
| **G2** | A worker cannot invoke a capability absent from its grant |
| **G3** | A worker cannot widen its own context/sensitivity ceiling |
| **G4** | Tool/MCP discovery does not grant invocation authority |
| **G5** | Prompt injection cannot exfiltrate a credential the worker was never granted |
| **G6** | Delegated grants only narrow and require explicit delegation permission |
| **G7** | Mutating tool calls cross ADR-0022 and return observed outcome |
| **G8** | Terminating an actor does not erase accepted evidence/open intentions |

## Alternatives Considered

### One permanent super-agent

Rejected.

### Give each worker shell access and rely on prompts

Rejected.

### Treat MCP server permissions as sufficient

Rejected.

## Related documents

- `ADR-0022-authorized-action-boundary.md`
- `ADR-0030-transparent-context-delivery.md`
- `ADR-0035-governed-model-brokerage.md`
- `ADR-0036-autonomous-security-control-plane.md`
