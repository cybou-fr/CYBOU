<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Debian Build and Deployment

## Sole active environment

The only active Linux build and deployment environment is Debian 13 at
`debian@vps-d0669a91.vps.ovh.net`. Windows is an editing and Git workstation only. All release builds and deployments run on Debian 13.

The target is Debian 13, systemd, D-Bus, Wayland, Chromium, SQLite, libsodium, Rust, and WASM.
The host runs pure Debian 13 with native systemd user services.

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

The public URLs are `https://vps-d0669a91.vps.ovh.net` and `https://51.255.46.58`. Both require a
login: the gateway runs with `CYBOU_SESSION_MODE=sign-in-required`, refuses every projection route
with `401` until a session exists, and the page shows nothing but the way in. This was
`public-preview` until 2026-08-22 — a filtered projection of a live Mind, served to anyone with the
address. The IP endpoint uses a renewable Let's Encrypt `shortlived` certificate.

What a stranger receives is filtered rather than all-or-nothing. Beliefs and concepts carry the
sensitivity of the contributions they were derived from, obligations are withheld entirely because a
promise is about the person by construction, and everything above `CYBOU_PUBLISHABLE_SENSITIVITY` —
ordinary by default — is dropped before the projection is built. There is one filtered source and
every route reads it, so a route added later cannot forget: filtering is not something a route does.

A reader who signs in with a Linux account is served the unfiltered projection instead. The gateway
never checks the password itself — that means reading the shadow database, which it must not be able
to do. It asks `cybou-authd` over the socket named by `CYBOU_AUTH_SOCKET`. Failure reveals one
indistinguishable bit; success also returns the UID and home required to address that account's
unprivileged filesystem owner.

`cybou-authd` is the only Cybou process that runs as root. Its vocabulary is a name and secret in,
then either an indistinguishable refusal or the authenticated account's UID/home out. It cannot be
asked to read a file or run a command. A failed attempt is answered identically
whether the account is wrong, the password is wrong, or the account is not permitted, so the socket
is not a way to enumerate the host. The socket is group-owned by `cybou`, so only the gateway can
attempt a password at all; failures are held 750ms before answering.

Membership in the `cybou-access` group is the grant. Being a valid Linux account is not the same as
being someone this system answers to — without that gate, every service account on the host and
`root` would be a way in. The PAM stack in `/etc/pam.d/cybou` is ordinary Unix password checking
plus `account`, which is what makes `usermod -L` revoke access rather than appear to.

Granting and revoking are ordinary administration:

```bash
sudo useradd --no-create-home --shell /usr/sbin/nologin alice && sudo passwd alice && sudo gpasswd -a alice cybou-access
```

A session is a cookie: `HttpOnly`, `Secure`, `SameSite=Strict`, eight hours, no sliding renewal, and
held in memory so restarting the gateway ends every one. A missing, expired or invented cookie is
served the public projection rather than refused: a public surface that answered 401 to strangers
would stop being a public surface.

Authenticated access to a user's home is a separate capability. Install
`cybou-host-filesd@.service`, set `CYBOU_HOST_FILES_SOCKET_DIR=/run/cybou-host-files` on the web
gateway, and explicitly enable an owner only for admitted accounts:

```bash
sudo systemctl enable --now cybou-host-filesd@alice.service
```

The instance runs as `alice`, refuses root, and owns only that account's home. Its socket is
`/run/cybou-host-files/<uid>/owner.sock`; systemd grants the `cybou` gateway group traversal and
connect access only after the owner has bound it. If the instance or configuration is absent, the
host-file routes return `hostUserFilesystemUnavailable` rather than reading the home as the gateway
service account.

This replaced a tripwire that refused to start at all whenever the Journal held anything above
ordinary. It fired correctly on 2026-08-20, on the first sentence spoken to `Meaning1`, and took the
whole site down over rows it would never have shown — and the pressure to bring the site back is
what had earlier produced a raised threshold that outlived its reason and published a person's words
verbatim. Withholding the rows is the answer that survives being inconvenient.

This deployment permits ordinary only, the strict default. Its first Journal was discarded on
2026-08-20 rather than carried: 1252 rows written before the classification rule carried a Personal
label a constant had put there, and the labels could not be corrected in place, because sensitivity
sits inside the canonical envelope the hash chain covers. That is the Journal refusing to let its
past be quietly rewritten. Discarding a development biography of machine facts was the proportionate
answer; on a Journal that held anything a person cared about it would not have been, and the choice
would have been a migration that rebuilds rather than edits.

**The public deployment serves the live Mind, and what it serves publicly is machine facts.** That
is a deliberate decision by the owner: the point of the deployment is a working desktop anyone can
look at. It no longer requires the host to hold nothing personal — the surface withholds what is the
person's, and someone with an account reaches the rest.

What is still missing: nothing records who read what. Access can be granted and revoked per person,
and a person's reading leaves no trace in the biography, which for a system whose whole argument is
that claims should be traceable to a source is a gap worth naming rather than discovering later.

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

Agent1 is a deployed runtime but not a Mind organ, so deployment enables `cybou-agentd` explicitly
rather than attaching it to `cybou-mind.target`. The remediation worker is part of that target, and
the user services that use the session bus keep `/run/user` visible read-only while `/home` and
`/root` remain inaccessible. Deployment also restarts the system executor after replacing its
binary. A missing or malformed agent-capacity policy is repaired to explicit zero capacity; a valid
operator policy is preserved unchanged. Together these rules make a reboot and an in-place upgrade
start the same runtime without turning absent policy into permission.

## Safety boundary

- Deployments strictly target standard Debian 13 installations with native systemd user services.
- Deployment does not modify the bootloader or replace Debian.
- Package/service activation will be added as part of the Debian hard cutover.
- Public access is unauthenticated by decision, behind TLS, and shows live Mind state with
  everything above ordinary withheld. Signing in with an account in `cybou-access` reaches the rest.
- `cybou-authd` runs as root and is the only process that does. Its socket is group-owned by
  `cybou`; failures answer one bit and never say why an attempt failed.
- No record is kept of who read what.
