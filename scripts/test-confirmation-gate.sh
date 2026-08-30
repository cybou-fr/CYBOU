#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# ADR-0022: the path a host takes when nobody pre-authorized anything.
#
# The A1 gate beside this one grants `service.restart` in advance and proves a finding can reach
# the Body with nobody present. That is the path a standing policy opens. This one runs the host
# in the state every installation is actually in — no standing policy at all — where a proposal
# stops at a question, and proves that a person's answer is what carries it the rest of the way.

set -euo pipefail

# Action1 and Executor1 meet on the system bus, and both write the lifecycle to Event1, which lives
# on the session one. A run without a session bus reaches the executor and is refused there —
# correctly, because an execution that cannot be made durable must not touch the Body — so the gate
# would be reporting the absence of a Journal as a failure of confirmation.
if [ -z "${CYBOU_CONFIRMATION_GATE_DBUS:-}" ]; then
    if command -v dbus-run-session >/dev/null 2>&1; then
        exec env CYBOU_CONFIRMATION_GATE_DBUS=1 dbus-run-session -- "$0" "$@"
    fi
    echo "==> confirmation gate NOT RUN: dbus-run-session is required" >&2
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
    echo "==> confirmation gate NOT RUN: a root systemd host is required" >&2
    exit 3
fi

cargo build --quiet -p cybou-actiond -p cybou-executord -p cybou-eventd
cargo build --quiet -p cybou-actiond --example confirmation-roundtrip
ACTIOND="$CARGO_TARGET_DIR/debug/cybou-actiond"
EXECUTORD="$CARGO_TARGET_DIR/debug/cybou-executord"
EVENTD="$CARGO_TARGET_DIR/debug/cybou-eventd"
ROUNDTRIP="$CARGO_TARGET_DIR/debug/examples/confirmation-roundtrip"
UNIT=/run/systemd/system/cybou-confirmation-gate.service
WORK="$(mktemp -d)"
DBUS_POLICY="/etc/dbus-1/system.d/cybou-confirmation-gate-$$.conf"
action_pid=
executor_pid=
event_pid=

# A Journal of its own. This gate writes real contributions and must not add them to whatever the
# host is keeping, and must not read a decision some earlier run left behind.
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
    systemctl stop cybou-confirmation-gate.service >/dev/null 2>&1 || true
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
Description=Harmless Cybou confirmed-action gate unit

[Service]
Type=oneshot
ExecStart=/usr/bin/true
RemainAfterExit=yes
EOF
systemctl daemon-reload
systemctl stop cybou-confirmation-gate.service >/dev/null 2>&1 || true

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

# The whole point of this gate. `CYBOU_PREAUTHORIZED_ACTIONS` is deliberately empty rather than
# unset, so the run states the condition instead of inheriting whatever the environment had.
echo "=== Action1 starts with nothing pre-authorized ==="
CYBOU_ACTION_SYSTEM_BUS=1 CYBOU_PREAUTHORIZED_ACTIONS= \
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

# The lifecycle is not only in the owner's memory. A confirmation that authorized a restart and
# left nothing behind would answer "why did this restart" with the process that has since exited.
echo "=== The answer is in the Journal, not only in the process that took it ==="
journal="$XDG_DATA_HOME/cybou/journal.sqlite3"
if [ ! -f "$journal" ]; then
    echo "ERROR: the gate wrote no Journal at $journal" >&2
    exit 1
fi
if ! command -v sqlite3 >/dev/null 2>&1; then
    echo "==> confirmation gate NOT RUN: sqlite3 is required to read the Journal back" >&2
    exit 3
fi

# Kind 10 is Decision. The numbers are frozen — every kind's numeric value is part of every hash
# already written — so reading them by number is reading them the way the Journal stores them.
decisions="$(sqlite3 "$journal" 'SELECT count(*) FROM contribution WHERE kind = 10;')"
echo "    decisions in the Journal: $decisions"
if [ "$decisions" -lt 2 ]; then
    # Two, because the question and the answer are two decisions. A run holding one is a run where
    # the confirmation authorized a restart and left the Journal saying only that somebody was
    # asked — and "why did this restart" would be answerable from the process that has since
    # exited, which is the one place it must not have to be answered from.
    echo "ERROR: the Journal holds $decisions decision(s); the question and the answer are two" >&2
    exit 1
fi

echo "=== confirmation gate passed ==="
