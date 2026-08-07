<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

<div align="center">

![Cybou Logo](packages/horizon-assets/cybou-aperture.svg)

# Cybou

**Smart Operating System based on NixOS with KDE Plasma**

</div>

## Current implementation position

After Package 06 passes its gates, M1 through M4 are implemented.

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

See `docs/CURRENT_STATE.md`, `docs/ARCHITECTURE.md`, and `docs/ROADMAP.md`.
