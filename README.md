<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

<div align="center">

![Cybou Logo](packages/horizon-assets/cybou-aperture.svg)

# Cybou

**Smart Operating System based on NixOS with KDE Plasma**

[![REUSE compliant](https://img.shields.io/badge/REUSE-compliant-green.svg)](https://reuse.software/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![NixOS 26.05](https://img.shields.io/badge/NixOS-26.05-blue.svg)](https://nixos.org/)

</div>

---

## About

Cybou is a smart operating-system project built on NixOS with KDE Plasma 6.

Its experimental core is **Mind**: a typed cognitive runtime prototype with identity, intentions,
prediction, self-model, bounded attention, an append-only Journal, and a Presence surface.

Current code deliberately remains an in-process prototype inside `plasmashell`; daemon-like source
directory names describe future process boundaries.

**Current implementation position:** M1 and M2 are complete for the in-process runtime. The next
architecture milestone is M3: make `cybou-eventd` the exclusive Journal owner without changing the
post-COMMIT accepted-contribution semantics established by M1.

---

## Project Status

| Component | Status |
|---|---|
| NixOS 26.05 / Plasma 6 foundation | ✅ Implemented |
| Cybou Horizon / right-side Mind Dock | ✅ Implemented |
| Cognitive protocol invariants | ✅ Implemented |
| Journal v2 | ✅ Implemented |
| Shared in-process Presence runtime | ✅ Implemented |
| Live Workspace accepted-contribution admission | ✅ Implemented |
| Stable Unix Mind state root `$XDG_STATE_HOME/cybou` | ✅ Implemented |
| Legacy host-derived state migration | ✅ Implemented, fail-closed |
| `cybou-eventd` exclusive Journal owner | ❌ Next milestone |
| Process-isolated organs / D-Bus fabric | ❌ Planned |
| Degraded process modes | ❌ Planned |
| Language-model faculty | ❌ Planned |
| Authorized OS action boundary | ❌ Planned |

---

## Quick Start

```bash
nix develop

nix build .#packages.x86_64-linux.cybou-mind --print-build-logs
nix build .#packages.x86_64-linux.cybou-presence-applet
nix build .#packages.x86_64-linux.cybou-layout-templates
nix build .#packages.x86_64-linux.cybou-tools

nix build .#nixosConfigurations.cybou-vm.config.system.build.vm
./result/bin/run-cybou-vm
```

---

## Current Architecture

```text
plasmashell
├── Presence surface ─┐
├── Presence surface ─┼── shared PresenceRuntime
└── Presence surface ─┘        ├── Journal v2
                               ├── Identity
                               ├── Intentions
                               ├── Predictor
                               ├── SelfModel
                               └── Workspace
```

Durable/live ordering:

```text
organ action
→ Journal validate + COMMIT
→ accepted(envelope, sequence)
→ Workspace admission
→ Presence notification
```

The default Unix Mind state is independent of the hosting UI process:

```text
$XDG_STATE_HOME/cybou
```

The Target architecture moves this runtime behind `eventd`, `presenced`, and other isolated
services.

See:

- [Current State](docs/CURRENT_STATE.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Roadmap](docs/ROADMAP.md)
- [Testing](docs/TESTING.md)
- [Mind docs](docs/mind/README.md)
- [ADRs](docs/adr/README.md)

---

## Technology Stack

| Layer | Technology |
|---|---|
| OS | NixOS 26.05 |
| Desktop | KDE Plasma 6 / Wayland |
| Mind | C++20 / Qt 6 |
| Persistence | SQLite Journal v2 |
| UI | QML / Plasma |
| Build | Nix flakes + CMake + Ninja |
| License | MIT code / CC-BY-SA-4.0 assets |

---

## License

- Code: [MIT](LICENSES/MIT.txt)
- Assets: [CC-BY-SA-4.0](LICENSES/CC-BY-SA-4.0.txt)
- Compliance: [REUSE 3.x](https://reuse.software/spec/)
