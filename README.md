<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

<div align="center">

![Cybou Logo](www/assets/cybou-aperture.svg)

# Cybou

**An experimental agent-native operating system with a persistent cognitive control plane**

Debian 13 · Rust/WebAssembly · one frontend for web and, as a target, desktop · local-first

</div>

## What Cybou is

Cybou is an experimental agent-native environment targeting Debian 13. The architecture builds one
Rust/WebAssembly frontend (Living Canvas) shared by ordinary browsers and, as a target, a lightweight
Chromium/Wayland desktop shell. The shell has no implementation in this tree: the Plasma packaging
that once stood in for it was removed with the rest of the C++/Nix legacy, and a Debian-native
launcher has not replaced it yet. Its long-term target is an **agent-native computing environment** in which a persistent
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

The current repository does **not** yet implement the future agent/worker runtime, model broker,
MCP/tool broker, general authorized action executor, firewall/endpoint controller, credential
broker, or autonomous remediation loop. `CURRENT_STATE.md` is authoritative for what exists today.

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
                           SQLite Journal v2
```

All thirteen Mind services are separate `systemd --user` D-Bus daemons written in Rust. `cybou-eventd`
is the only canonical Journal writer. Living Canvas is a pure read-model projection and user command
gateway that cannot become a second cognitive owner.

## Capability status

| Capability | Status |
|---|---|
| Accepted durable events and live Presence projection | Implemented — M1 |
| Journal v2 causal, privacy, hashing, and migration semantics | Implemented — M2 |
| Single canonical Journal writer (`cybou-eventd`) | Implemented — M3 |
| Process-isolated identity, intention, prediction, Self, Workspace, Presence | Implemented — M4 |
| Restart continuity and lifecycle/consolidation core | Implemented — M5 |
| Continuity across a real reboot | Implemented — gated on the deployed Debian host |
| Capability health, RPC resilience, typed homeostatic observation | Implemented — M6 |
| Contribution origin bound to the calling executable | Implemented — P7.0 |
| Measured Journal scale budgets, paged replay, incremental verification | Implemented — P7.0 |
| Grounded local perception and epistemic projection (`cybou-epistemicd`) | Implemented — ADR-0027 |
| Associative context live integration (`cybou-contextd`) | Partial — ADR-0029 |
| Journal v3 commitments and crash-safe transitive erasure | Implemented — M7 slices |
| Sensitivity as a durable schema axis | Implemented — M7 slices |
| Distributed Mind prototype | Planned — M7 |
| Structured language and meaning boundary | Partial — ADR-0031, no generative model |
| Lifelong learning and learned-artifact governance | Planned — M9 |
| Governed action and remediation boundary | Planned — M10 |
| Agent/worker runtime, model broker, and governed tool/MCP use | Planned — M11 |
| Continuous autonomous security and system operations | Planned — M12 |
| Distributed perimeter and multi-node governance | Planned — M13 |

The milestone labels describe engineering capability, not consciousness or biological equivalence.

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

Cybou remains local-first, but **local-only inference is not the product boundary**.

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
systemd/    User service definitions for the 12 Mind daemons
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

Cybou is pre-release research and engineering software. Do not treat planned M8–M13 behavior as
implemented. The current tree does not yet contain the general agent/worker runtime, model broker,
MCP governance layer, privileged security control plane, or unattended remediation engine described
by the future ADRs.

## Support and partnerships

Cybou is independently developed. Partnership enquiries and canonical contact information are
published at [cybou.fr](https://cybou.fr/).

For collaboration, hardware enablement, distribution work, or security contact:
[info@cybou.fr](mailto:info@cybou.fr).

## License

Code and most documentation are licensed under the MIT License. Design and visual assets may use
CC BY-SA 4.0 as declared by their SPDX metadata. The repository follows the REUSE specification;
see [LICENSES](LICENSES/) and [ADR-0007](docs/adr/ADR-0007-reuse-3.x-compliance.md).
