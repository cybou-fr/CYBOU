#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# ADR-0022 A1: one granted typed host action, followed by an independent observation.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

if [ "$(id -u)" -ne 0 ] || ! command -v systemctl >/dev/null 2>&1 \
    || ! systemctl show --property=Version --value >/dev/null 2>&1 \
    || ! command -v dbus-run-session >/dev/null 2>&1; then
    echo "==> action gate NOT RUN: a root systemd host and dbus-run-session are required" >&2
    exit 3
fi

cargo build --quiet -p cybou-actiond -p cybou-executord
cargo build --quiet -p cybou-executord --example action-roundtrip
ACTIOND="$CARGO_TARGET_DIR/debug/cybou-actiond"
EXECUTORD="$CARGO_TARGET_DIR/debug/cybou-executord"
ROUNDTRIP="$CARGO_TARGET_DIR/debug/examples/action-roundtrip"
UNIT=/run/systemd/system/cybou-action-gate.service
WORK="$(mktemp -d)"

cleanup() {
    status=$?
    if [ "$status" -ne 0 ]; then
        [ -f "$WORK/actiond.log" ] && cat "$WORK/actiond.log" >&2
        [ -f "$WORK/executord.log" ] && cat "$WORK/executord.log" >&2
    fi
    systemctl stop cybou-action-gate.service >/dev/null 2>&1 || true
    rm -f "$UNIT"
    systemctl daemon-reload >/dev/null 2>&1 || true
    rm -rf "$WORK"
    return "$status"
}
trap cleanup EXIT

install -m 0644 /dev/stdin "$UNIT" <<'EOF'
[Unit]
Description=Harmless Cybou authorized-action gate unit

[Service]
Type=oneshot
ExecStart=/usr/bin/true
RemainAfterExit=yes
EOF
systemctl daemon-reload
systemctl stop cybou-action-gate.service >/dev/null 2>&1 || true

export ACTIOND EXECUTORD ROUNDTRIP WORK
dbus-run-session -- bash -euo pipefail <<'INNER'
CYBOU_PREAUTHORIZED_ACTIONS=service.restart "$ACTIOND" >"$WORK/actiond.log" 2>&1 &
action_pid=$!
"$EXECUTORD" >"$WORK/executord.log" 2>&1 &
executor_pid=$!
cleanup_session() {
    kill "$action_pid" "$executor_pid" >/dev/null 2>&1 || true
    wait "$action_pid" "$executor_pid" >/dev/null 2>&1 || true
}
trap cleanup_session EXIT

for _ in $(seq 1 50); do
    if busctl --user --list 2>/dev/null | grep -q org.cybou.Mind.Action1 \
        && busctl --user --list 2>/dev/null | grep -q org.cybou.Body.Executor1; then
        break
    fi
    sleep 0.1
done
busctl --user --list | grep -q org.cybou.Mind.Action1
busctl --user --list | grep -q org.cybou.Body.Executor1

"$ROUNDTRIP"
INNER

echo "=== action gate passed ==="
