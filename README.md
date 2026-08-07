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

M1, M2, and M3 are implemented.

The durable cognitive path is now:

```text
organs in Presence runtime
        │
        ▼
EventStore / EventClient
        │ Qt D-Bus Event1
        ▼
cybou-eventd
        │
        ▼
Journal v2
        │
        └── accepted after COMMIT
                 │
                 ▼
        Workspace / Presence
```

`cybou-eventd` is the normal production owner of `journal.db`. Identity, Intentions, Predictor,
SelfModel, Workspace, and Presence no longer depend on the SQLite Journal class; they depend on
the transport-neutral `EventStore` contract.

The remaining organs are still in-process inside `plasmashell`. Process isolation of those organs
and `presenced` is M4.

## Status

| Component | Status |
|---|---|
| NixOS / Plasma foundation | ✅ |
| Journal v2 | ✅ M2 |
| Shared Presence runtime / live Workspace | ✅ M1 |
| Stable `$XDG_STATE_HOME/cybou` state | ✅ |
| `cybou-eventd` | ✅ M3 |
| Event1 Qt D-Bus + versioned CBOR | ✅ M3 |
| Exclusive normal production Journal writer | ✅ M3 |
| Process-isolated remaining organs | ❌ M4 |
| Degraded process modes | ❌ M6 |
| Language faculty | ❌ M8 |
| Authorized OS action boundary | ❌ M9 |

## Build

```bash
nix build .#packages.x86_64-linux.cybou-mind --print-build-logs
```

The Mind package now includes `cybou-eventd`, its D-Bus activation file, the Presence QML module,
and eleven CTest suites.

VM:

```bash
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm --print-build-logs
./result/bin/run-cybou-vm
```

See `docs/CURRENT_STATE.md`, `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, and `docs/mind/`.
