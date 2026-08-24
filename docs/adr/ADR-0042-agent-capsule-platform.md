<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0042: Agent Capsules and the Agent Platform

## Status

Proposed

Written before implementation, deliberately. [ADR-0034](ADR-0034-governed-agents-workers-and-tools.md)
already decides that execution actors are replaceable and that Mind is not; this narrows that into a
buildable unit and settles the two questions it left open — *where does an agent's autonomy end*, and
*who writes the agents*.

## Context

Cybou has a persistent Mind, a Journal, bounded Body observation, a diagnosis layer, a proposal and
authorization boundary, and a browser surface. Each is defensible on its own. None of them is a
product a person can want, because a person does not want a control plane; they want work done on a
machine they own.

Meanwhile the coding-agent ecosystem solved the part Cybou has not: the agent loop, context
compaction, sub-agents, planning, tool calling, code editing, session restore, multimodal input, and
several hundred provider quirks. Several agents compete on exactly that and are open source.

What none of them solved is the part Cybou is built for. Every one of them is either asking a person
to approve `npm install` for the fifteenth time, or has been given enough authority that nobody is
approving anything. Both are the same failure: **authority is being decided per command, at the
moment of the command, by whoever is tired.**

### What the research found (2026-08-24)

Three facts changed what is worth building. They are recorded with a date because they are about a
moving ecosystem, and a decision that quietly rests on a stale fact is the failure this repository
keeps finding in itself.

- **ACP, the Agent Client Protocol, is a working standard with a registry.** Claude, Codex CLI,
  Gemini CLI, OpenCode, OpenHands, Cline, Goose, Mistral Vibe, Qwen Code, Copilot and Devin were
  reported as speaking it, and a stabilised registry exists so a client can find, install and
  configure agents from one catalogue.
- **MCP is host/client/server by design**, with the host owning permissions and lifecycle and
  servers offering constrained tools and resources.
- **A2A is under the Linux Foundation** and is the emerging agent-to-agent language across vendors.

None of this has been verified by this repository. It is the basis of a design decision, not a claim
about the world, and the design below is arranged so that being wrong about any of it costs an
adapter rather than an architecture.

## Decision

### Cybou hosts agents; it does not become one

Cybou does not write a general coding agent. Writing one means competing on the agent loop, and the
value Cybou has is entirely underneath it:

```text
                    portability
                    sandbox
                    persistent Mind
                    capability leases
                    secrets isolation
                    model routing
                    behaviour observation
                    causal history
                    outcome verification
```

A later Cybou agent is not excluded, and is described at the end of this document. It is a different
kind of thing from a coding agent and is not a prerequisite for any of this.

### The Agent Capsule is the unit of autonomy

An agent runs inside a capsule. The capsule is what is granted, observed, budgeted and destroyed —
not the agent, and not each command the agent runs.

```text
AgentCapsule
├── agent identity          which agent, which version
├── ACP endpoint            how Cybou speaks to it
├── workspace               the one directory it may change
├── process namespace       its own processes, and only its own
├── filesystem namespace    the host's filesystem is not in it
├── network namespace       an allow-list of destinations
├── model lease             a class of model and a spending ceiling
├── MCP grants              which tools, and which methods of them
├── resource budget         CPU, memory, wall-clock
├── secrets lease           handles, never provider credentials
├── lifetime                when this stops existing
└── audit and telemetry     what it did, continuously
```

### Autonomy inside the capsule; a proposal at its boundary

This is the decision the rest of the document exists to support.

```text
autonomous inside its capsule  ≠  autonomous on the host
```

A person grants a profile **once**. Inside it the agent reads, edits, compiles, runs tests, installs
dependencies, starts servers, fetches documentation, calls its model and spawns sub-agents, for
hours, without being asked anything.

The moment it reaches for something outside the capsule — restart a host service, change the
firewall, read a host key, publish a port — that is not a permission dialogue either. It is an
[ADR-0022](ADR-0022-authorized-action-boundary.md) `ActionProposal`, and it crosses the boundary
that already exists:

