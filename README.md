<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

<div align="center">

![Cybou Logo](www/assets/cybou-aperture.svg)

# Cybou

**An experimental agent-native operating system with a persistent cognitive control plane**

Debian 13 · Rust/WebAssembly · server-side, reached through a browser · local-sufficient

</div>

## What Cybou is

Cybou is an experimental agent-native environment for a **server or container** running Debian 13 —
a machine that runs unattended and is reached remotely (ADR-0041). A personal workstation is a
supported place to run it and is not what it is for.

*Local-sufficient* rather than local-first: nothing Cybou needs to function is remote, and it is
built to be reached. The cognitive layer — biography, identity, epistemics, context, attention,
meaning, planning, disclosure — is deterministic, loads no model, needs no accelerator, and keeps
working with no network at all. A larger model may be consulted through an API as a governed
external-boundary consumer; it makes answers more fluent and lets Cybou attempt things it otherwise
cannot, and it is not what makes Cybou work. The architecture builds one
Rust/WebAssembly frontend (Living Canvas) shared by ordinary browsers and, as a target, a lightweight
Chromium/Wayland desktop shell. A Debian-native launcher now exists — Cage showing one Chromium
window over the loopback gateway, installed by a deployment and left disabled — but it has never
been run on a machine with a seat, so the desktop remains a target rather than something this tree
demonstrates. Cybou writes no compositor and no shell. Its long-term target is an **agent-native computing environment** in which a persistent
**Mind** remembers and governs the system while models, agents, workers, tools, and user interfaces
remain replaceable.

Debian 13 is the production and integration authority: the daemons, the multi-daemon gate and every
deployment run there, because they need a session bus and systemd user units. The portable half of
the workspace — everything that is not behind `cfg(target_os = "linux")` — is also checked on an
ordinary CI runner, which proves the code and nothing about the daemons. See
[Debian Build and Deployment](docs/DEPLOYMENT.md) and [Testing](docs/TESTING.md).

Mind owns durable cognitive continuity: biography, identity, commitments, prediction/calibration,
epistemic state, associative context, learning, policy inputs, and the future authorization boundary.
The target extends that substrate into a cognitive **control plane** that can continuously observe,
protect, and maintain the machine even when no person is present.

The intended hierarchy is:

```text
person / owner policy
        ↓
      Mind
        ↓
governed agents / workers / models
        ↓
capabilities / tools / executors
        ↓
       Body
```

A model is not Mind. An agent is not Mind. A worker is not Mind. MCP or another tool protocol is not
an authorization boundary.

Local and remote models may both be used in the future, but only through explicit context-delivery,
sensitivity, egress, cost, and capability policy. Cybou has no required cloud dependency: loss of
remote inference may reduce capability, but it must not erase identity, biography, policy, or the
minimum local control substrate.

The current repository does **not** yet implement the agent platform, the MCP/tool broker, the
authorized action executor, the firewall/endpoint controller, the credential broker, or the
autonomous remediation loop. The model broker exists as a faculty and has no worker registered
behind it, so it answers every request by saying what happens instead. `CURRENT_STATE.md` is
authoritative for what exists today.

Where this is going is decided rather than sketched.
[ADR-0042](docs/adr/ADR-0042-agent-capsule-platform.md) and
[ADR-0043](docs/adr/ADR-0043-model-gateway-for-external-agents.md) settle the next architecture: an
**Agent Capsule** is the unit of grant, an agent is autonomous inside it and produces an
`ActionProposal` at its boundary, the kernel enforces that boundary while cognition only explains it,
agents come from the public ACP registry rather than a catalogue of our own, and an agent receives a
capsule-scoped model lease instead of a provider credential. Cybou hosts agents; it does not become
one.

## Why this architecture

Most assistants collapse language, memory, identity, planning, tool use, and execution into one
model process. Cybou keeps those responsibilities explicit:

```text
model ≠ identity
agent ≠ Mind
worker ≠ authority
tool access ≠ permission
MCP availability ≠ authorization
UI ≠ Mind
attention ≠ biography
perception ≠ truth
observation ≠ knowledge
association ≠ truth
confidence ≠ authorization
proposal ≠ permission to execute
command sent ≠ observed outcome
consolidation ≠ rewriting history
```

This makes model/agent replacement, process restart, degraded operation, disclosure, tool use, and
future autonomous actions independently testable.

## Current architecture

```text
             Living Canvas (WASM / Wayland)
                           │ Presence1
                           ▼
                    cybou-presenced
                           │
                           ├── cybou-healthd (Health1)
                           ├── cybou-workspaced (Workspace1)
                           ├── cybou-contextd (Context1)
                           ├── cybou-epistemicd (Epistemic1)
                           ├── cybou-perceptiond (Perception1)
                           ├── cybou-intentiond (Intention1)
                           ├── cybou-predictord (Predictor1)
                           ├── cybou-selfd (Self1)
                           ├── cybou-identityd (Identity1)
                           └── cybou-lifecycled (Lifecycle1)
                                   │
                                   ▼
                             cybou-eventd (Event1)
                                   │
                                   ▼
                           SQLite Journal v3
```

