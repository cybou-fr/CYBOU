#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Multi-daemon integration verification gate running under an isolated D-Bus session.
#
# The script re-executes itself inside `dbus-run-session` rather than launching the daemons
# from a nested shell: the cleanup trap, the PID list and the daemons must all live in one
# process, otherwise the trap runs in a shell that never saw the PIDs and the daemons leak
# past the end of the run.

set -euo pipefail

if [ -z "${CYBOU_TEST_DBUS_SESSION:-}" ]; then
    if command -v dbus-run-session >/dev/null 2>&1; then
        exec env CYBOU_TEST_DBUS_SESSION=1 dbus-run-session -- "$0" "$@"
    fi
    if [ "$(uname -s)" = "Linux" ]; then
        echo "ERROR: dbus-run-session not found; install dbus-daemon before running this gate." >&2
        exit 1
    fi
    echo "==> Skipping: Linux session bus unavailable on $(uname -s)."
    exit 0
fi

TMP_DIR="$(mktemp -d)"
export XDG_STATE_HOME="$TMP_DIR/state"
export XDG_DATA_HOME="$TMP_DIR/data"
mkdir -p "$XDG_STATE_HOME/cybou" "$XDG_DATA_HOME/cybou"

PIDS=()
cleanup() {
    echo "==> Cleaning up spawned test daemons..."
    for pid in "${PIDS[@]:-}"; do
        [ -n "$pid" ] || continue
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null || true
            wait "$pid" 2>/dev/null || true
        fi
    done
    rm -rf "$TMP_DIR"
    echo "==> Integration test cleanup complete."
}
trap cleanup EXIT

echo "==> Building all Mind daemons..."
cargo build --workspace --bins

BIN_DIR="$(cargo metadata --format-version 1 | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')/debug"

# Wait for a well-known name to be owned instead of sleeping a fixed interval: a loaded CI
# runner takes longer than any constant that is still fast on a developer machine.
wait_for_name() {
    local name="$1"
    local deadline=$((SECONDS + 20))
    while [ "$SECONDS" -lt "$deadline" ]; do
        if busctl --user status "$name" >/dev/null 2>&1; then
            return 0
        fi
        sleep 0.2
    done
    echo "ERROR: $name never appeared on the isolated session bus." >&2
    return 1
}

spawn() {
    "$BIN_DIR/$1" &
    PIDS+=("$!")
}

echo "==> Launching cybou-eventd..."
spawn cybou-eventd
wait_for_name org.cybou.Mind.Event1

echo "==> Launching cognitive organ daemons..."
spawn cybou-identityd
spawn cybou-healthd

spawn cybou-intentiond
INTENTION_PID="${PIDS[-1]}"

spawn cybou-predictord
spawn cybou-perceptiond
spawn cybou-epistemicd
spawn cybou-contextd
spawn cybou-workspaced
spawn cybou-lifecycled
spawn cybou-selfd
spawn cybou-presenced

NAMES=(
    org.cybou.Mind.Event1
    org.cybou.Mind.Identity1
    org.cybou.Mind.Health1
    org.cybou.Mind.Intention1
    org.cybou.Mind.Predictor1
    org.cybou.Mind.Perception1
    org.cybou.Mind.Epistemic1
    org.cybou.Mind.Context1
    org.cybou.Mind.Workspace1
    org.cybou.Mind.Lifecycle1
    org.cybou.Mind.Self1
    org.cybou.Mind.Presence1
)

for name in "${NAMES[@]}"; do
    wait_for_name "$name"
done

# Health1 probes Ready on every organ, so an organ that does not export it is indistinguishable
# from one that is down and pins the whole control plane at "unavailable". Check all of them.
echo "==> Testing that every organ answers the Health1 readiness probe..."
for name in "${NAMES[@]}"; do
    path="/$(printf '%s' "$name" | tr . /)"
    answer="$(busctl --user call "$name" "$path" "$name" Ready)"
    if [ "$answer" != "b true" ]; then
        echo "ERROR: $name Ready answered '$answer', expected 'b true'." >&2
        exit 1
    fi
    echo "    $name Ready -> $answer"
done

echo "==> Testing Intention formation and restart survival..."
INTENTION_ID=$(busctl --user call org.cybou.Mind.Intention1 /org/cybou/Mind/Intention1 org.cybou.Mind.Intention1 Form sss "Run integration tests" "Session startup" "" | awk '{print $2}' | tr -d '"')
echo "Formed Intention ID: $INTENTION_ID"

if [ -z "$INTENTION_ID" ]; then
    echo "ERROR: Failed to form intention!" >&2
    exit 1
fi

echo "==> Restarting cybou-intentiond to verify restart survival..."
kill "$INTENTION_PID" 2>/dev/null || true
wait "$INTENTION_PID" 2>/dev/null || true

spawn cybou-intentiond
wait_for_name org.cybou.Mind.Intention1

busctl --user call org.cybou.Mind.Intention1 /org/cybou/Mind/Intention1 org.cybou.Mind.Intention1 Ready

echo "==> Testing Presence1 query..."
busctl --user call org.cybou.Mind.Presence1 /org/cybou/Mind/Presence1 org.cybou.Mind.Presence1 Ready

# With every organ up and answering, the control plane must describe itself as healthy. The first
# probe round fires before the later organs have taken their names, so the reading starts degraded
# and settles; poll until it does rather than accepting the transient.
echo "==> Waiting for the control plane to report its own health..."
health="unset"
deadline=$((SECONDS + 40))
while [ "$SECONDS" -lt "$deadline" ]; do
    health="$(busctl --user call org.cybou.Mind.Presence1 /org/cybou/Mind/Presence1 org.cybou.Mind.Presence1 Health)"
    if [ "$health" = 's "healthy"' ]; then
        break
    fi
    sleep 1
done
echo "    Presence1 Health -> $health"
if [ "$health" != 's "healthy"' ]; then
    echo "ERROR: every organ is running and answering Ready, yet the control plane settled on $health." >&2
    exit 1
fi

echo "==> Multi-daemon integration test PASSED successfully!"
