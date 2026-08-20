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
gateway on loopback and exposes it through public HTTPS:

```bash
bash scripts/deploy-vps.sh
```

The public URLs are `https://vps-d0669a91.vps.ovh.net` and `https://51.255.46.58`; neither requires
a login. The IP endpoint uses a renewable Let's Encrypt `shortlived` certificate.

The gateway refuses to start in `public-preview` mode if the Journal holds anything above the
sensitivity this deployment permits, which is `CYBOU_PUBLISHABLE_SENSITIVITY` in the unit and
ordinary by default. That is not authentication; it is the tripwire that decides when authentication stops
being optional. Today every contribution is a fact about the machine and the surface serves it. The
first time a person records something of their own — a promise, an observation they asked to track —
the public surface stops instead of publishing it, and the choice becomes explicit rather than
forgotten.

This deployment currently permits sensitivity 1, and the reason is written in the unit: its Journal
predates the classification rule, so 1252 machine facts carry a Personal label a constant put there.
Those labels cannot be corrected in place — sensitivity is inside the canonical envelope the hash
chain covers, which is the Journal refusing to let its past be quietly rewritten, working exactly as
intended and against us here. The alternatives are to keep the deployment as it is, or to begin a
new biography; both are the owner's call, and neither is a code change.

**The public deployment serves the live Mind, without authentication.** This is a deliberate,
temporary decision by the owner: the point of the deployment is a working desktop anyone can look
at, and there is nothing on that host worth protecting yet. It is not an oversight and not a
default — it was chosen while authentication does not exist. Two things must land before that
stops being true: an authentication boundary, and a demo user on the Linux side so access can be
granted per request rather than to everyone. Until then, treat everything the deployed Mind
observes as public, and put nothing on that host you would not publish.

The gateway runs as a user unit inside the `cybou` user manager, on the same session bus as the
organs. That is what makes live data possible at all: as a system service it had no session bus and
could not reach Presence1, so it could only ever serve fixtures. The fixture source still exists
and is what `CYBOU_GATEWAY_FIXTURE=1` selects for development and tests.

The unit sets `CYBOU_SESSION_MODE=public-preview`; the gateway exposes that trust context as
`publicPreview` rather than falsely claiming `localDesktop` or authenticated `remoteBrowser`. The
mode names the trust that was established — none — and it keeps naming it honestly now that live
state is behind it.

`vps-checks.sh` and `deploy-vps.sh` always transfer the unfinished working tree, excluding local
build outputs. Builds never happen on Windows or WSL. The remote source root defaults to
`/home/debian/cybou-src`; the persistent Cargo cache defaults to `/home/debian/cybou-target`.
They may be overridden with `CYBOU_VPS_SRC` and `CYBOU_VPS_TARGET`.

## Safety boundary

- There is no NixOS conversion path. The retired `prepare-vps-nixos.sh` has been removed rather
  than left in the tree as a script whose only behaviour was to refuse to run.
- Deployment does not modify the bootloader or replace Debian.
- Package/service activation will be added as part of the Debian hard cutover.
- Public access is unauthenticated by decision, behind TLS, and currently shows live Mind state.
  The boundary that will replace it is authentication plus a demo user granted on request; until
  that exists, the deployed host must hold nothing its owner would not publish.
