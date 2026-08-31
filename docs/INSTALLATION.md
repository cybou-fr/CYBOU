<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Installation and Evaluation

## Maturity warning

Cybou images and packages are development artifacts unless a release explicitly states otherwise. Do not use a
development preview as the only copy of important data. Continuity and the documented v0/v1
lifecycle-state migration are implemented and tested, but general in-place system-upgrade,
rollback, installer-migration, and stable-release compatibility guarantees are not.

## Recommended evaluation path

Build and run the workspace services locally on Debian 13 Linux or WSL2:

```bash
cargo build --workspace --release --locked
cargo test --workspace --locked
```

This evaluates the Living Canvas spatial surface, the fifteen-process Mind package, D-Bus/systemd activation, and
persistent state without installing to a physical disk. Individual test gates activate the service
subgraph required by their scenario; see [Testing](TESTING.md) for exact coverage.

## Available targets

| Target | Build command | Intended use |
|---|---|---|
| Cargo Workspace | `cargo build --workspace --release` | Local evaluation, development, and unit/integration test gates |
| Living Canvas WASM | `trunk build crates/living-canvas/index.html` | Client-side spatial desktop WebAssembly application |
| Web Gateway | `cargo run -p cybou-web-gateway` | Local HTTP/WebSocket gateway server on port 8080 |
| Debian VPS Deploy | `./scripts/deploy-vps.sh` | Remote deployment to a Debian 13 host with systemd user units |

There is currently no standalone desktop ISO. Deployment onto Debian 13 is tracked in
[Deployment](DEPLOYMENT.md).

## Service diagnostics

For systemd user service diagnostics:

```bash
systemctl --user status cybou-presenced.service
systemctl --user status cybou-eventd.service
systemctl --user status cybou-healthd.service
systemctl --user status cybou-lifecycled.service
journalctl --user -u 'cybou-*' --since boot
```

See [Troubleshooting](TROUBLESHOOTING.md) and [Current State](CURRENT_STATE.md).
