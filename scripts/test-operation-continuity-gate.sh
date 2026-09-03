#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# A real Agent1 session becomes one durable Operation1 identity, and the HTTP projection keeps that
# identity when either stateless gateway or lifecycle owner is restarted. Exit 3 means the host
# boundary needed to run the proof is absent.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

not_run() { echo "==> operation continuity gate NOT RUN: $1" >&2; exit 3; }
for command in curl gdbus python3 systemd-run; do
    command -v "$command" >/dev/null || not_run "$command is not installed"
done
systemctl --user is-system-running >/dev/null 2>&1 || not_run "there is no user service manager"
[ -n "${DBUS_SESSION_BUS_ADDRESS:-}" ] || not_run "there is no session bus"

name_is_owned() {
    gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$1" 2>/dev/null | grep -q true
}
name_is_owned org.cybou.Runtime.Agent1 && not_run "Agent1 already owns this session bus"
name_is_owned org.cybou.Runtime.Operation1 && not_run "Operation1 already owns this session bus"

# The directory a deployment keeps under /run, or wherever this run can write one. The owner reads
# the same override, so a proof of it does not need root; nothing deployed sets it.
LEASES="${CYBOU_AGENT_LEASE_ROOT:-/run/cybou-agent-leases}"
export CYBOU_AGENT_LEASE_ROOT="$LEASES"
mkdir -p "$LEASES" 2>/dev/null || not_run "$LEASES cannot be created"
[ -w "$LEASES" ] || not_run "$LEASES is not writable"
CAPSULE="$(cat /proc/sys/kernel/random/uuid)"
TASK="$(cat /proc/sys/kernel/random/uuid)"
OPERATION=""
CAPSULE_UNIT="cybou-capsule-$CAPSULE.service"
EGRESS_UNIT="cybou-egress-$CAPSULE.service"
WORK="$(mktemp -d)"
PORT="$(python3 - <<'PYTHON'
import socket
with socket.socket() as sock:
    sock.bind(('127.0.0.1', 0))
    print(sock.getsockname()[1])
PYTHON
)"
BASE="http://127.0.0.1:$PORT"
agent_pid="" operation_pid="" gateway_pid=""

cleanup() {
    test -z "$gateway_pid" || kill "$gateway_pid" 2>/dev/null || true
    test -z "$operation_pid" || kill "$operation_pid" 2>/dev/null || true
    test -z "$agent_pid" || kill "$agent_pid" 2>/dev/null || true
    systemctl --user stop "$CAPSULE_UNIT" "$EGRESS_UNIT" 2>/dev/null || true
    test -z "${SECOND:-}" || systemctl --user stop "cybou-capsule-$SECOND.service" \
        "cybou-egress-$SECOND.service" 2>/dev/null || true
    test -z "${SECOND:-}" || rm -f "$LEASES/$SECOND.lease" "$LEASES/$SECOND.env"
    rm -rf "$LEASES/$CAPSULE.lease" "$LEASES/$CAPSULE.env" "$WORK"
}
trap cleanup EXIT

cargo build --quiet --locked -p cybou-agentd -p cybou-operationd -p cybou-web-gateway
AGENTD="$CARGO_TARGET_DIR/debug/cybou-agentd"
OPERATIOND="$CARGO_TARGET_DIR/debug/cybou-operationd"
GATEWAY="$CARGO_TARGET_DIR/debug/cybou-web-gateway"

CYBOU_PROFILE_ID=operation-continuity-gate CYBOU_CAPSULE_ID="$CAPSULE" CYBOU_AGENT=opencode \
CYBOU_AGENT_WORKSPACE="$WORK" CYBOU_AGENT_LEASE_SECONDS=600 CYBOU_CAPSULE_MEMORY_MIB=256 \
CYBOU_CAPSULE_CPUS=1 CYBOU_CAPSULE_TASKS_MAX=32 CYBOU_CAPSULE_MAY_EXECUTE=yes \
CYBOU_EGRESS_HOSTS=example.com CYBOU_MODEL_CLASS=Strong CYBOU_MODEL_SPEND_LIMIT=100 \
    cargo run --quiet --locked -p cybou-capsule --example issue-lease -- "$LEASES/$CAPSULE.lease"
cat >"$LEASES/$CAPSULE.env" <<ENV
CYBOU_AGENT_TASK_ID=$TASK
CYBOU_MODEL_TOKEN_LIMIT=1000
CYBOU_MODEL_MAX_OUTPUT_TOKENS=32
CYBOU_MODEL_SENSITIVITY=1
ENV
systemd-run --user --quiet --collect --unit="${CAPSULE_UNIT%.service}" -- /bin/sleep 600
systemd-run --user --quiet --collect --unit="${EGRESS_UNIT%.service}" -- /bin/sleep 600

