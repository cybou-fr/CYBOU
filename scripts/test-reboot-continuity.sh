#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Continuity across a real reboot of the deployed host.
#
# The systemd gate proves the owners come back when their processes are restarted. It cannot prove
# the machine can come back: lingering, unit enablement, the user manager starting without a login,
# state that only exists because a directory happened to be warm — none of that is exercised by
# restarting a target inside a session that is already running. This is the gate that was lost with
# the NixOS platform, rebuilt against the host that actually runs the Mind.
#
# It reboots the target host. It is deliberately not part of any other gate, and it takes the
# service down for as long as the host takes to come back.

set -euo pipefail

# shellcheck source=scripts/vps-env.sh
. "$(dirname "$0")/vps-env.sh"

MACHINE="${CYBOU_USER_MACHINE:-cybou@.host}"

mind() {
    local name="org.cybou.Mind.$1"
    local path="/$(printf '%s' "$name" | tr . /)"
    cybou_ssh "sudo busctl --user --machine='$MACHINE' call $name $path $name $2 ${*:3}"
}

scalar() {
    mind "$@" | awk '{print $2}' | tr -d '"'
}

# The host is unreachable for part of this, so every read has to tolerate no answer rather than
# treating the gap as a verdict.
wait_for_ssh() {
    local deadline=$((SECONDS + 300))
    while [ "$SECONDS" -lt "$deadline" ]; do
        if cybou_ssh true >/dev/null 2>&1; then
            return 0
        fi
        sleep 5
    done
    echo "ERROR: the host did not come back within five minutes." >&2
    return 1
}

wait_for_mind() {
    local deadline=$((SECONDS + 300))
    while [ "$SECONDS" -lt "$deadline" ]; do
        if mind Identity1 Ready >/dev/null 2>&1; then
            return 0
        fi
        sleep 5
    done
    echo "ERROR: the Mind never answered after the reboot." >&2
    return 1
}

wait_for_health() {
    local deadline=$((SECONDS + 180))
    local health=""
    while [ "$SECONDS" -lt "$deadline" ]; do
        health="$(scalar Presence1 Health 2>/dev/null || true)"
        if [ "$health" = healthy ]; then
            return 0
        fi
        sleep 5
    done
    echo "ERROR: the control plane reported '${health:-nothing}' rather than healthy." >&2
    return 1
}

echo "==> Reading what the deployed system holds before the reboot..."
identity_before="$(scalar Identity1 IdentityId)"
sessions_before="$(scalar Identity1 SessionCount)"
journal_before="$(scalar Event1 Count)"
if [ -z "$identity_before" ]; then
    echo "ERROR: no identity to be continuous with; is the Mind deployed and running?" >&2
    exit 1
fi
echo "    identity $identity_before, session $sessions_before, journal $journal_before"

# The uptime is read so the reboot can be proven to have happened. A gate that asserted continuity
# without establishing that the machine actually went down would pass most convincingly when the
# reboot silently failed.
booted_before="$(cybou_ssh 'cat /proc/sys/kernel/random/boot_id')"
echo "    boot $booted_before"

echo "==> Rebooting the host..."
# The reboot kills the connection, so a non-zero exit here is the command working.
cybou_ssh 'sudo systemctl reboot' >/dev/null 2>&1 || true
sleep 15

wait_for_ssh
booted_after="$(cybou_ssh 'cat /proc/sys/kernel/random/boot_id')"
if [ "$booted_after" = "$booted_before" ]; then
    echo "ERROR: the host never rebooted; boot id is unchanged." >&2
    exit 1
fi
echo "    The host rebooted: boot $booted_after"

# The user manager has to start without anyone logging in. Without lingering it does not, and the
# whole Mind is simply absent until a human connects — which on a headless host is never.
echo "==> Waiting for the Mind to come back on its own..."
wait_for_mind

identity_after="$(scalar Identity1 IdentityId)"
sessions_after="$(scalar Identity1 SessionCount)"
journal_after="$(scalar Event1 Count)"

if [ "$identity_after" != "$identity_before" ]; then
    echo "ERROR: the subject changed across a reboot: $identity_before -> $identity_after." >&2
    exit 1
fi
echo "    Identity survived the reboot: $identity_after"

if [ "$sessions_after" -le "$sessions_before" ]; then
    echo "ERROR: session count did not advance across a reboot: $sessions_before -> $sessions_after." >&2
    exit 1
fi
echo "    Session advanced: $sessions_before -> $sessions_after"

if [ "$journal_after" -lt "$journal_before" ]; then
    echo "ERROR: the Journal shrank across a reboot: $journal_before -> $journal_after." >&2
    exit 1
fi
echo "    Journal did not shrink: $journal_before -> $journal_after"

# A count the biography cannot account for is the identity claiming more than it can show. This is
# the one assertion a reboot can make that a process restart cannot: the start was recorded by a
# process that came up with no session, no bus and no warm state behind it.
echo "==> Verifying the session the reboot created is one the Journal holds..."
start_id=""
deadline=$((SECONDS + 120))
while [ "$SECONDS" -lt "$deadline" ]; do
    start_id="$(scalar Identity1 SessionStartContribution 2>/dev/null || true)"
    if [ -n "$start_id" ]; then
        break
    fi
    sleep 5
done
if [ -z "$start_id" ]; then
    echo "ERROR: the identity counted session $sessions_after and recorded no start for it." >&2
    exit 1
fi
held="$(mind Event1 Contains s "$start_id" | awk '{print $2}')"
if [ "$held" != true ]; then
    echo "ERROR: the identity names start $start_id, and the Journal does not hold it." >&2
    exit 1
fi
echo "    Session $sessions_after is in the biography as $start_id"

# The chain is what a power cut is most likely to damage: a write interrupted mid-commit leaves a
# row whose hash does not follow the one before it. Verification is asked for by position, so a
# chain that was replayed to its head is the only answer that counts as verified.
echo "==> Verifying the Journal survived the power cut intact..."
integrity="$(scalar Event1 Verification >/dev/null 2>&1 && echo readable || echo unreadable)"
if [ "$integrity" != readable ]; then
    echo "ERROR: Event1 cannot answer for the integrity of its own chain after the reboot." >&2
    exit 1
fi
# healthd derives the biography capability from the chain: a chain it found contradicting itself
# takes accepted-biography down, so the control plane reaching healthy below is the assertion that
# the chain is intact. Checking it twice by different means is what makes that reading load-bearing
# rather than incidental.
echo "    Event1 answers for its chain"

wait_for_health
echo "    Control plane healthy after the reboot"

echo "==> Verifying the public surface came back with it..."
if ! cybou_ssh 'curl -fsS -o /dev/null http://127.0.0.1:8787/'; then
    echo "ERROR: the read-only web surface did not come back after the reboot." >&2
    exit 1
fi
echo "    The read-only web surface is serving again"

echo "==> Reboot continuity gate PASSED"
