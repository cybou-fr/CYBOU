#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# ADR-0022 A1: one granted typed host action, followed by an independent observation.

set -euo pipefail

# Action1 and Executor1 meet on the system bus and both write the lifecycle to Event1, which lives
# on the session one. Without a Journal the executor refuses — correctly, because an execution that
# cannot be made durable must not touch the Body — and the gate would be reporting the absence of a
# Journal as a failure of the action boundary.
if [ -z "${CYBOU_ACTION_GATE_DBUS:-}" ]; then
    if command -v dbus-run-session >/dev/null 2>&1; then
        exec env CYBOU_ACTION_GATE_DBUS=1 dbus-run-session -- bash "$0" "$@"
    fi
    echo "==> action gate NOT RUN: dbus-run-session is required" >&2
    exit 3
fi

cd "$(dirname "$0")/.."

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

if [ "$(id -u)" -ne 0 ] || ! command -v systemctl >/dev/null 2>&1 \
    || ! systemctl show --property=Version --value >/dev/null 2>&1; then
    echo "==> action gate NOT RUN: a root systemd host is required" >&2
    exit 3
fi

cargo build --quiet -p cybou-actiond -p cybou-executord -p cybou-eventd
cargo build --quiet -p cybou-executord --example action-roundtrip
ACTIOND="$CARGO_TARGET_DIR/debug/cybou-actiond"
EXECUTORD="$CARGO_TARGET_DIR/debug/cybou-executord"
EVENTD="$CARGO_TARGET_DIR/debug/cybou-eventd"
ROUNDTRIP="$CARGO_TARGET_DIR/debug/examples/action-roundtrip"
UNIT=/run/systemd/system/cybou-action-gate.service
WORK="$(mktemp -d)"
DBUS_POLICY="/etc/dbus-1/system.d/cybou-action-gate-$$.conf"
action_pid=
executor_pid=
event_pid=

# A Journal of its own. This gate writes real contributions and must not add them to whatever the
# host is keeping, nor read a decision an earlier run left behind.
export XDG_STATE_HOME="$WORK/state"
export XDG_DATA_HOME="$WORK/data"
mkdir -p "$XDG_STATE_HOME/cybou" "$XDG_DATA_HOME/cybou"

cleanup() {
    status=$?
    if [ "$status" -ne 0 ]; then
        [ -f "$WORK/eventd.log" ] && cat "$WORK/eventd.log" >&2
        [ -f "$WORK/actiond.log" ] && cat "$WORK/actiond.log" >&2
        [ -f "$WORK/executord.log" ] && cat "$WORK/executord.log" >&2
    fi
    [ -n "$action_pid" ] && kill "$action_pid" >/dev/null 2>&1 || true
    [ -n "$executor_pid" ] && kill "$executor_pid" >/dev/null 2>&1 || true
    [ -n "$event_pid" ] && kill "$event_pid" >/dev/null 2>&1 || true
    [ -n "$action_pid" ] && wait "$action_pid" >/dev/null 2>&1 || true
    [ -n "$executor_pid" ] && wait "$executor_pid" >/dev/null 2>&1 || true
    [ -n "$event_pid" ] && wait "$event_pid" >/dev/null 2>&1 || true
    systemctl stop cybou-action-gate.service >/dev/null 2>&1 || true
    rm -f "$UNIT"
    systemctl daemon-reload >/dev/null 2>&1 || true
    rm -f "$DBUS_POLICY"
    systemctl reload dbus.service >/dev/null 2>&1 || true
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

# Both halves of the production permit boundary use the system transport. Install a uniquely named,
# temporary ownership rule rather than relying on a developer machine to have deployment policy.
install -m 0644 /dev/stdin "$DBUS_POLICY" <<'EOF'
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <policy user="root">
    <allow own="org.cybou.Mind.Action1"/>
    <allow send_destination="org.cybou.Mind.Action1"/>
    <allow own="org.cybou.Body.Executor1"/>
    <allow send_destination="org.cybou.Body.Executor1"/>
  </policy>
</busconfig>
EOF
systemctl reload dbus.service

echo "=== The Journal is running, so an execution can be made durable ==="
"$EVENTD" >"$WORK/eventd.log" 2>&1 &
event_pid=$!
for _ in $(seq 1 100); do
    if busctl --user --list 2>/dev/null | grep -q org.cybou.Mind.Event1; then
        break
    fi
    sleep 0.1
done
busctl --user --list | grep -q org.cybou.Mind.Event1

echo "=== Operator policy provisioning is closed and fail-safe ==="
POLICY="$WORK/action-policy.env"
CYBOU_ACTION_POLICY_PATH="$POLICY" CYBOU_ACTION_POLICY_NO_RESTART=1 \
    bash scripts/cybou-action-policy.sh service.status,service.restart >/dev/null
grep -qx 'CYBOU_PREAUTHORIZED_ACTIONS=service.status,service.restart' "$POLICY"
before="$(sha256sum "$POLICY")"
if CYBOU_ACTION_POLICY_PATH="$POLICY" CYBOU_ACTION_POLICY_NO_RESTART=1 \
    bash scripts/cybou-action-policy.sh service.reload >/dev/null 2>&1; then
    echo "ERROR: policy provisioning accepted an adapter that does not exist" >&2
    exit 1
fi
[ "$(sha256sum "$POLICY")" = "$before" ]
CYBOU_ACTION_POLICY_PATH="$POLICY" CYBOU_ACTION_POLICY_NO_RESTART=1 \
    bash scripts/cybou-action-policy.sh none >/dev/null
grep -qx 'CYBOU_PREAUTHORIZED_ACTIONS=' "$POLICY"
echo "    ok      invalid policy is refused without replacing the previous grant"

CYBOU_ACTION_SYSTEM_BUS=1 CYBOU_PREAUTHORIZED_ACTIONS=service.restart \
    "$ACTIOND" >"$WORK/actiond.log" 2>&1 &
action_pid=$!
export CYBOU_ACTION_SYSTEM_BUS=1
export CYBOU_EXECUTOR_SYSTEM_BUS=1
"$EXECUTORD" >"$WORK/executord.log" 2>&1 &
executor_pid=$!

for _ in $(seq 1 50); do
    if busctl --system --list 2>/dev/null | grep -q org.cybou.Mind.Action1 \
        && busctl --system --list 2>/dev/null | grep -q org.cybou.Body.Executor1; then
        break
    fi
    sleep 0.1
done
busctl --system --list | grep -q org.cybou.Mind.Action1
busctl --system --list | grep -q org.cybou.Body.Executor1

"$ROUNDTRIP"

echo "=== action gate passed ==="
