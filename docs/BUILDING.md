<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Building Cybou

> The commands below describe the current C++/Qt implementation. The proposed target is one locked
> Rust workspace, including Rust/WASM Living Canvas and Rust replacements for all first-party
> executable code. See [Rust Migration Plan](RUST_MIGRATION.md) and
> [ADR-0038](adr/ADR-0038-rust-first-codebase.md).

## Rust foundation build

The R0/W0 workspace is additive: it does not replace the currently installed C++ owners.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check -p living-canvas --target wasm32-unknown-unknown --locked
```

The pinned toolchain and WASM target are declared in `rust-toolchain.toml`. On NixOS/Linux, build
the native reproducibility seam with:

```bash
nix build .#checks.x86_64-linux.rust-foundation
```

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
  .#checks.x86_64-linux.cognitive-docs \
  .#checks.x86_64-linux.mind-access \
  .#checks.x86_64-linux.qml-api \
  .#checks.x86_64-linux.ui-polish \
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

`nix flake check` includes the focused headless lifecycle reboot gate and the heavy two-node Plasma
VM smoke check. A normal GitHub push does not run that full matrix; the workflow reserves it for
the tag-only full job.

## Remote build and test host

A Windows workstation cannot build a NixOS closure or run a KVM-backed VM test. The OVH host
described in [Deployment](DEPLOYMENT.md) does both, from the working tree rather than from `HEAD`:

```bash
scripts/deploy-vps.sh switch
scripts/vps-checks.sh fast
```

`scripts/vps-checks.sh full` adds the four NixOS VM gates, which the host can run because
`/dev/kvm` is present. Preparing the machine itself is a separate, staged procedure in the same
document.

## Clean local outputs

```bash
rm -rf build/dev result result-*
```