All fourteen Mind services are separate `systemd --user` D-Bus daemons written in Rust. `cybou-eventd`
is the only canonical Journal writer. Living Canvas is a pure read-model projection and user command
gateway that cannot become a second cognitive owner.

## Capability status

| Capability | Status |
|---|---|
| Accepted durable events and live Presence projection | Implemented |
| Journal v2 causal, privacy, hashing, and migration semantics | Implemented |
| Single canonical Journal writer (`cybou-eventd`) | Implemented |
| Process-isolated identity, intention, prediction, Self, Workspace, Presence | Implemented |
| Restart continuity and lifecycle/consolidation core | Implemented |
| Continuity across a real reboot | Implemented — gated on the deployed Debian host |
| Capability health, RPC resilience, typed homeostatic observation | Implemented |
| Contribution origin bound to the calling executable | Implemented |
| Measured Journal scale budgets, paged replay, incremental verification | Implemented |
| Grounded local perception and epistemic projection (`cybou-epistemicd`) | Implemented — ADR-0027 |
| Associative context live integration (`cybou-contextd`) | Implemented — ADR-0029 |
| Activation from seeds that are not words, bounded and inspectable | Implemented — ADR-0029 A2, A12 |
| Journal v3 commitments and crash-safe transitive erasure | Implemented |
| Erasure a person can ask for, reaching what was derived | Implemented — ADR-0028, live-bus gated |
| A record of what was supplied to whom, and withheld | Implemented — ADR-0030 |
| An inspector for that record, answering for the caller alone | Implemented — ADR-0030 |
| A bounded history of what a consumer was supplied over time | Implemented — recent deliveries |
| Sensitivity as a durable schema axis | Implemented |
| Bounded transient Body telemetry, kept out of the biography | Implemented — ADR-0041 |
| Named things an operator declares: certificates, services, backups | Implemented |
| Findings as hypotheses carrying the readings behind them | Implemented |
| Where a watched thing is heading, and when it arrives | Implemented — robust statistics, no model |
| Structured language and meaning boundary | Implemented — ADR-0031, non-generative |
| Public surface withholds what is the person's | Implemented — filtered projection plus a credential |
| Governed action and remediation boundary | Implemented through three typed adapters and independent re-observation — ADR-0022 gate |
| Model brokerage as a faculty, not an organ | Implemented — ADR-0035; no worker registered |
| Agent capsules with kernel-enforced boundaries and brokered egress | Implemented — ADR-0042 gates |
| A model gateway and leases for agents that are not Mind | Planned — ADR-0043 |
| Distributed Mind prototype | Planned |
| Lifelong learning and learned-artifact governance | Planned |
| Governed tool and MCP mediation by the host | Planned — ADR-0034 |
| Continuous autonomous security and system operations | Planned |
| Distributed perimeter and multi-node governance | Planned |

These describe engineering capability, not consciousness or biological equivalence. The milestone
numbers live in [the roadmap](docs/ROADMAP.md) and nowhere else, because a number repeated in five
documents is a number that is wrong in four of them.

## Target agent-native control plane

```text
                         PERSON
                           │
                    goals / policy
                           │
                           ▼
                         MIND
                           │
          ┌────────────────┼─────────────────┐
          │                │                 │
       cognition        security         operations
          │                │                 │
          └────────────────┼─────────────────┘
                           ▼
                     CONTROL PLANE
                           │
        ┌──────────────────┼───────────────────┐
        ▼                  ▼                   ▼
      models             agents             workers
 local / remote      long-lived scope    task-scoped
        │                  │                   │
        └──────────────────┼───────────────────┘
                           ▼
                    CAPABILITY BROKER
                           │
        ┌──────────────────┼───────────────────┐
        ▼                  ▼                   ▼
   filesystem/process    network/security     tools/APIs
   services/packages     firewall/SSH/VPN     MCP/cloud
                           │
                           ▼
                          BODY
```

### Faculty, worker, agent

- **Faculty** — a replaceable ability such as language, vision, planning, or code analysis.
- **Worker** — a temporary actor created for one bounded task, with task-scoped context,
  capabilities, network access, resource budget, and lifetime.
- **Agent** — a longer-lived actor responsible for a continuing domain or intention.
- **Mind** — the persistent owner of continuity, evidence relationships, policy boundaries, and
  durable cognitive state.

### Continuous control

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

Autonomous does not mean unrestricted.

## Cognitive lifecycle

The implemented maintenance lifecycle remains the substrate for bounded background work:

