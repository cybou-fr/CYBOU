<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Release Process

## Versioning

Use semantic pre-release versions until the first stable release:

```text
v0.1.0-alpha.1
v0.1.0-alpha.2
```

## Release gates

- fast and full CI are green;
- all packages build from the tagged commit;
- VM and ISO smoke tests pass;
- checksums are generated;
- known limitations are documented;
- CHANGELOG is updated;
- persistent-state compatibility is stated;
- rollback instructions exist.
- `CURRENT_STATE.md` matches the tagged implementation;
- proposed lifecycle/epistemic behavior is not presented as shipped;
- documentation link/cognitive validators pass;
- any state migration has interruption and recovery evidence.

## Artifacts

A release may include:

```text
Cybou ISO
NixOS VM image
Hyper-V development image
SHA256SUMS
release notes
source archive
```

Never label a development image stable when Mind-state migrations or installation paths are unverified.

## Reproducible release outline

```bash
git status --short
nix build --print-build-logs \
  .#checks.x86_64-linux.formatting \
  .#checks.x86_64-linux.reuse \
  .#checks.x86_64-linux.package-metadata \
  .#packages.x86_64-linux.cybou-mind \
  .#packages.x86_64-linux.cybou-presence-applet
nix flake check --print-build-logs
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm
nix build .#nixosConfigurations.cybou-iso.config.system.build.isoImage
sha256sum result/iso/*.iso > SHA256SUMS
```

Run release commands from the clean tagged tree. Record the flake revision, artifact hashes, test
environment, known limitations, and whether persistent state can be upgraded or must be reset.

## Release notes must distinguish

- implemented capability from proposed architecture;
- package/VM evaluation from booted smoke evidence;
- supported migration from untested state reuse;
- local-first runtime from future optional language or network faculties.
