<!--
SPDX-FileCopyrightText: 2026 Stanislav Saveliev
SPDX-License-Identifier: MIT
-->

# Cybou

**Cybou** — calm, reproducible KDE Plasma desktop on NixOS.

This is the implementation repository for **Cybou v0.1 — Visual Foundation**.
The specification package (vision, design system, packaging rules, acceptance gates, ADRs) lives separately in its own repository. When in conflict, `spec/` from the specification repository is authoritative.

---

## Status

| Component | Status |
|-----------|--------|
| **Phase** | Phase 0 — repository bootstrap |
| **C++ Mind** | ✅ Implemented and building |
| **Presence Applet** | ✅ Implemented and building |
| **Build Artifacts** | ✅ Cleaned from repository history |
| **CI** | ✅ Validates C++ compilation on every push |
| **REUSE Compliance** | ✅ All source files have SPDX headers |

---

## Quick Start

### Prerequisites

- NixOS 26.05 (stable) or Nix on any Linux distribution
- flakes and nix-command experimental features enabled

### Basic Commands

```bash
# Format all Nix files
nix fmt

# Run all checks (formatting, REUSE, package metadata)
nix flake check

# Build the theme package
nix build .#packages.x86_64-linux.cybou-theme
```

### C++ Packages

```bash
# Build the Mind cognitive engine
nix build .#packages.x86_64-linux.cybou-mind

# Build the Presence applet (QML plugin for Plasma panel)
nix build .#packages.x86_64-linux.cybou-presence-applet
```

### Virtual Machine (Phase 1)

```bash
# Build the VM (requires KVM)
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm -o result-vm

# Run the VM
./result-vm/bin/run-cybou-vm
```

> **Note:** Always pass `-o` to `nix build`. Without it, every build overwrites the same `result` symlink, which can cause the runner to disappear after running checks.

---

## Architecture

### Core Components

| Component | Description | Language |
|-----------|-------------|----------|
| **cybou-mind** | Cognitive engine (Presence organs) | C++/Qt6 |
| **cybou-presence-applet** | Plasma panel applet showing Presence | C++/QML |
| **cybou-theme** | Complete desktop theme (colors, wallpapers, icons, styles) | Nix |
| **cybou-vm** | NixOS configuration with Cybou desktop | Nix |

### Mind Architecture

The Mind implements the **Presence** cognitive surface through isolated organs:

- **identityd** — continuity of the subject across restarts
- **intentiond** — obligations derived from the journal
- **predictord** — forecasts joined to outcomes for measurable error
- **selfd** — self-assessment from measured facts only
- **workspaced** — bounded attention and coalitions over the journal
- **presenced** — the surface that shows what the journal holds

Each organ is a separate C++ module that communicates through a shared journal.

---

## Development

### Building

All packages can be built individually:

```bash
nix build .#packages.x86_64-linux.<package-name>
```

Available packages:
- `cybou-theme` — Complete desktop theme
- `cybou-mind` — Cognitive engine
- `cybou-presence-applet` — Plasma panel applet
- `horizon-colors` — Color scheme
- `horizon-wallpaper` — Desktop wallpaper
- `horizon-global-theme` — Global theme
- `horizon-plasma-style` — Plasma style
- `horizon-aurorae` — Window decorations
- `horizon-sddm` — SDDM theme
- `cybou-tools` — Development tools
- `cybou-branding` — Branding assets

### Running Tests

C++ tests run automatically during package build:

```bash
nix build .#packages.x86_64-linux.cybou-mind
```

This builds the Mind and runs all unit tests via CTest with the `offscreen` Qt platform plugin.

### Continuous Integration

GitHub Actions runs the following on every push:

1. **Fast checks** (every push):
   - Nix formatting validation
   - REUSE license compliance
   - Package metadata validation
   - **C++ compilation** (cybou-mind, cybou-presence-applet)

2. **Full checks** (tags only):
   - Complete `nix flake check`
   - VM build (requires KVM)

---

## Technical Stack

| Layer | Technology |
|-------|------------|
| **OS** | NixOS 26.05 (stable) |
| **Desktop** | KDE Plasma 6 (Wayland) |
| **Display Manager** | SDDM |
| **Build System** | CMake + Ninja |
| **Language** | C++20 + Qt6 |
| **Package Manager** | Nix |
| **Installer** | Calamares |
| **License** | MIT (code), CC-BY-SA-4.0 (assets) |
| **Compliance** | REUSE 3.x |

---

## Frozen Decisions

These decisions are frozen for v0.1 and documented in ADRs:

| Decision | Value | ADR |
|----------|-------|-----|
| Base System | NixOS 26.05 stable | - |
| Desktop Environment | KDE Plasma 6, Wayland, SDDM | - |
| Theme Name | Cybou Horizon (`org.cybou.horizon.desktop`) | - |
| Installer | Calamares with upstream profile | ADR-0005 |
| State Version | `26.05` | ADR-0006 |
| Licensing | MIT code, CC-BY-SA-4.0 assets, REUSE 3.x | ADR-0007 |
| AI Usage | None in v0.1 | ADR-0003 |

---

## Project Structure

```
cybou/
├── .github/              # GitHub workflows
│   └── workflows/
│       └── checks.yml   # CI configuration
├── mind/                # C++ Mind implementation
│   ├── foundation/      # Core foundation (storage, journal)
│   ├── organs/          # Cognitive organs
│   │   ├── identityd/
│   │   ├── intentiond/
│   │   ├── predictord/
│   │   ├── selfd/
│   │   ├── workspaced/
│   │   └── presenced/
│   ├── protocol/        # Shared protocol library
│   ├── shell/           # Plasma shell integration
│   └── tests/           # Unit tests
├── packages/            # Nix packages
│   ├── cybou-mind/
│   ├── cybou-presence-applet/
│   ├── horizon-colors/
│   ├── horizon-wallpaper/
│   ├── horizon-assets/
│   ├── horizon-global-theme/
│   ├── horizon-plasma-style/
│   ├── horizon-aurorae/
│   ├── horizon-sddm/
│   └── cybou-tools/
├── systems/             # NixOS configurations
│   ├── vm.nix
│   ├── iso.nix
│   └── hyperv.nix
├── flake.nix            # Flake configuration
├── flake.lock           # Lock file
└── README.md            # This file
```

---

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run `nix fmt` and `nix flake check`
5. Commit your changes (`git commit -m 'feat: add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

### Commit Guidelines

- Use [Conventional Commits](https://www.conventionalcommits.org/) format
- Include SPDX license header in all source files
- Keep commits atomic and focused
- Reference ADRs and issues when applicable

---

## License

- **Code**: MIT License (see [LICENSES/MIT.txt](LICENSES/MIT.txt))
- **Assets**: CC-BY-SA-4.0 License (see [LICENSES/CC-BY-SA-4.0.txt](LICENSES/CC-BY-SA-4.0.txt))
- **Compliance**: REUSE 3.x specification

All source files include SPDX license identifiers. See [REUSE specification](https://reuse.software/spec/) for details.
