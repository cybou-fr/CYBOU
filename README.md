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

Its experimental core is **Mind**: a typed cognitive runtime prototype with identity,
intentions, prediction, self-model, bounded attention, an append-only Journal, and a Presence
surface.

The repository deliberately distinguishes **Current** behavior from **Target** architecture.
Directory names such as `identityd`, `predictord`, or `presenced` describe future process
boundaries; today those components are C++ objects and libraries loaded in one Presence object
graph inside `plasmashell`.

**Current implementation position:** Journal v2 and protocol invariants are implemented. The
next runtime work is to complete M1 (one Presence backend, live Workspace admission, stable state
ownership) before extracting `cybou-eventd`.

---

## Project Status

Status snapshot: 2026-08-07, based on `main` at
`0ce6074e8b4b24ded982cfccec1572db0e8f397a`.

| Component | Status |
|---|---|
| NixOS 26.05 / Plasma 6 foundation | ✅ Implemented |
| Cybou Horizon branding | ✅ Implemented |
| Presence applet and right-side Mind Dock | ✅ Implemented |
| C++ Mind object graph | ✅ Implemented in-process |
| Cognitive protocol invariants | ✅ Implemented for new v2 contributions |
| Journal v2 | ✅ Implemented and tested |
| v1 → v2 Journal migration | ✅ Implemented with retained backup |
| Fast push CI gates | ✅ Green on this snapshot |
| REUSE 3.x compliance | ✅ Green |
| One Presence backend per session | ⚠️ Not yet enforced |
| Live Workspace for every accepted contribution | ⚠️ Not yet complete |
| Stable `$XDG_STATE_HOME/cybou` Mind state | ⚠️ Not yet implemented |
| `cybou-eventd` single Journal owner | ❌ Not implemented |
| Process-isolated organs and D-Bus fabric | ❌ Not implemented |
| Language-model faculty | ❌ Not implemented |
| Authorized OS mutation boundary | ❌ Not implemented |

The normal push workflow runs the fast gates. The heavier `nix flake check` / VM path is a
separate tag-only job and must not be inferred as having run from a green ordinary push.

---

## Quick Start

```bash
# Format Nix files
nix fmt

# Enter the pinned development environment
nix develop

# Build and test the C++ Mind package
nix build .#packages.x86_64-linux.cybou-mind --print-build-logs

# Build Plasma/UI packages used by the current desktop
nix build .#packages.x86_64-linux.cybou-presence-applet
nix build .#packages.x86_64-linux.cybou-layout-templates
nix build .#packages.x86_64-linux.cybou-tools

# Build the development VM
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm
./result/bin/run-cybou-vm
```

For the full validation matrix, see [docs/TESTING.md](docs/TESTING.md).

---

## Architecture

### Current

```text
plasmashell
└── Presence QObject
    ├── Journal v2
    ├── Identity
    ├── Intentions
    ├── Predictor
    ├── SelfModel
    └── Workspace
```

The Plasma surface talks to Presence. The organs currently share the in-process Journal object
graph. Journal writes are serialized by SQLite `BEGIN IMMEDIATE`, but there is no `eventd`, no
stable D-Bus contract, and no process isolation yet.

### Target

```text
Plasma/QML
    │
    ▼
cybou-presenced
    │
    ▼
Typed cognitive fabric
    ├── cybou-eventd
    ├── cybou-identityd
    ├── cybou-intentiond
    ├── cybou-predictord
    ├── cybou-selfd
    └── cybou-workspaced
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and
[docs/CURRENT_STATE.md](docs/CURRENT_STATE.md).

### Technology Stack

| Layer | Technology |
|---|---|
| OS | NixOS 26.05 |
| Desktop | KDE Plasma 6, Wayland, SDDM |
| Mind | C++20 / Qt 6 |
| Persistence | SQLite Journal |
| UI | QML / Plasma |
| Build | Nix flakes + CMake + Ninja |
| License | MIT (code), CC-BY-SA-4.0 (assets) |

---

## Documentation

- **Current implementation:** `docs/CURRENT_STATE.md`
- **Architecture:** `docs/ARCHITECTURE.md`
- **Roadmap:** `docs/ROADMAP.md`
- **Mind:** `docs/mind/`
- **ADRs:** `docs/adr/`
- **Development:** `docs/`
- **Specification:** separate repository when explicitly referenced as authoritative

Documentation must not present a Target behavior as Current implementation.

---

## License

- Code: [MIT](LICENSES/MIT.txt)
- Assets: [CC-BY-SA-4.0](LICENSES/CC-BY-SA-4.0.txt)
- Compliance: [REUSE 3.x](https://reuse.software/spec/)
