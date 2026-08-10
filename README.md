<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

<div align="center">

![Cybou Logo](packages/horizon-assets/cybou-aperture.svg)

# Cybou

**A reproducible personal NixOS desktop with a persistent, typed cognitive runtime**

NixOS 26.05 · KDE Plasma 6 Wayland · C++20/Qt 6 · local-first · zero cloud dependency

</div>

## What Cybou is

Cybou is an experimental personal operating-system project built on NixOS and KDE Plasma. It
combines a reproducible desktop, the Cybou Horizon visual system, and **Mind**: a local runtime for
durable biography, identity continuity, commitments, prediction/calibration, self projection, and
bounded attention.

Mind is deliberately not a chatbot or a single AI agent. Language models are planned as optional,
replaceable faculties. They do not own identity, canonical memory, authorization, or privileged
execution.

The current repository implements the evaluated M1–M5 substrate, P6.1–P6.4 capability health,
RPC resilience and typed homeostatic observation, plus the first P6.5 capability-aware Presence
slice. It does **not** yet implement automatic capability-aware scheduling, a complete degraded UI,
a language model, distributed Mind, or authorized external agency. See
[Current State](docs/CURRENT_STATE.md) for the exact implementation boundary.

## Why this architecture

Most assistants collapse language, memory, identity, planning, and execution into one model
process. Cybou keeps those responsibilities explicit:

```text
model ≠ identity
UI ≠ Mind
attention ≠ biography
perception ≠ truth
confidence ≠ authorization
proposal ≠ permission to execute
consolidation ≠ rewriting history
```

This makes model replacement, process restart, degraded operation, privacy policy, and future
actions independently testable.

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

All nine Mind services are separate `systemd --user` D-Bus processes. `cybou-eventd` is the only
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
| Capability health, RPC resilience, typed homeostatic observation | Implemented — M6 P6.1–P6.4 |
| Capability-aware Presence and interruptible automatic scheduling | Implemented — M6 P6.5 complete |
| Partial-availability and recovery fault matrix | In progress — M6 P6.6 slices 1–3 |
| Grounded perception, epistemics, retention, distributed prototype | Planned — M7 |
| Optional replaceable language faculty | Planned — M8 |
| Policy-controlled authorized action boundary | Planned — M9 |

The milestone labels describe engineering capability, not consciousness or biological equivalence.

## Cognitive lifecycle

The implemented analogue of sleep is an explicit maintenance lifecycle rather than a biological
simulation or a central `sleepd`:

```text
Awake → Idle → Consolidating → Awake
           └→ Maintenance
failure   → Recovering
session   → Suspended
```

`cybou-lifecycled` orchestrates bounded, interruptible runs while existing organs retain state
ownership. Predictor and Workspace create evidence-linked, idempotent Event1 results, and completed
runs require an accepted terminal Outcome. Lifecycle projection and the reboot/split-commit
fault-injection matrix are implemented. See
[ADR-0024](docs/adr/ADR-0024-cognitive-lifecycle-and-consolidation.md).

## Build and test

Use Linux or WSL2 with Nix. The flake pins the complete toolchain and NixOS base.

Build the Mind runtime and Presence applet:

```bash
nix build .#packages.x86_64-linux.cybou-mind --print-build-logs
nix build .#packages.x86_64-linux.cybou-presence-applet --print-build-logs
```

Build and start the development VM:

```bash
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm --print-build-logs
./result/bin/run-cybou-vm
```

Run the fast repository gates:

```bash
nix build --print-build-logs \
  .#checks.x86_64-linux.formatting \
  .#checks.x86_64-linux.reuse \
  .#checks.x86_64-linux.package-metadata \
  .#checks.x86_64-linux.cognitive-docs \
  .#checks.x86_64-linux.mind-access \
  .#checks.x86_64-linux.qml-api \
  .#checks.x86_64-linux.ui-polish \
  .#packages.x86_64-linux.cybou-mind \
  .#packages.x86_64-linux.cybou-presence-applet

nix fmt
git diff --exit-code
```

The full `nix flake check` includes both the focused headless lifecycle reboot gate and the heavy
two-node Plasma VM smoke test. Build details and the direct CMake workflow are in
[Building](docs/BUILDING.md); test coverage is described in [Testing](docs/TESTING.md).

## Flake outputs

| Output | Purpose |
|---|---|
| `packages.x86_64-linux.cybou-mind` | Nine daemons plus the Presence QML proxy plugin |
| `packages.x86_64-linux.cybou-presence-applet` | Plasma Presence and access-handle packages |
| `packages.x86_64-linux.cybou-theme` | Combined Horizon Plasma theme |
| `nixosConfigurations.cybou-vm` | QEMU/KVM development VM |
| `nixosConfigurations.cybou-iso` | Live/installer ISO |
| `nixosConfigurations.cybou-hyperv` | Hyper-V development image |
| `checks.x86_64-linux.lifecycle-continuity` | Headless P2 identity/run reboot continuity gate |
| `checks.x86_64-linux.vm-smoke` | Full NixOS/Plasma service-graph smoke test |

Run `nix flake show` for the complete package list.

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
  [Next Engineering Steps](docs/NEXT_STEPS.md) → [Release](docs/RELEASE.md)
- **Review Mind contracts:** [Mind documentation](docs/mind/README.md)
- **Review trust boundaries:** [Security documentation](docs/security/README.md)
- **Review decisions:** [ADR index](docs/adr/README.md)

The complete navigation map is in the [documentation index](docs/README.md).

## Design principles

- Local-first operation and no required cloud service.
- Reproducible Nix builds and explicit state transitions.
- Durable state is accepted before it becomes visible.
- One canonical writer for cognitive history.
- UI, language, planning, authorization, and execution remain separate boundaries.
- Unknown, stale, inferred, and disputed state must remain distinguishable.
- Privacy includes retention and erasure, not only classification.
- External actions must return observed outcomes to cognition.

## Project maturity

Cybou is pre-release research and engineering software. Images are development artifacts unless a
specific release states otherwise. Do not use a development image as the only copy of important
data, and do not treat planned M5–M9 behavior as implemented.

## License

Code and most documentation are licensed under the MIT License. Design and visual assets may use
CC BY-SA 4.0 as declared by their SPDX metadata. The repository follows the REUSE specification;
see [LICENSES](LICENSES/) and [ADR-0007](docs/adr/ADR-0007-reuse-3.x-compliance.md).
