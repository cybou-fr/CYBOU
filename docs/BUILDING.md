<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Building

One toolchain builds everything that ships: Rust 1.95.0, pinned by `rust-toolchain.toml`. There is
no second build system. The C++/Qt implementation and the Nix packaging that used to live beside it
were removed on 2026-08-20; see [Current State](CURRENT_STATE.md) for why.

## Prerequisites

Debian 13 is the target and the only supported build host for the daemons, because they use a D-Bus
session bus and systemd user units. `scripts/bootstrap-debian-builder.sh` installs everything needed
on a fresh remote builder.

```bash
bash scripts/bootstrap-debian-builder.sh
```

A local Debian 13 is worth having, and on Windows a WSL image is enough:

```bash
wsl --install Debian
```

It runs the daemons and `dbus-run-session`, so the multi-daemon gate can be run there instead of
against a deployment. That matters more than convenience: the gate starts twelve owners and, since
it also exercises the guard that stops a public surface publishing personal state, running it
against the deployed host would mean proving that guard by making the deployment trip it.

The workspace itself is portable: `cargo test` and `cargo clippy` run on any platform the toolchain
supports, and that is what the GitHub workflow does. Everything behind `cfg(target_os = "linux")` —
the twelve daemons and their D-Bus surfaces — compiles only on Linux, so a green run elsewhere says
nothing about them.

## Build

```bash
cargo build --workspace --release --locked
```

The shared Rust/WASM frontend is built separately, because it targets the browser:

```bash
cargo install trunk --version 0.21.14 --locked
cd crates/living-canvas
trunk build --release
```

`trunk` writes to `target/living-canvas`, which is what `scripts/deploy-vps.sh` installs as the web
root. `cargo check -p living-canvas --target wasm32-unknown-unknown` is the cheap version of the
same question and is what CI runs on every push.

## Run the daemons locally

The daemons need a session bus. Running them under one keeps a development run from taking the
well-known names of a system that is already running:

```bash
dbus-run-session -- bash scripts/test-multi-daemon-integration.sh
```

That script starts all twelve, waits for each bus name, and asserts the properties listed in
[Testing](TESTING.md). It is the fastest way to see the whole Mind alive.

## What each artifact is

| Artifact | What it is |
|---|---|
| `cybou-eventd` | the single canonical Journal writer and `org.cybou.Mind.Event1` |
| the other eleven `cybou-*d` | one cognitive owner each, one D-Bus name each |
| `cybou-web-gateway` | read-only HTTP boundary, loopback only |
| `living-canvas` | the Rust/WASM frontend, shared by the browser and the desktop shell |