```text
agent
  → ActionProposal
  → criticism
  → standing policy
  → confirmation, or a grant already given
  → executor
  → independent re-observation
```

The existing action boundary was designed for Cybou's own remediation proposals. It fits an agent's
request without change, and that is the strongest evidence available that it was drawn in the right
place.

### The kernel enforces; cognition explains

Cybou must not be the thing that stops an agent. A capsule holds because the kernel holds it:

```text
namespaces · cgroups · seccomp · Landlock or AppArmor
mount policy · network policy · no-new-privileges · quotas
```

Above that, and only above it, Mind observes:

```text
telemetry → agent behaviour → SystemInsight → policy
```

The order matters and is not negotiable. A design where a model notices misbehaviour and asks the
agent to stop is a design with no boundary in it — the observer is the thing being observed, through
a channel the observed party controls. Cognition may **explain** a containment; it may never be the
containment.

What Cybou can then say, that a conventional endpoint agent cannot, is *why*:

```text
Person:  "update dependencies"
   → OpenCode task #17
      → npm install → registry.npmjs.org      expected
      → curl unknown.example                  not in this capsule's grant
```

That chain exists because Cybou holds the intention, the task, the agent, the model, the tool call,
the files changed and the destinations reached — and because they are one causal record rather than
four logs somebody correlates afterwards.

### Agents come from ACP and its registry, not from a Cybou catalogue

Cybou does not maintain installers for every agent. The registry is upstream; Cybou adds what the
registry does not have:

```text
ACP Registry  →  agent manifest  →  Cybou packaging  →  Agent Capsule
                                    trust
                                    sandbox profile
                                    model routing
                                    supervision
```

A Cybou-specific agent format would be a second catalogue to keep current forever, and would be
wrong within a quarter.

### Three protocols, three boundaries, kept apart

```text
ACP  Cybou ↔ agent        how an agent is driven and observed
MCP  agent ↔ tools        what an agent may reach, mediated by the host
A2A  agent ↔ agent        later; not a prerequisite for anything above
```

MCP capability grants are mediated by Cybou rather than configured inside the agent. An agent that
configures its own tool access is an agent that has granted itself capabilities, which
[ADR-0034](ADR-0034-governed-agents-workers-and-tools.md) already refuses.

### An agent never holds a provider credential

Covered in full by [ADR-0043](ADR-0043-model-gateway-for-external-agents.md). Stated here because it
is a property of the capsule: what the agent receives is an ephemeral token scoped to this capsule,
this model class, this budget and this lifetime. The provider secret stays where the agent cannot
reach it, so a compromised or misbehaving agent leaks a lease that expires rather than a key that
does not.

### One launch screen, and then silence

The user-facing consequence, stated normatively because it is the point of the whole design:

```text
Launch OpenCode
  Workspace   /projects/cybou
  Autonomy    ● sandboxed autonomous   ○ ask for every tool
  Allowed     workspace read/write · execute · install · GitHub ·
              package registries · selected model · development MCP tools
  Not allowed host filesystem · host services · firewall · SSH keys ·
              the Journal · other capsules
  Budget      4 GB · 2 CPU · 4 hours · a model spending ceiling
```

After `Launch`, **nothing is asked** while the agent stays inside those bounds. An interface that
then asks anyway has not made a weaker promise; it has made the grant meaningless, because a person
who is asked thirty times stops reading the thirty-first.

### An agent's own subscription is not an agent's authority

Some agents can authenticate to a model with a subscription the person already pays for. That is
permitted. It changes who pays for the tokens and nothing else:

```text
agent model ownership  ≠  agent execution authority
```

The capsule, the grants, the observation and the boundary are identical either way.

## Consequences

A person installs Cybou on a VPS, opens a browser, picks an agent, picks a model, picks a repository,
and presses Launch. What they get is an agent with a real Linux environment and no ability to reach
past it.

