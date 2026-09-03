#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# The whole loop, through the door a person actually walks through.
#
# Three gates beside this one prove parts of it: the self-maintenance gate proves a host repairing
# itself unattended, the confirmation gate proves a person's answer carrying a proposal to the Body,
# and the request gate proves a person asking for something directly. All three drive D-Bus. None of
# them is the thing a person does, which is: look at a panel, see a finding with the readings behind
# it, answer the question it raises, and watch the host say what looking again established.
#
# So this one goes through HTTP and nothing else. It never touches Action1 or the executor itself,
# the way the browser never does.
#
# It was written because that path was broken. Asking for a restart directly carried its permit to
# the executor; answering *yes* to the host's own proposal issued a permit nothing claimed, so the
# desktop showed a permission granted and an act that never happened.
#
# Exit 3 means this host has no root systemd to run it on.
set -euo pipefail

if [ -z "${CYBOU_ATTENDED_GATE_DBUS:-}" ]; then
    if command -v dbus-run-session >/dev/null 2>&1; then
        exec env CYBOU_ATTENDED_GATE_DBUS=1 dbus-run-session -- bash "$0" "$@"
    fi
    echo "==> attended remediation gate NOT RUN: dbus-run-session is required" >&2
    exit 3
fi

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

not_run() { echo "==> attended remediation gate NOT RUN: $1" >&2; exit 3; }
[ "$(id -u)" -eq 0 ] || not_run "a root systemd host is required"
command -v systemctl >/dev/null 2>&1 || not_run "systemctl is not available"
systemctl show --property=Version --value >/dev/null 2>&1 || not_run "systemd is not running here"
for command in curl python3 busctl; do
    command -v "$command" >/dev/null || not_run "$command is not installed"
done

UNIT_NAME=cybou-attended-gate.service
UNIT="/run/systemd/system/$UNIT_NAME"
DBUS_POLICY="/etc/dbus-1/system.d/cybou-attended-gate-$$.conf"
WORK="$(mktemp -d)"
export XDG_STATE_HOME="$WORK/state"
export XDG_DATA_HOME="$WORK/data"
export XDG_CONFIG_HOME="$WORK/config"
mkdir -p "$XDG_STATE_HOME/cybou" "$XDG_DATA_HOME/cybou" "$XDG_CONFIG_HOME/cybou"
pids=()

