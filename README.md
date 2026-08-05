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

Cybou is a smart operating system built on NixOS with KDE Plasma 6 desktop environment.

The core innovation is **Mind** — a cognitive engine that provides cognitive capabilities through isolated organs (identity, intention, prediction, self, workspace, presence).

---

## Project Status

| Component | Status |
|-----------|--------|
| **Phase** | Phase 0 — repository bootstrap |
| **C++ Mind** | ✅ Implemented and building |
| **Presence Applet** | ✅ Implemented and building |
| **Build Artifacts** | ✅ Cleaned from repository history |
| **CI** | ✅ Validates C++ compilation |
| **REUSE** | ✅ All files have SPDX headers |

---

## Quick Start

```bash
# Format
nix fmt

# Check
nix flake check

# Build theme
nix build .#packages.x86_64-linux.cybou-theme

# Build C++ packages
nix build .#packages.x86_64-linux.cybou-mind
nix build .#packages.x86_64-linux.cybou-presence-applet
```

---

## Architecture

### Mind — Cognitive Engine

- **identityd** — subject continuity across restarts
- **intentiond** — obligations derived from the journal
- **predictord** — forecasts joined to outcomes for measurable error
- **selfd** — self-assessment from measured facts only
- **workspaced** — bounded attention and coalitions over the journal
- **presenced** — the surface that shows what the journal holds

### Technology Stack

| Layer | Technology |
|-------|------------|
| OS | NixOS 26.05 (stable) |
| Desktop | KDE Plasma 6, Wayland, SDDM |
| Language | C++20 / Qt6 |
| Build | CMake + Ninja |
| License | MIT (code), CC-BY-SA-4.0 (assets) |

---

## Documentation

- **Specification**: separate repository (authoritative)
- **ADRs**: in the specification repository
- **Development**: see `docs/`

---

## License

- Code: [MIT](LICENSES/MIT.txt)
- Assets: [CC-BY-SA-4.0](LICENSES/CC-BY-SA-4.0.txt)
- Compliance: [REUSE 3.x](https://reuse.software/spec/)