```text
Awake → Idle → Consolidating → Awake
           └→ Maintenance
failure   → Recovering
session   → Suspended
```

Future agent/security work may use lifecycle and homeostatic signals for scheduling, but lifecycle
does not become the owner of security state, agents, or learned artifacts.

## Model policy

Cybou remains local-sufficient, but **local-only inference is not the product boundary**.

Future model use may include local and remote inference through a governed model broker. Context
must cross the same named-consumer delivery boundary as any other disclosure. External inference is
an external boundary; it may not silently retrieve Mind state or become the only place continuity
exists.

```text
core cognitive ownership and minimum control → local
optional inference capacity                  → local and/or remote
```

## Security direction

Cybou aims to govern the whole managed computing environment, including:

- firewall and network exposure;
- endpoint/process and persistence state;
- service/package integrity;
- SSH identities and access grants;
- credentials and delegated access;
- AI agents and task workers;
- local and remote model usage;
- MCP/tool/server access;
- network egress and cross-node trust.

The security substrate must not depend on persuading an AI model to behave.

## Build and test

Run standard workspace gates:

```bash
cargo check --workspace
cargo test --workspace
```

Run the repository gates described in [Building](docs/BUILDING.md) and
[Testing](docs/TESTING.md). `CURRENT_STATE.md` remains the authority for demonstrated behavior.

## Repository map

```text
crates/     Rust workspace: protocol, storage, crypto, runtime, fabric, daemons, living-canvas
systemd/    User service definitions for the Mind daemons, the model-broker faculty,
            the web gateway and the desktop session
spec/       Machine-readable visual design tokens
docs/       Architecture, operations, security, and ADRs
scripts/    Repository and package validators
www/        Project website
```

## Documentation

Choose a route:

- **Understand the idea:** [Mind Model](docs/MIND_MODEL.md) →
  [Architecture](docs/ARCHITECTURE.md) → [Roadmap](docs/ROADMAP.md)
- **Verify what exists:** [Current State](docs/CURRENT_STATE.md) →
  [Testing](docs/TESTING.md) → [Failure Modes](docs/mind/FAILURE_MODES.md)
- **Build or contribute:** [Building](docs/BUILDING.md) →
  [Development Workflow](docs/DEVELOPMENT_WORKFLOW.md) →
  [Next Engineering Steps](docs/NEXT_STEPS.md)
- **Deploy and test off the workstation:** [Deployment](docs/DEPLOYMENT.md)
- **Review trust boundaries:** [Security documentation](docs/security/README.md)
- **Review decisions:** [ADR index](docs/adr/README.md)

## Design principles

- Local-first operation and no required cloud service.
- Remote inference or external tools, when used, cross explicit policy and disclosure boundaries.
- Native Debian 13 deployment and deterministic Rust workspace.
- Durable state is accepted before it becomes visible.
- One canonical writer for cognitive history (`cybou-eventd`).
- UI, language, planning, authorization, execution, and security governance remain separate.
- Unknown, stale, inferred, and disputed state remain distinguishable.
- Privacy, sensitivity, retention, erasure, and egress are separate governed concerns.
- Agents, workers, models, and tool servers never acquire authority merely by being available.
- Managed MCP/tool access is capability-scoped rather than raw model-to-tool access.
- External actions return observed outcomes to cognition.
- Unattended autonomy is bounded by standing policy, risk, reversibility, and verification.

## Project maturity

Cybou is pre-release research and engineering software. Do not treat planned behaviour as
implemented. The current tree does not yet contain the general agent/worker runtime, MCP governance
layer, privileged security control plane, or unattended remediation engine described by the future
ADRs. The model broker exists and has no worker behind it.

## Support and partnerships

Cybou is independently developed. Partnership enquiries and canonical contact information are
published at [cybou.fr](https://cybou.fr/).

For collaboration, hardware enablement, distribution work, or security contact:
[info@cybou.fr](mailto:info@cybou.fr).

### Donations

| | Address |
|---|---|
| Solana (SOL) | `39iqkHNMqncEPp3p52zKwUHnYzk2MJbcaHyY4Hhg2fWC` |
| Bitcoin (BTC) | `bc1q5a0yq9kflu755jz9a7juveelj3lrnaml6cnjur` |
| Ethereum / USDT (ERC-20) | `0xf4B7fF998600617785ad7D4d0aad3D2Ea342526B` |
| TRON / USDT (TRC-20) | `TCWmbxJXwes4GLZkVjpKpY3p34mjg4C6qo` |


## License

Code and most documentation are licensed under the MIT License. Design and visual assets may use
CC BY-SA 4.0 as declared by their SPDX metadata. The repository follows the REUSE specification;
see [LICENSES](LICENSES/) and [ADR-0007](docs/adr/ADR-0007-reuse-3.x-compliance.md).