Cybou becomes describable in one sentence that is neither a control plane nor an assistant:

> A secure operating environment in which any AI agent can work almost autonomously, because the
> agent's freedom ends at a technically enforced boundary rather than at somebody's patience.

And the observation layer gains a subject it is unusually well suited to: not only Linux, but the
agents on it — what they tried, why, with which model, on what data, under which capabilities, and
what actually changed.

The cost is real. Cybou takes on a sandbox it must keep correct, an upstream protocol it does not
control, and a supervision surface that is worthless if it is ever wrong in the reassuring
direction.

## Acceptance gates

| | Gate |
|---|---|
| **G1** | A capsule's filesystem, process, and network isolation is enforced by the kernel and holds with Mind stopped |
| **G2** | An agent inside its grant completes a long task with no further human interaction |
| **G3** | An agent reaching outside its capsule produces an `ActionProposal`, never a silent success and never a silent failure |
| **G4** | An agent never receives a provider credential, only a scoped ephemeral token |
| **G5** | A capsule's resource and spending budgets are enforced, not advisory |
| **G6** | Behaviour outside a capsule's declared network grant is blocked by policy and explained afterwards, in that order |
| **G7** | Quarantine — freeze, revoke, disconnect — is available without Mind's participation |
| **G8** | Every capsule action is attributable to an agent, a task, a model, and an originating human intention |
| **G9** | Cybou's own capabilities, Journal, and other capsules are unreachable from inside a capsule |
| **G10** | An agent installed from the registry gains no capability the profile did not grant |

G6 is the one to defend hardest. A containment that depends on a model having noticed is not a
containment, and the sentence "the agent was asked to stop" is the shape of a boundary that does not
exist.

## Alternatives Considered

### Write a universal Cybou agent first

Rejected for now. It means writing the agent loop, context compaction, sub-agents, planning, tool
calling, code editing, session restore, MCP support, multimodal handling, prompt strategies and
several hundred provider quirks — all of which several funded open-source projects are already
competing on, and none of which is the thing Cybou is for.

### Ship a Cybou agent catalogue with its own manifest format

Rejected. A second catalogue is a permanent maintenance obligation against a moving ecosystem, and
the registry that already exists is the thing agents are publishing to.

### Rely on the agent's own sandboxing

Rejected. Some agents sandbox themselves, usually with Docker, and the recommended configuration
often involves handing them a Docker socket. A capsule that contains an agent holding the host's
Docker socket contains nothing. Where an agent expects to sandbox itself, it runs inside a capsule
that is already isolated and is not given the means to nest.

### Let Mind block misbehaviour

Rejected, and it is the most tempting of these. It puts the boundary inside the thing the boundary
exists to constrain, and makes containment depend on inference being right.

## Later: the Cybou Operator Agent

Not a coding agent, and not a prerequisite for anything above. A persistent agent belonging to one
machine, holding standing goals about it:

```text
Agent          PostgreSQL caretaker
Standing goal  PostgreSQL should remain available
Capabilities   inspect service · inspect logs · inspect storage ·
               restart under standing policy
Model          optional
Lifetime       persistent
```

This is where the persistent Mind is an advantage no session-scoped coding agent has, because the
question it answers is *what has been happening to this machine for the last month* — and it is worth
building only once the capsule, the boundary and the observation beneath it are real.

## Related documents

- [ADR-0022](ADR-0022-authorized-action-boundary.md) — the boundary an agent crosses to touch the host
- [ADR-0034](ADR-0034-governed-agents-workers-and-tools.md) — actor identity, grants, and tool mediation
- [ADR-0035](ADR-0035-governed-model-brokerage.md) — Mind's own typed model access
- [ADR-0043](ADR-0043-model-gateway-for-external-agents.md) — what an agent receives instead of a key
- [ADR-0036](ADR-0036-autonomous-security-control-plane.md) — the response layer capsule telemetry feeds
- [ADR-0041](ADR-0041-server-first-deployment.md) — why a server is the environment this is designed for
