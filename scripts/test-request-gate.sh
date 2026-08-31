#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# ADR-0048: the door a person walks through.
#
# The two gates beside this one drive proposals Mind made from its own findings — one pre-authorized
# in advance, one answered by a person when nothing was. Both start from something this host
# concluded about itself.
#
# This one starts from a person. Nothing observes anything, no finding is written, and there is no
# question to answer, because the asking is the confirmation. What it proves is the other entrance
# whole: request, permit, restart, and an independent observation that it happened — with what is
# forbidden still forbidden, a verb outside the table still not an operation, and the permit still
# spent once.

set -euo pipefail

# Action1 and Executor1 meet on the system bus, and both write the lifecycle to Event1, which lives
# on the session one. A run without a session bus reaches the executor and is refused there —
# correctly, because an execution that cannot be made durable must not touch the Body — so the gate
# would be reporting the absence of a Journal as a failure of confirmation.
if [ -z "${CYBOU_REQUEST_GATE_DBUS:-}" ]; then
    if command -v dbus-run-session >/dev/null 2>&1; then
        exec env CYBOU_REQUEST_GATE_DBUS=1 dbus-run-session -- "$0" "$@"
    fi
    echo "==> request gate NOT RUN: dbus-run-session is required" >&2
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
    echo "==> request gate NOT RUN: a root systemd host is required" >&2
    exit 3
fi

cargo build --quiet -p cybou-actiond -p cybou-executord -p cybou-eventd
cargo build --quiet -p cybou-actiond --example request-roundtrip
ACTIOND="$CARGO_TARGET_DIR/debug/cybou-actiond"
EXECUTORD="$CARGO_TARGET_DIR/debug/cybou-executord"
EVENTD="$CARGO_TARGET_DIR/debug/cybou-eventd"
ROUNDTRIP="$CARGO_TARGET_DIR/debug/examples/request-roundtrip"
UNIT=/run/systemd/system/cybou-request-gate.service
WORK="$(mktemp -d)"
DBUS_POLICY="/etc/dbus-1/system.d/cybou-request-gate-$$.conf"
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
    systemctl stop cybou-request-gate.service >/dev/null 2>&1 || true
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
Description=Harmless Cybou requested-action gate unit

[Service]
# It sleeps rather than exiting, because the gate now asks this unit to stop, to start and to
# reload, and each of those wants a unit it can be true of: a oneshot that has already run is
# `active` because it was told to remember it ran, and has nothing to reload.
Type=simple
ExecStart=/usr/bin/sleep infinity
# Reloading is a no-op here on purpose. What the gate checks is that the request reached systemd
# under its own name and the unit survived it, not what re-reading a configuration would do.
ExecReload=/usr/bin/true

# An [Install] section, because enabling a unit without one changes nothing about the next boot and
# the executor refuses to pretend otherwise. The gate wants the succeeding path here; the refusal
# has its own coverage in the executor's own tests.
[Install]
WantedBy=multi-user.target
EOF
systemctl daemon-reload
systemctl stop cybou-request-gate.service >/dev/null 2>&1 || true

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

# Empty rather than unset, so the run states the condition instead of inheriting whatever the
# environment had. A person's request is never pre-authorized in any case — pre-authorization
# exists so something can act while nobody is present — so this is here to prove that rather than
# to enable anything.
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

# A real process for the roundtrip to end. It is a sleep owned by whoever runs this gate, and the
# roundtrip is told its pid and its owner separately so that the executor's own check — which reads
# /proc rather than believing either number — has something true to agree with.
sleep 600 &
victim_pid=$!
CYBOU_GATE_VICTIM_PID="$victim_pid"
CYBOU_GATE_VICTIM_UID="$(id -u)"
export CYBOU_GATE_VICTIM_PID CYBOU_GATE_VICTIM_UID

"$ROUNDTRIP"

echo "=== What the person enabled is enabled, and what they disabled is not ==="
# `is-enabled` reads the unit file state from systemd rather than from the executor's account of
# itself, and it is the same question `systemctl` answers for anybody who asks later.
state="$(systemctl is-enabled cybou-request-gate.service 2>&1 || true)"
if [ "$state" != "disabled" ]; then
    echo "ERROR: after enable then disable, systemd says the unit is $state" >&2
    exit 1
fi
echo "    ok      systemd says disabled, having said enabled in between"

echo "=== The process the person asked about is gone ==="
if kill -0 "$victim_pid" 2>/dev/null; then
    # `kill -0` asks whether the pid can be signalled, which is how to ask whether it is still
    # there without sending anything to it.
    echo "ERROR: pid $victim_pid is still running after a granted process.terminate" >&2
    exit 1
fi
wait "$victim_pid" 2>/dev/null || true
echo "    ok      pid $victim_pid ended, and an independent check says so"

# The lifecycle is not only in the owner's memory, and for a person's request that took a fix: the
# Journal refuses a proposal with no cause, and a person's request has none, so until the asking
# was written down as the root of its own episode every request a person made was recorded as
# nothing at all.
echo "=== The person who asked is in the Journal, and so is what was decided ==="
journal="$XDG_DATA_HOME/cybou/journal.sqlite3"
if [ ! -f "$journal" ]; then
    echo "ERROR: the gate wrote no Journal at $journal" >&2
    exit 1
fi
if ! command -v sqlite3 >/dev/null 2>&1; then
    echo "==> request gate NOT RUN: sqlite3 is required to read the Journal back" >&2
    exit 3
fi

# Kind 10 is Decision. The numbers are frozen — every kind's numeric value is part of every hash
# already written — so reading them by number is reading them the way the Journal stores them.
# Kind 1 is Observation and kind 10 is Decision. The numbers are frozen — every kind's numeric
# value is part of every hash already written — so reading them by number is reading them the way
# the Journal stores them.
#
# Nothing else in this gate writes an Observation: no telemetry runs and no finding is made. So the
# root is the asking, and its absence would mean a restart nobody can be traced to.
asked="$(sqlite3 "$journal" 'SELECT count(*) FROM contribution WHERE kind = 1;')"
decisions="$(sqlite3 "$journal" 'SELECT count(*) FROM contribution WHERE kind = 10;')"
echo "    askings: $asked   decisions: $decisions"
if [ "$asked" -lt 1 ]; then
    echo "ERROR: nobody is recorded as having asked, so the restart traces back to no one" >&2
    exit 1
fi
if [ "$decisions" -lt 1 ]; then
    echo "ERROR: the Journal holds no decision for a request that was carried out" >&2
    exit 1
fi

echo "=== request gate passed ==="
