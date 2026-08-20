#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Continuity and recovery under the process manager that actually runs the Mind.
#
# The multi-daemon gate proves the owners work together, but it starts them by hand under
# dbus-run-session: a different manager, a different startup order, and no unit dependencies. What
# a deployed system does when systemd restarts it, or when a required owner is lost and comes back,
# was proven by nothing after the NixOS VM gates were removed. This is the replacement.
#
# It runs against a deployed host and touches its live Mind: the target is restarted and one owner
# is stopped and started again. Everything it does is something a deploy or a crash already does.

set -euo pipefail

MACHINE="${CYBOU_USER_MACHINE:-cybou@.host}"

# The Mind runs in another user's manager, so reaching it needs privilege unless we already have
# it. Deciding here rather than requiring the caller to remember is one less way to run this
# against the wrong bus and believe the answer.
SUDO=()
if [ "$(id -u)" -ne 0 ]; then
    SUDO=(sudo)
fi
CTL=("${SUDO[@]}" systemctl --user --machine="$MACHINE")

mind() {
    local name="org.cybou.Mind.$1"
    local path="/$(printf '%s' "$name" | tr . /)"
    "${SUDO[@]}" busctl --user --machine="$MACHINE" call "$name" "$path" "$name" "$2" "${@:3}"
}

scalar() {
    mind "$@" | awk '{print $2}' | tr -d '"'
}

# Health is re-probed on an interval, so both directions need waiting for; polling for one and
# asserting the other immediately would test the probe interval rather than the system.
wait_for_health() {
    local want="$1"
    local deadline=$((SECONDS + 60))
    local health=""
    while [ "$SECONDS" -lt "$deadline" ]; do
        health="$(scalar Presence1 Health 2>/dev/null || true)"
        if [ "$health" = "$want" ]; then
            return 0
        fi
        sleep 1
    done
    echo "ERROR: the control plane never reported $want; it reported '${health:-nothing}'." >&2
    return 1
}

wait_until_not_healthy() {
    local deadline=$((SECONDS + 60))
    local health=""
    while [ "$SECONDS" -lt "$deadline" ]; do
        health="$(scalar Presence1 Health 2>/dev/null || true)"
        if [ -n "$health" ] && [ "$health" != "healthy" ]; then
            printf '%s' "$health"
            return 0
        fi
        sleep 1
    done
    return 1
}

wait_for_answer() {
    local deadline=$((SECONDS + 60))
    while [ "$SECONDS" -lt "$deadline" ]; do
        if mind Identity1 Ready >/dev/null 2>&1; then
            return 0
        fi
        sleep 1
    done
    echo "ERROR: Identity1 never answered after the restart." >&2
    return 1
}

echo "==> Reading the identity the deployed system holds..."
identity_before="$(scalar Identity1 IdentityId)"
sessions_before="$(scalar Identity1 SessionCount)"
journal_before="$(scalar Event1 Count)"
if [ -z "$identity_before" ]; then
    echo "ERROR: no identity to be continuous with; is the Mind deployed and running?" >&2
    exit 1
fi
echo "    identity $identity_before, session $sessions_before, journal $journal_before"

# Continuity is the one claim a restart can falsify outright. An identity that changes across a
# restart is a different subject wearing the same biography.
echo "==> Restarting the whole Mind through systemd..."
"${CTL[@]}" restart cybou-mind.target
wait_for_answer

identity_after="$(scalar Identity1 IdentityId)"
sessions_after="$(scalar Identity1 SessionCount)"
journal_after="$(scalar Event1 Count)"

if [ "$identity_after" != "$identity_before" ]; then
    echo "ERROR: the subject changed across a restart: $identity_before -> $identity_after." >&2
    exit 1
fi
echo "    Identity survived: $identity_after"

# A restart is a new session and the system has to know it happened. Equal counts would mean the
# restart left no trace in the thing whose job is remembering that it did.
if [ "$sessions_after" -le "$sessions_before" ]; then
    echo "ERROR: session count did not advance across a restart: $sessions_before -> $sessions_after." >&2
    exit 1
fi
echo "    Session advanced: $sessions_before -> $sessions_after"

# The Journal is append-only; a restart that lost rows would be losing biography.
if [ "$journal_after" -lt "$journal_before" ]; then
    echo "ERROR: the Journal shrank across a restart: $journal_before -> $journal_after." >&2
    exit 1
fi
echo "    Journal did not shrink: $journal_before -> $journal_after"

wait_for_health 'healthy'
echo "    Control plane healthy after the restart"

# Losing a required owner is the failure the removed VM gates used to cover. Under systemd the
# question is not only whether the control plane notices, but whether it recovers on its own once
# the unit is back.
echo "==> Losing a required owner and getting it back..."
"${CTL[@]}" stop cybou-identityd.service

# Put the owner back whatever happens next: leaving a deployed Mind without its identity because
# an assertion failed would make this gate worse than the gap it fills.
trap '"${CTL[@]}" start cybou-identityd.service >/dev/null 2>&1 || true' EXIT

if ! degraded="$(wait_until_not_healthy)"; then
    echo "ERROR: identityd is stopped and the control plane still calls itself healthy." >&2
    exit 1
fi
echo "    Control plane reported '$degraded' with identityd stopped"

"${CTL[@]}" start cybou-identityd.service
trap - EXIT
wait_for_answer
wait_for_health 'healthy'
echo "    Control plane recovered after identityd returned"

identity_final="$(scalar Identity1 IdentityId)"
if [ "$identity_final" != "$identity_before" ]; then
    echo "ERROR: the subject changed after losing and restoring its owner." >&2
    exit 1
fi

echo "==> systemd continuity and recovery gate PASSED"
