<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Installation and Evaluation

## Maturity warning

Cybou images are development artifacts unless a release explicitly states otherwise. Do not use a
development image as the only copy of important data. Continuity and the documented v0/v1
lifecycle-state migration are implemented and tested, but general in-place system-upgrade,
rollback, installer-migration, and stable-release compatibility guarantees are not.

## Recommended evaluation path

Use the development VM first:

```bash
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm --print-build-logs
./result/bin/run-cybou-vm
```

This evaluates the Plasma surface, the fourteen-process Mind package, D-Bus/systemd activation, and
persistent state without installing to a physical disk. Individual VM gates activate the service
subgraph required by their scenario; see [Testing](TESTING.md) for exact coverage. Development
login details are defined in `systems/vm.nix` and intentionally not duplicated here.

## Available image targets

| Target | Build output | Intended use |
|---|---|---|
| QEMU/KVM VM | `cybou-vm.config.system.build.vm` | development/evaluation and the KVM gates |

One target, and it is a test harness rather than a product. The live ISO, its Calamares installer,
and the Hyper-V image were removed: they installed NixOS, and [ADR-0038](adr/ADR-0038-rust-first-codebase.md)
makes Debian 13 the deployment target. Keeping an installer for a system nothing is aimed at would
have meant maintaining an install path no gate exercised and no release intended anyone to use.

There is currently no installable Cybou image. The Debian packaging that replaces it is tracked in
[Deployment](DEPLOYMENT.md); until it exists, evaluation means the VM above or the deployed web
preview, not an installation.

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
