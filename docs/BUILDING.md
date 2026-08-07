<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Building Cybou

Use Linux or WSL2 with Nix. Do not install a separate Windows Qt SDK for Linux/NixOS builds.

## C++ development build

```bash
nix develop
cmake -S mind -B build/dev -G Ninja -DBUILD_TESTING=ON
cmake --build build/dev
ctest --test-dir build/dev --output-on-failure
```

## Nix packages

Core Mind and Presence:

```bash
nix build .#packages.x86_64-linux.cybou-mind --print-build-logs
nix build .#packages.x86_64-linux.cybou-presence-applet
```

Desktop integration used by the current VM:

```bash
nix build .#packages.x86_64-linux.cybou-tools
nix build .#packages.x86_64-linux.cybou-layout-templates
nix build .#packages.x86_64-linux.cybou-theme
```

## Development VM

```bash
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm --print-build-logs
./result/bin/run-cybou-vm
```

The development login is defined by `systems/vm.nix`; do not duplicate credentials in release
documentation.

## Fast CI-equivalent validation

```bash
nix build --print-build-logs \
  .#checks.x86_64-linux.formatting \
  .#checks.x86_64-linux.reuse \
  .#checks.x86_64-linux.package-metadata \
  .#packages.x86_64-linux.cybou-mind \
  .#packages.x86_64-linux.cybou-presence-applet

nix fmt
git diff --exit-code
```

## Full validation

```bash
nix flake check --print-build-logs
reuse lint
git diff --check
```

`nix flake check` includes the heavy VM smoke check. A normal GitHub push does not run that full
matrix; the workflow reserves it for the tag-only full job.

## Clean local outputs

```bash
rm -rf build/dev result result-*
```
