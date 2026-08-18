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
cargo check -p cybou-web-gateway --target x86_64-unknown-linux-gnu --locked
cd crates/living-canvas && trunk build --release
```

The pinned Rust 1.95 toolchain matches nixpkgs-26.05, and the WASM target is declared in
`rust-toolchain.toml`. On NixOS/Linux, build
the native reproducibility seam with:

```bash
nix build .#checks.x86_64-linux.rust-foundation
nix build .#packages.x86_64-linux.cybou-web-ui
nix build .#packages.x86_64-linux.cybou-web-gateway
nix build .#packages.x86_64-linux.cybou-desktop-shell
```

For a same-origin local integration run without D-Bus, build the frontend, set
`CYBOU_GATEWAY_FIXTURE=1`, point `CYBOU_WEB_ROOT` at the absolute `target/living-canvas` directory,
and run `cargo run -p cybou-web-gateway --locked`. Open `http://127.0.0.1:8787/`; the page and its
typed `/api/v1/*` reads are served by the same loopback process. Fixture mode is deterministic test
infrastructure, not a production fallback. On Linux without that variable, the gateway fails closed
unless it can connect to the existing user-session `Presence1` service.

The development VM exposes `Cybou Living Canvas` as a separate SDDM Wayland session. It starts the
same gateway/frontend closure under Cage and Chromium/Ozone; Plasma remains installed as the
fallback session during W2 evaluation:

```bash
nix build .#nixosConfigurations.cybou-vm.config.system.build.vm --print-build-logs
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

## WSL2 build and test environment

The active Linux/Nix environment is the local `NixOS` WSL2 distribution. The helper copies the
working tree from the Windows mount into a temporary Linux filesystem before invoking Nix:

```bash
wsl -d NixOS -- bash /mnt/c/Users/cybou/Documents/CYBOU/scripts/wsl-checks.sh fast
```

Use `full` only when `/dev/kvm` is available inside WSL2. The former OVH path and its in-place
Debian-to-NixOS conversion are retired after loss of SSH access; see [Deployment](DEPLOYMENT.md).

## Clean local outputs

```bash
rm -rf build/dev result result-*
```
