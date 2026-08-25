#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# What a root service manager will hand an unprivileged service, and what Cybou therefore may not ask
# it for.
#
# This gate exists because the answer was not what the design assumed. `LoadCredential=` is read by
# the service manager, which is root, and it **follows symlinks**: a credential whose source is a
# symlink to a root-only `0600` file delivers that file's contents to a service running as nobody.
# Measured here rather than believed either way.
#
# That mattered because the agent gateway loaded its lease that way, out of a directory owned by the
# unprivileged user that writes leases into it. The two together — a cybou-owned launch directory and
# a root-read credential source — let cybou name any root-only file and have root read it out. The
# proxy master key this very unit is handed was among the reachable targets, which is the one secret
# the whole arrangement exists to keep out of cybou's reach.
#
# So there are two checks. The first establishes the fact, so nobody has to trust this comment. The
# second is the rule that follows from it: no unit in this repository may load a credential from a
# path an unprivileged user can replace.
#
# Exit 3 means this host has no root service manager to ask, which is a check that did not run rather
# than one that passed.
set -euo pipefail

cd "$(dirname "$0")/.."

not_run() {
    echo "==> credential boundary gate NOT RUN: $1" >&2
    exit 3
}

command -v systemd-run >/dev/null || not_run "systemd-run is not available"
[ "$(id -u)" -eq 0 ] || not_run "asking the system manager for a credential needs root"
systemctl is-system-running >/dev/null 2>&1 || not_run "there is no system service manager"
id nobody >/dev/null 2>&1 || not_run "there is no unprivileged user to hand a credential to"

WORK="$(mktemp -d)"
ROOT_ONLY="$WORK/root-only"
SOURCES="$WORK/sources"
trap 'rm -rf "$WORK"' EXIT
chmod 755 "$WORK"
mkdir -p "$SOURCES"
chmod 755 "$SOURCES"

MARKER="CYBOU-CREDENTIAL-BOUNDARY-MARKER"
printf '%s\n' "$MARKER" >"$ROOT_ONLY"
chmod 600 "$ROOT_ONLY"
chown root:root "$ROOT_ONLY"

printf '%s\n' "ORDINARY-BYTES" >"$SOURCES/plain"
chmod 644 "$SOURCES/plain"
ln -s "$ROOT_ONLY" "$SOURCES/pointing-elsewhere"

install -m 0755 fixtures/credential-probe.sh "$WORK/probe.sh"

probe() {
    systemd-run --collect --wait --pipe --quiet "--unit=cybou-credential-$1" \
        -p User=nobody -p "LoadCredential=lease:$2" "$WORK/probe.sh" 2>&1 || true
}

# One: a plain source loads, so a refusal below is about the symlink and not about this invocation.
plain="$(probe plain "$SOURCES/plain")"
grep -q 'ORDINARY-BYTES' <<<"$plain" || {
    echo "the control case did not load a credential at all:" >&2
    echo "$plain" >&2
    exit 3
}

# Two: the fact. Recorded whichever way it comes out, because the rule below is worth keeping either
# way and a gate that only ran on one answer would stop telling anybody anything if it changed.
followed="$(probe symlink "$SOURCES/pointing-elsewhere")"
if grep -q "$MARKER" <<<"$followed"; then
    echo "==> observed: LoadCredential follows symlinks; a root-only file reached an unprivileged service"
else
    echo "==> observed: LoadCredential did not follow the symlink on this systemd"
fi

# Three: the rule. Whatever this systemd does today, no unit here may load a credential from a path an
# unprivileged user can replace — because the answer above is a property of a version, and the rule is
# a property of the design.
#
# /etc/cybou is root-owned and cybou cannot put a symlink in it, so credentials sourced from there are
# fine. /run/cybou-agent-leases is written by the unprivileged session owner, so nothing may be
# credential-loaded out of it.
offending="$(grep -rn '^LoadCredential=' systemd/ | grep 'cybou-agent-leases' || true)"
if [ -n "$offending" ]; then
    echo "a credential is loaded from a directory an unprivileged user writes:" >&2
    echo "$offending" >&2
    exit 1
fi

# And the lease is still delivered, just not that way: the gateway is told a path and opens it itself,
# which is the same user reading a file it wrote.
grep -q '^Environment=CYBOU_AGENT_LEASE_FILE=/run/cybou-agent-leases/%i.lease' \
    systemd/system/cybou-agent-gateway@.service || {
    echo "the gateway is no longer told where its lease is" >&2
    exit 1
}

echo "=== Credential boundary gate passed: nothing root reads comes from a path cybou can replace ==="
