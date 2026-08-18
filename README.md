<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

<div align="center">

![Cybou Logo](packages/horizon-assets/cybou-aperture.svg)

# Cybou

**An experimental agent-native operating system with a persistent cognitive control plane**

NixOS 26.05 · KDE Plasma 6 Wayland · C++20/Qt 6 · local-first · no required cloud service

</div>

## What Cybou is

Cybou is an experimental operating-system project built on NixOS and KDE Plasma. Its long-term
target is not "Linux with a chatbot". It is an **agent-native computing environment** in which a
persistent **Mind** remembers and governs the system while models, agents, workers, tools, and user
interfaces remain replaceable.

Mind owns durable cognitive continuity: biography, identity, commitments, prediction/calibration,
epistemic state, context, learning, policy inputs, and the future authorization boundary. The target
extends that substrate into a cognitive **control plane** that can continuously observe, protect,
and maintain the machine even when no person is present.

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
confidence ≠ authorization
proposal ≠ permission to execute
command sent ≠ observed outcome
consolidation ≠ rewriting history
```

This makes model/agent replacement, process restart, degraded operation, disclosure, tool use, and
future autonomous actions independently testable.

## Current architecture

```text
Plasma/QML Presence proxy
          │ Presence1
          ▼
   cybou-presenced
     │    │    │
     │    │    ├── cybou-identityd
     │    │    ├── cybou-intentiond
     │    │    ├── cybou-predictord
     │    │    ├── cybou-selfd
     │    │    └── cybou-workspaced
     │    ├─────── cybou-lifecycled ── Lifecycle1
     │    ├─────── cybou-healthd ───── Health1
     │    │              │
     └────┴──────────────┤ Event1
                         ▼
                   cybou-eventd
                         │
                         ▼
                 SQLite Journal v2
```

All twelve Mind services are separate `systemd --user` D-Bus processes. `cybou-eventd` is the only
canonical Journal writer. The Plasma component is a remote projection/cache and cannot silently
become a second cognitive owner.

## Capability status

| Capability | Status |
|---|---|
| Accepted durable events and live Presence projection | Implemented — M1 |
| Journal v2 causal, privacy, hashing, and migration semantics | Implemented — M2 |
| Single canonical Journal writer (`cybou-eventd`) | Implemented — M3 |
| Process-isolated identity, intention, prediction, Self, Workspace, Presence | Implemented — M4 |
| Restart/reboot continuity and lifecycle/consolidation core | Evaluation complete — M5 |
| Capability health, RPC resilience, typed homeostatic observation | Implemented — M6 |
| Contribution origin bound to the calling executable | Implemented — P7.0 |
| Measured Journal scale budgets, paged replay, incremental verification | Implemented — P7.0 |
| Grounded local perception and epistemic projection | Implemented — M7 slices |
| Journal v3 commitments and crash-safe transitive erasure | Implemented — M7 slices |
| Sensitivity as a durable schema axis | Implemented — M7 slices |
| Associative context and transparent governed delivery | Implemented/advancing — ADR-0029/0030 |
| Distributed Mind prototype | Planned — M7 |
| Structured language and meaning boundary | Planned — M8 |
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

Use Linux or WSL2 with Nix. The flake pins the complete toolchain and NixOS base.

```bash
nix build .#packages.x86_64-linux.cybou-mind --print-build-logs
nix build .#packages.x86_64-linux.cybou-presence-applet --print-build-logs
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm --print-build-logs
```

Run the repository gates described in [Building](docs/BUILDING.md) and
[Testing](docs/TESTING.md). `CURRENT_STATE.md` remains the authority for demonstrated behavior.

## Repository map

```text
mind/       C++/Qt protocols, storage, IPC, organs, services, and tests
modules/    reusable NixOS modules
systems/    VM, ISO, and Hyper-V compositions
packages/   Mind, Plasma, Horizon, layout, and tool derivations
tests/      NixOS VM integration test
spec/       machine-readable visual design tokens
docs/       architecture, operations, security, and ADRs
scripts/    repository and package validators
www/        project website
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
- **Review trust boundaries:** [Security documentation](docs/security/README.md)
- **Review decisions:** [ADR index](docs/adr/README.md)

## Design principles

- Local-first operation and no required cloud service.
- Remote inference or external tools, when used, cross explicit policy and disclosure boundaries.
- Reproducible Nix builds and explicit state transitions.
- Durable state is accepted before it becomes visible.
- One canonical writer for cognitive history.
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
