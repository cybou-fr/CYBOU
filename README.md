<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

<div align="center">

![Cybou Logo](packages/horizon-assets/cybou-aperture.svg)

# Cybou

**Smart Operating System based on NixOS with KDE Plasma**

</div>

## What Cybou Mind is

Cybou is built around a persistent cognitive runtime rather than around a language model.

Mind maintains typed durable biography, identity, commitments, prediction/calibration state,
self-projection, and bounded attention. Language models are planned as optional, replaceable
faculties: they may interpret or formulate information, but they do not become identity, canonical
memory, authorization authority, or privileged executor.

The Plasma Presence UI is only a projection into that runtime. Restarting or replacing a UI or
future language model must not, by itself, create a new identity or erase accepted biography.

The engineering model, its invariants, and the planned perception → cognition → authorized action
loop are described in [`docs/MIND_MODEL.md`](docs/MIND_MODEL.md).

## Current implementation position

The current tree contains the M1–M4 architecture:

```text
Plasma/QML
    │
    ▼
Presence QML proxy
    │ Presence1
    ▼
cybou-presenced
    ├── Identity1   ─► cybou-identityd
    ├── Intention1  ─► cybou-intentiond
    ├── Predictor1  ─► cybou-predictord
    ├── Self1       ─► cybou-selfd
    ├── Workspace1  ─► cybou-workspaced
    └── Event1      ─► cybou-eventd ─► Journal v2
```

The QML module no longer owns hidden mutable cognition. `plasmashell` contains only a Presence
proxy and visual state.

`docs/CURRENT_STATE.md` is authoritative for implemented behavior and current limitations.

## Status

| Component | Status |
|---|---|
| Journal v2 | ✅ M2 |
| accepted-event semantics | ✅ M1 |
| single-writer `cybou-eventd` | ✅ M3 |
| `cybou-identityd` | ✅ M4 |
| `cybou-intentiond` | ✅ M4 |
| `cybou-predictord` | ✅ M4 |
| `cybou-selfd` | ✅ M4 |
| `cybou-workspaced` | ✅ M4 |
| `cybou-presenced` | ✅ M4 |
| QML Presence as remote proxy only | ✅ M4 |
| explicit degraded-mode policy | ❌ M6 |
| distributed nodes | ❌ M7 |
| optional language faculty | ❌ M8 |
| authorized action boundary | ❌ M9 |

## Build

```bash
nix build .#packages.x86_64-linux.cybou-mind --print-build-logs
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm --print-build-logs
```

Start with:

- [`docs/MIND_MODEL.md`](docs/MIND_MODEL.md) — what the cognitive architecture means;
- [`docs/CURRENT_STATE.md`](docs/CURRENT_STATE.md) — what exists now;
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — how the current system is structured;
- [`docs/ROADMAP.md`](docs/ROADMAP.md) — how capabilities are intended to evolve.
