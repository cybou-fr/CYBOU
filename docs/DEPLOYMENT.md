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

Build and install the current release artifacts on that same host. Deployment keeps the Rust
gateway on loopback and exposes the non-personal fixture preview through public HTTPS:

```bash
bash scripts/deploy-vps.sh
```

The public URLs are `https://vps-d0669a91.vps.ovh.net` and `https://51.255.46.58`; neither requires
a login. The IP endpoint uses a renewable Let's Encrypt `shortlived` certificate. Until the Rust
Presence owner is implemented, the public preview uses the explicit non-personal fixture
projection; it does not publish a Journal or live Mind state. Authentication must be restored
before any personal or live state is connected.
The systemd unit sets `CYBOU_SESSION_MODE=public-preview`; the gateway exposes that trust context as
`publicPreview` rather than falsely claiming `localDesktop` or authenticated `remoteBrowser`.

`vps-checks.sh` and `deploy-vps.sh` always transfer the unfinished working tree, excluding local
build outputs. Builds never happen on Windows or WSL. The remote source root defaults to
`/home/debian/cybou-src`; the persistent Cargo cache defaults to `/home/debian/cybou-target`.
They may be overridden with `CYBOU_VPS_SRC` and `CYBOU_VPS_TARGET`.

## Safety boundary

- `scripts/prepare-vps-nixos.sh` remains permanently retired and refuses to run.
- Deployment does not modify the bootloader or replace Debian.
- Package/service activation will be added as part of the Debian hard cutover.
- Public preview access is limited to deterministic non-personal fixtures behind TLS. Any personal
  or live state requires a separately reviewed authentication boundary before exposure.