"$AGENTD" serve >"$WORK/agent.log" 2>&1 & agent_pid=$!
for _ in $(seq 1 100); do name_is_owned org.cybou.Runtime.Agent1 && break; sleep 0.1; done
name_is_owned org.cybou.Runtime.Agent1 || { cat "$WORK/agent.log" >&2; exit 1; }

start_operation() {
    CYBOU_OPERATION_STORE="$WORK/operations.sqlite3" "$OPERATIOND" >"$WORK/operation.log" 2>&1 &
    operation_pid=$!
    for _ in $(seq 1 100); do name_is_owned org.cybou.Runtime.Operation1 && return; sleep 0.1; done
    cat "$WORK/operation.log" >&2; exit 1
}
start_gateway() {
    CYBOU_GATEWAY_FIXTURE=1 CYBOU_SESSION_MODE=local-desktop \
    CYBOU_GATEWAY_ADDR="127.0.0.1:$PORT" "$GATEWAY" >"$WORK/gateway.log" 2>&1 &
    gateway_pid=$!
    for _ in $(seq 1 100); do
        curl -fsS --max-time 2 "$BASE/api/v1/session" >/dev/null 2>&1 && return
        sleep 0.1
    done
    cat "$WORK/gateway.log" >&2; exit 1
}
operation_is_visible() {
    curl -fsS --max-time 2 "$BASE/api/v1/operations/$OPERATION" >"$WORK/visible.json" 2>/dev/null || return 1
    python3 - "$WORK/visible.json" "$OPERATION" "$CAPSULE" <<'PYTHON'
import json, sys
operation = json.load(open(sys.argv[1]))
assert operation['id'] == sys.argv[2], operation
assert operation['subject']['type'] == 'agent', operation
assert operation['subject']['payload']['capsule_id'] == sys.argv[3], operation
assert operation['progress']['percent'] is None, operation
PYTHON
}
discover_operation() {
    curl -fsS --max-time 2 "$BASE/api/v1/operations" >"$WORK/list.json" 2>/dev/null || return 1
    OPERATION="$(python3 - "$WORK/list.json" "$CAPSULE" <<'PYTHON'
import json, sys
matches = [item for item in json.load(open(sys.argv[1]))['operations']
           if item.get('subject', {}).get('type') == 'agent'
           and item['subject']['payload'].get('capsule_id') == sys.argv[2]]
assert len(matches) == 1, matches
print(matches[0]['id'])
PYTHON
)" || return 1
    test -n "$OPERATION"
}

start_operation
start_gateway
for _ in $(seq 1 100); do discover_operation && break; sleep 0.1; done
operation_is_visible || {
    echo "Agent1 did not become visible through Operation1 and the gateway" >&2
    cat "$WORK/agent.log" "$WORK/operation.log" "$WORK/gateway.log" >&2
    curl --silent --show-error --max-time 2 "$BASE/api/v1/operations" >&2 || true
    exit 1
}

kill "$gateway_pid"; wait "$gateway_pid" 2>/dev/null || true; gateway_pid=""
start_gateway
operation_is_visible

kill "$operation_pid"; wait "$operation_pid" 2>/dev/null || true; operation_pid=""
start_operation
for _ in $(seq 1 100); do operation_is_visible && break; sleep 0.1; done
operation_is_visible

# A session Agent1 no longer establishes must stop claiming a live worker, without being given an
# ending nobody observed.
observation_is() {
    curl -fsS --max-time 2 "$BASE/api/v1/operations/$OPERATION" >"$WORK/observed.json" 2>/dev/null || return 1
    python3 - "$WORK/observed.json" "$1" <<'PYTHON'
import json, sys
operation = json.load(open(sys.argv[1]))
assert operation['observation'] == sys.argv[2], operation
# Detachment is an observation verdict: no one witnessed this work ending, so the lifecycle state
# stays whatever the worker last published.
assert operation['state']['status'] == 'running', operation
PYTHON
}

observation_is known || {
    echo "an observed Agent1 session was not reported as known" >&2
    exit 1
}

# A second session, so one restart can serve both halves of what is left to prove: the first
# capsule is gone and must detach, and this one is live and must be cancellable from the panel.
SECOND="$(cat /proc/sys/kernel/random/uuid)"
SECOND_CAPSULE_UNIT="cybou-capsule-$SECOND.service"
SECOND_EGRESS_UNIT="cybou-egress-$SECOND.service"
CYBOU_PROFILE_ID=operation-continuity-gate CYBOU_CAPSULE_ID="$SECOND" CYBOU_AGENT=opencode \
CYBOU_AGENT_WORKSPACE="$WORK" CYBOU_AGENT_LEASE_SECONDS=600 CYBOU_CAPSULE_MEMORY_MIB=256 \
CYBOU_CAPSULE_CPUS=1 CYBOU_CAPSULE_TASKS_MAX=32 CYBOU_CAPSULE_MAY_EXECUTE=yes \
CYBOU_EGRESS_HOSTS=example.com CYBOU_MODEL_CLASS=Strong CYBOU_MODEL_SPEND_LIMIT=100 \
    cargo run --quiet --locked -p cybou-capsule --example issue-lease -- "$LEASES/$SECOND.lease"