cleanup() {
    status=$?
    if [ "$status" -ne 0 ]; then
        for log in "$WORK"/*.log; do
            [ -f "$log" ] && { echo "--- $log"; tail -25 "$log"; } >&2
        done
    fi
    for pid in "${pids[@]:-}"; do
        [ -n "$pid" ] && kill "$pid" >/dev/null 2>&1 || true
    done
    wait >/dev/null 2>&1 || true
    systemctl stop "$UNIT_NAME" >/dev/null 2>&1 || true
    rm -f "$UNIT"
    systemctl daemon-reload >/dev/null 2>&1 || true
    rm -f "$DBUS_POLICY"
    systemctl reload dbus.service >/dev/null 2>&1 || true
    rm -rf "$WORK"
    return "$status"
}
trap cleanup EXIT

cargo build --quiet --locked -p cybou-eventd -p cybou-telemetryd -p cybou-actiond \
    -p cybou-executord -p cybou-remediationd -p cybou-web-gateway
BIN="$CARGO_TARGET_DIR/debug"

# A unit that does nothing and can be stopped and started without consequence. The finding this gate
# waits for is about this and nothing else on the host.
install -m 0644 /dev/stdin "$UNIT" <<'EOF'
[Unit]
Description=Harmless Cybou attended remediation gate unit

[Service]
Type=oneshot
ExecStart=/usr/bin/true
RemainAfterExit=yes
EOF
systemctl daemon-reload
systemctl start "$UNIT_NAME"
systemctl is-active --quiet "$UNIT_NAME"

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

spawn() {
    "$@" >>"$WORK/$(basename "$1").log" 2>&1 &
    pids+=("$!")
}

wait_for_session_name() {
    for _ in $(seq 1 150); do
        busctl --user list --no-legend 2>/dev/null | grep -q "^$1 " && return 0
        sleep 0.1
    done
    echo "$1 never appeared on the session bus" >&2
    return 1
}

wait_for_system_name() {
    for _ in $(seq 1 150); do
        busctl --system list --no-legend 2>/dev/null | grep -q "^$1 " && return 0
        sleep 0.1
    done
    echo "$1 never appeared on the system bus" >&2
    return 1
}

echo "=== The Journal is running, so what happens here is durable ==="
spawn "$BIN/cybou-eventd"
wait_for_session_name org.cybou.Mind.Event1

# The telemetry organ watches this unit because an operator declared it, and nothing else about this
# host is any of its business.
printf 'service %s\n' "$UNIT_NAME" >"$XDG_CONFIG_HOME/cybou/telemetry.watch"
spawn "$BIN/cybou-telemetryd"
wait_for_session_name org.cybou.Mind.Telemetry1

# Nothing is pre-authorized. Empty rather than unset, so the run states the condition instead of
# inheriting whatever the environment had: this is the installation every deployment starts as, and
# the whole point of the gate is what happens when a person is the only way forward.
export CYBOU_PREAUTHORIZED_ACTIONS=
export CYBOU_ACTION_SYSTEM_BUS=1
export CYBOU_EXECUTOR_SYSTEM_BUS=1
spawn "$BIN/cybou-actiond"
spawn "$BIN/cybou-executord"
wait_for_system_name org.cybou.Mind.Action1
wait_for_system_name org.cybou.Body.Executor1

# What proposes. It will be refused, once, and stop — which is what a host with nothing permitted
# should do, and is why a person has to answer for anything to happen.
spawn "$BIN/cybou-remediationd"

PORT="$(python3 - <<'PYTHON'
import socket
with socket.socket() as sock:
    sock.bind(('127.0.0.1', 0))
    print(sock.getsockname()[1])
PYTHON
)"
BASE="http://127.0.0.1:$PORT"
CYBOU_SESSION_MODE=local-desktop CYBOU_GATEWAY_ADDR="127.0.0.1:$PORT" \
    spawn "$BIN/cybou-web-gateway"
for _ in $(seq 1 150); do
    curl -fsS --max-time 2 "$BASE/api/v1/session" >/dev/null 2>&1 && break
    sleep 0.1
done
curl -fsS --max-time 2 "$BASE/api/v1/session" >/dev/null

echo "=== Now break it, and let this host notice on its own ==="
systemctl stop "$UNIT_NAME"
systemctl is-active --quiet "$UNIT_NAME" && {
    echo "the gate unit refused to stop, so there is nothing to notice" >&2
    exit 1
}

# What a person sees first: a finding, with the readings behind it and what the gate would say.
echo "==> waiting for the finding to reach the panel"
finding=""
for _ in $(seq 1 120); do
    curl -fsS --max-time 3 "$BASE/api/v1/insight" >"$WORK/insight.json" 2>/dev/null || { sleep 1; continue; }
    finding="$(python3 - "$WORK/insight.json" "$UNIT_NAME" <<'PYTHON'
import json, sys
insight = json.load(open(sys.argv[1]))
# This gate's own unit and nothing else. A host with something else unhealthy — a unit left failed
# by an earlier run, say — would otherwise have the gate answering a question about that instead.
for item in insight.get("findings", []):
    if item.get("about") != sys.argv[2]:
        continue
    offers = item.get("offers", [])
    if not any(offer.get("verdict") == "requires-confirmation" for offer in offers):
        continue
    # A finding a person is asked to act on must carry what it was concluded from.
    assert item.get("readings"), f"a finding reached the panel with no readings behind it: {item}"
    print(item["id"])
    break
PYTHON
)" || finding=""
    [ -n "$finding" ] && break
    sleep 1
done
[ -n "$finding" ] || {
    echo "no finding awaiting a person's answer reached the panel" >&2
    cat "$WORK/insight.json" >&2 || true
    exit 1
}
echo "    ok      the panel shows a finding, its readings, and a question"

# The record behind that question, as the card reads it.
echo "==> reading the proposal the question belongs to"
for _ in $(seq 1 60); do
    curl -fsS --max-time 3 "$BASE/api/v1/actions?cause=$finding" >"$WORK/actions.json" 2>/dev/null && \
        python3 - "$WORK/actions.json" >"$WORK/ids" 2>/dev/null <<'PYTHON' && break
import json, sys
records = json.load(open(sys.argv[1]))
awaiting = [r for r in records if r["verdict"] == "requires-confirmation"]
assert awaiting, "no proposal is awaiting an answer"
record = awaiting[0]
assert record["outcome"] is None, "something already concluded before anybody answered"
print(record["proposalId"], record["decisionId"], record["operation"], record["targetResource"])
PYTHON
    sleep 1
done
read -r proposal decision operation target <"$WORK/ids"
[ -n "$proposal" ] || { echo "no proposal identity to answer" >&2; exit 1; }
echo "    ok      $operation on $target is waiting for a person"

echo "=== A person answers, and nothing else does ==="
curl -fsS --max-time 10 -X POST "$BASE/api/v1/actions/confirm" \
    -H 'content-type: application/json' \
    -d "{\"proposalId\":\"$proposal\",\"decisionId\":\"$decision\"}" >"$WORK/confirmed.json"
python3 - "$WORK/confirmed.json" <<'PYTHON'
import json, sys
record = json.load(open(sys.argv[1]))
assert record["verdict"] == "granted-on-confirmation", record["verdict"]
# The answer carried it. A record that says authorized and holds no attempt is the defect this gate
# was written for: a permission granted and an act that never happened.
assert record["attempt"] is not None, "the answer authorized a restart that nothing carried out"
assert record["attempt"]["report"] == "completed", record["attempt"]
PYTHON
echo "    ok      the answer was carried out, and the record says by whom it was authorized"

echo "=== And the host looks again rather than believing the executor ==="
for _ in $(seq 1 60); do
    systemctl is-active --quiet "$UNIT_NAME" && break
    sleep 1
done
systemctl is-active --quiet "$UNIT_NAME" || {
    echo "the unit was never actually restarted on this host" >&2
    exit 1
}
echo "    ok      the unit is running again, checked against systemd itself"

echo "=== attended remediation gate passed: finding → evidence → question → answer → act → look again ==="
