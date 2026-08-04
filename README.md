<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cybou

Implementation repository for **Cybou v0.1 — Visual Foundation**.

The specification package that governs this repository (vision, design system, packaging rules,
acceptance gates, ADRs) lives separately. `spec/` there is authoritative; prose in `docs/` mirrors
it, and on conflict the spec wins.

**Current phase:** Phase 0 — repository bootstrap.

## Build interface

```bash
nix fmt
nix flake check
nix build .#packages.x86_64-linux.cybou-theme
```

Phase 1 adds:

```bash
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm -o result-vm
./result-vm/bin/run-cybou-vm
```

**Always pass `-o`.** Without it every build overwrites the same `result` symlink, so running a
check after building the VM removes the runner and the next launch fails with “No such file or
directory”. One output link per artefact.

The VM writes `cybou.qcow2` into the working directory and reuses it; run it from a scratch
directory and delete that file for a clean boot. Under WSL the QEMU window reaches the Windows
desktop through WSLg. Reaching SDDM takes about a minute and a half.

The ISO is built here, in WSL — not on a hosted CI runner, whose disk is too small to make the
result trustworthy.

## Frozen decisions

| Decision | Value |
|---|---|
| Base | NixOS 26.05 stable |
| Desktop | KDE Plasma 6, Wayland, SDDM |
| Theme | Cybou Horizon, `org.cybou.horizon.desktop` |
| Installer | Calamares, upstream profile (ADR-0005) |
| `system.stateVersion` | `26.05` (ADR-0006) |
| Licence | `MIT` code, `CC-BY-SA-4.0` assets, REUSE 3.x (ADR-0007) |
| AI in v0.1 | none (ADR-0003) |

## Checks

`nix flake check` runs:

- `formatting` — `nixfmt --check` over the tree;
- `package-metadata` — `scripts/validate-packages.py`, which statically catches the Gate B
  failures (wrong metadata file name, `KPlugin.Id` not matching its directory, a layout script
  not named `org.kde.plasma.desktop-layout.js`, symlinks inside a package, malformed SVG,
  `TBD` licences) without needing a running Plasma session.
