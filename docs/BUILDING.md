<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Building Cybou

Use Linux or WSL2 with Nix. Do not install a separate Windows Qt SDK for Linux/NixOS builds.

```bash
nix develop
cmake -S mind -B build/dev -G Ninja -DBUILD_TESTING=ON
cmake --build build/dev
ctest --test-dir build/dev --output-on-failure
```

Build packages:

```bash
nix build .#packages.x86_64-linux.cybou-mind
nix build .#packages.x86_64-linux.cybou-presence-applet
```

Build the VM:

```bash
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm
./result/bin/run-cybou-vm
```

Full validation:

```bash
nix flake check --print-build-logs
reuse lint
git diff --check
```

Clean local outputs:

```bash
rm -rf build/dev result result-*
```
