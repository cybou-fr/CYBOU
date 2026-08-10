<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Installation and Evaluation

## Maturity warning

Cybou images are development artifacts unless a release explicitly states otherwise. Do not use a
development image as the only copy of important data. M5 continuity and the documented v0/v1
lifecycle-state migration are implemented and tested, but general in-place system-upgrade,
rollback, installer-migration, and stable-release compatibility guarantees are not.

## Recommended evaluation path

Use the development VM first:

```bash
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm --print-build-logs
./result/bin/run-cybou-vm
```

This evaluates the Plasma surface, the nine-process Mind package, D-Bus/systemd activation, and
persistent state without installing to a physical disk. Individual VM gates activate the service
subgraph required by their scenario; see [Testing](TESTING.md) for exact coverage. Development
login details are defined in `systems/vm.nix` and intentionally not duplicated here.

## Available image targets

| Target | Build output | Intended use |
|---|---|---|
| QEMU/KVM VM | `cybou-vm.config.system.build.vm` | primary development/evaluation |
| Live ISO | `cybou-iso.config.system.build.isoImage` | installer/live-session testing |
| Hyper-V | `cybou-hyperv.config.system.build.hypervImage` | Windows/Hyper-V development |

Example ISO build:

```bash
nix build .#nixosConfigurations.cybou-iso.config.system.build.isoImage --print-build-logs
sha256sum result/iso/*.iso
```

## Before physical installation

- verify the checksum against the release record;
- confirm the exact target disk and maintain an independent backup;
- test UEFI, storage, network, graphics, suspend, and rollback in a live/VM environment;
- read release notes and persistent-state compatibility;
- confirm that the release includes the expected installer test evidence.

The Calamares profile inherits the upstream NixOS graphical installer with Cybou branding. Branding
does not imply that every hardware/install path has been validated by this project.

## After boot

For service diagnostics:

```bash
systemctl --user status cybou-presenced.service
systemctl --user status cybou-eventd.service
systemctl --user status cybou-healthd.service
systemctl --user status cybou-lifecycled.service
journalctl --user -u 'cybou-*' --since boot
```

See [Troubleshooting](TROUBLESHOOTING.md) and [Current State](CURRENT_STATE.md).
