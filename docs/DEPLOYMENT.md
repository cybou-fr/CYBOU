<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Debian Build and Deployment

## Sole active environment

The only active Linux build and deployment environment is Debian 13 at
`debian@vps-d0669a91.vps.ovh.net`. Windows is an editing and Git workstation only. WSL and NixOS
are not valid build, test, packaging, or deployment evidence.

The target is Debian 13, systemd, D-Bus, Wayland, Chromium, SQLite, libsodium, Rust, and WASM.
Never attempt another in-place Debian-to-NixOS conversion on this host.

Bootstrap the clean server once:

```bash
bash scripts/bootstrap-debian-builder.sh
```

Push the current working tree and run normal gates remotely:

```bash
bash scripts/vps-checks.sh fast
bash scripts/vps-checks.sh release
```

Build and install the current release artifacts on that same host:

```bash
bash scripts/deploy-vps.sh
```

`vps-checks.sh` and `deploy-vps.sh` always transfer the unfinished working tree, excluding local
build outputs. Builds never happen on Windows or WSL. The remote source root defaults to
`/home/debian/cybou-src` and may be overridden with `CYBOU_VPS_SRC`.

## Safety boundary

- `scripts/prepare-vps-nixos.sh` remains permanently retired and refuses to run.
- Deployment does not modify the bootloader or replace Debian.
- Package/service activation will be added as part of the Debian hard cutover.
- Remote browser access requires a separately reviewed TLS and authentication boundary.