cat >"$LEASES/$SECOND.env" <<ENV
CYBOU_AGENT_TASK_ID=$(cat /proc/sys/kernel/random/uuid)
CYBOU_MODEL_TOKEN_LIMIT=1000
CYBOU_MODEL_MAX_OUTPUT_TOKENS=32
CYBOU_MODEL_SENSITIVITY=1
ENV
systemd-run --user --quiet --collect --unit="${SECOND_CAPSULE_UNIT%.service}" -- /bin/sleep 600
systemd-run --user --quiet --collect --unit="${SECOND_EGRESS_UNIT%.service}" -- /bin/sleep 600

kill "$agent_pid"; wait "$agent_pid" 2>/dev/null || true; agent_pid=""
systemctl --user stop "$CAPSULE_UNIT" "$EGRESS_UNIT" 2>/dev/null || true
rm -f "$LEASES/$CAPSULE.lease" "$LEASES/$CAPSULE.env"
"$AGENTD" serve >>"$WORK/agent.log" 2>&1 & agent_pid=$!
for _ in $(seq 1 100); do name_is_owned org.cybou.Runtime.Agent1 && break; sleep 0.1; done
name_is_owned org.cybou.Runtime.Agent1 || { cat "$WORK/agent.log" >&2; exit 1; }

for _ in $(seq 1 100); do observation_is detached && break; sleep 0.1; done
observation_is detached || {
    echo "a vanished Agent1 session kept claiming a live worker" >&2
    cat "$WORK/agent.log" "$WORK/operation.log" >&2
    curl --silent --show-error --max-time 2 "$BASE/api/v1/operations/$OPERATION" >&2 || true
    exit 1
}

# Cancelling from the panel, which is the only thing a person can do to an operation. Agent1 is the
# executing authority here and confirms the teardown before answering, so this is the confirmed
# case: `200`, and a record that says cancelled because something observed it, not because somebody
# asked.
echo "==> cancelling the live operation the way the panel does"
for _ in $(seq 1 100); do
    curl -fsS --max-time 2 "$BASE/api/v1/operations" >"$WORK/list.json" 2>/dev/null || { sleep 0.1; continue; }
    SECOND_OPERATION="$(python3 - "$WORK/list.json" "$SECOND" <<'PYTHON'
import json, sys
matches = [item for item in json.load(open(sys.argv[1]))['operations']
           if item.get('subject', {}).get('type') == 'agent'
           and item['subject']['payload'].get('capsule_id') == sys.argv[2]]
if len(matches) == 1 and matches[0]['cancellable']:
    print(matches[0]['id'])
PYTHON
)" || SECOND_OPERATION=""
    [ -n "$SECOND_OPERATION" ] && break
    sleep 0.1
done
[ -n "$SECOND_OPERATION" ] || {
    echo "the second session never became a cancellable operation" >&2
    cat "$WORK/list.json" >&2
    exit 1
}

status="$(curl -sS --max-time 20 -o "$WORK/cancelled.json" -w '%{http_code}' \
    -X POST "$BASE/api/v1/operations/cancel" \
    -H 'content-type: application/json' \
    -d "{\"operationId\":\"$SECOND_OPERATION\"}")"
[ "$status" = "200" ] || {
    echo "cancelling an operation Agent1 tears down answered $status, not the confirmed 200" >&2
    cat "$WORK/cancelled.json" >&2
    exit 1
}

curl -fsS --max-time 5 "$BASE/api/v1/operations/$SECOND_OPERATION" >"$WORK/after-cancel.json"
python3 - "$WORK/after-cancel.json" <<'PYTHON'
import json, sys
operation = json.load(open(sys.argv[1]))
assert operation['state']['status'] == 'cancelled', operation
assert operation['cancellable'] is False, operation
assert operation['finishedAt'] is not None, operation
PYTHON

# And the host agrees: a cancellation that left the capsule running would be the panel saying an
# ending it did not get.
systemctl --user is-active --quiet "$SECOND_CAPSULE_UNIT" && {
    echo "the capsule is still running after its operation was cancelled" >&2
    exit 1
}
echo "    ok      200, the record says cancelled, and the capsule is gone from this host"

echo "=== Operation continuity gate passed: Agent1 identity survived gateway and Operation1 restarts, a vanished session became detached, and a cancellation from the panel reached the host ==="
