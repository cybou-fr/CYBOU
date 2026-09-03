#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# A capsule, driven the way a person drives one: through HTTP and nothing else.
#
# The control gate beside this one proves the same boundaries over D-Bus, and it needs a deployed
# gateway template and a provider credential, so it reports NOT RUN nearly everywhere. This one
# needs neither. What it exercises is the path the desktop takes — list, telemetry, freeze, resume,
# quarantine, stop — and the thing that path kept getting wrong: what a person is told when the
# answer is no.
#
# Every claim is checked against the kernel rather than against the owner's projection: a freeze is
# `cgroup.freeze` reading 1 and the capsule's own output stopping, and a resume is both going back.
#
# Exit 3 means this host has no user service manager, session bus or writable lease directory to
# run it on.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

not_run() { echo "==> agent desktop gate NOT RUN: $1" >&2; exit 3; }
for command in systemd-run systemctl python3 curl; do
    command -v "$command" >/dev/null || not_run "$command is not installed"
done
systemctl --user is-system-running >/dev/null 2>&1 || not_run "there is no user service manager"
[ -n "${DBUS_SESSION_BUS_ADDRESS:-}" ] || not_run "there is no session bus"

WORK="$(mktemp -d)"
# The lease directory a deployment keeps under /run, put where a proof can write it. The owner reads
# this override for exactly this reason; nothing deployed sets it.
export CYBOU_AGENT_LEASE_ROOT="$WORK/leases"
mkdir -p "$CYBOU_AGENT_LEASE_ROOT"

CAPSULE="$(cat /proc/sys/kernel/random/uuid)"
TASK="$(cat /proc/sys/kernel/random/uuid)"
CAPSULE_UNIT="cybou-capsule-$CAPSULE.service"
EGRESS_UNIT="cybou-egress-$CAPSULE.service"
TICKS="$WORK/ticks"
UID_NUMBER="$(id -u)"
CGROUP="/sys/fs/cgroup/user.slice/user-$UID_NUMBER.slice/user@$UID_NUMBER.service/app.slice/$CAPSULE_UNIT"
agent_pid=""
gateway_pid=""

cleanup() {
    test -z "$gateway_pid" || kill "$gateway_pid" 2>/dev/null || true
    test -z "$agent_pid" || kill "$agent_pid" 2>/dev/null || true
    systemctl --user thaw "$CAPSULE_UNIT" 2>/dev/null || true
    systemctl --user stop "$CAPSULE_UNIT" "$EGRESS_UNIT" 2>/dev/null || true
    rm -rf "$WORK"
}
trap cleanup EXIT

cargo build --quiet --locked -p cybou-agentd -p cybou-web-gateway
AGENTD="$CARGO_TARGET_DIR/debug/cybou-agentd"
GATEWAY="$CARGO_TARGET_DIR/debug/cybou-web-gateway"

# One admitted session, as an operator's profile would have granted it.
CYBOU_PROFILE_ID=desktop-gate \
CYBOU_CAPSULE_ID="$CAPSULE" \
CYBOU_AGENT=opencode \
CYBOU_AGENT_WORKSPACE="$WORK" \
CYBOU_AGENT_LEASE_SECONDS=600 \
CYBOU_CAPSULE_MEMORY_MIB=256 \
CYBOU_CAPSULE_CPUS=1 \
CYBOU_CAPSULE_TASKS_MAX=32 \
CYBOU_CAPSULE_MAY_EXECUTE=yes \
CYBOU_EGRESS_HOSTS=example.com \
CYBOU_MODEL_CLASS=Strong \
CYBOU_MODEL_SPEND_LIMIT=100 \
    cargo run --quiet --locked -p cybou-capsule --example issue-lease -- \
    "$CYBOU_AGENT_LEASE_ROOT/$CAPSULE.lease"

cat >"$CYBOU_AGENT_LEASE_ROOT/$CAPSULE.env" <<ENV
CYBOU_AGENT_TASK_ID=$TASK
CYBOU_MODEL_TOKEN_LIMIT=1000
CYBOU_MODEL_MAX_OUTPUT_TOKENS=32
CYBOU_MODEL_SENSITIVITY=1
ENV

# Something inside the capsule that is doing work, so a freeze is observable as work stopping rather
# than as a flag changing.
: >"$TICKS"
systemd-run --user --quiet --collect --unit="${CAPSULE_UNIT%.service}" -- \
    /bin/sh -c "while true; do echo tick >> '$TICKS'; sleep 0.2; done"
systemd-run --user --quiet --collect --unit="${EGRESS_UNIT%.service}" -- /bin/sleep 600
[ -f "$CGROUP/cgroup.freeze" ] || not_run "this host does not expose cgroup.freeze for user units"

"$AGENTD" serve >"$WORK/agentd.log" 2>&1 &
agent_pid=$!
for _ in $(seq 1 150); do
    grep -q 'Registered org.cybou.Runtime.Agent1' "$WORK/agentd.log" 2>/dev/null && break
    sleep 0.1
done
grep -q 'Registered org.cybou.Runtime.Agent1' "$WORK/agentd.log" || {
    cat "$WORK/agentd.log" >&2
    exit 1
}

PORT="$(python3 - <<'PYTHON'
import socket
with socket.socket() as sock:
    sock.bind(('127.0.0.1', 0))
    print(sock.getsockname()[1])
PYTHON
)"
BASE="http://127.0.0.1:$PORT"
CYBOU_SESSION_MODE=local-desktop CYBOU_GATEWAY_ADDR="127.0.0.1:$PORT" \
    "$GATEWAY" >"$WORK/gateway.log" 2>&1 &
gateway_pid=$!
for _ in $(seq 1 150); do
    curl -fsS --max-time 2 "$BASE/api/v1/session" >/dev/null 2>&1 && break
    sleep 0.1
done
curl -fsS --max-time 2 "$BASE/api/v1/session" >/dev/null

control() {
    curl -sS --max-time 10 -o "$WORK/control.json" -w '%{http_code}' \
        -X POST "$BASE/api/v1/agents/$CAPSULE/action" \
        -H 'content-type: application/json' \
        -d "{\"action\":\"$1\"}"
}

freeze_is() {
    test "$(cat "$CGROUP/cgroup.freeze")" = "$1" || {
        echo "the kernel says cgroup.freeze is $(cat "$CGROUP/cgroup.freeze"), not $1" >&2
        exit 1
    }
}

echo "=== The desktop can see the session, and what it is doing ==="
curl -fsS --max-time 5 "$BASE/api/v1/agents" >"$WORK/agents.json"
python3 - "$WORK/agents.json" "$CAPSULE" <<'PYTHON'
import json, sys
sessions = json.load(open(sys.argv[1]))
mine = [s for s in sessions if s["capsuleId"] == sys.argv[2]]
assert len(mine) == 1, sessions
assert mine[0]["standing"] in ("launching", "running"), mine[0]
PYTHON
curl -fsS --max-time 5 "$BASE/api/v1/agents/$CAPSULE/telemetry" >"$WORK/telemetry.json"
python3 - "$WORK/telemetry.json" <<'PYTHON'
import json, sys
record = json.load(open(sys.argv[1]))["telemetry"]
# Readings taken from the capsule's own cgroup, and each one saying whether it was read at all: a
# shape full of zeroes would be this panel inventing a quiet capsule.
assert record["pidsCount"]["state"] == "known", record
assert record["pidsCount"]["value"] >= 1, record
assert record["memoryUsedMib"]["state"] == "known", record
# And what could not be read says so rather than reading as none.
assert record["egressRequestsCount"]["state"] == "unavailable", record
assert record["egressRequestsCount"]["value"] is None, record
PYTHON
echo "    ok      the panel lists it and reads its telemetry"

echo "=== Freeze stops the work, and the kernel agrees ==="
sleep 1
before="$(wc -l <"$TICKS")"
[ "$(control freeze)" = "200" ] || { echo "freeze was refused: $(cat "$WORK/control.json")" >&2; exit 1; }
freeze_is 1
sleep 1
after_freeze="$(wc -l <"$TICKS")"
test "$((after_freeze - before))" -le 1 || {
    echo "the capsule kept working after it was frozen" >&2
    exit 1
}
echo "    ok      frozen: cgroup.freeze is 1 and the capsule stopped writing"

[ "$(control resume)" = "200" ] || { echo "resume was refused: $(cat "$WORK/control.json")" >&2; exit 1; }
freeze_is 0
sleep 1
test "$(wc -l <"$TICKS")" -gt "$after_freeze" || {
    echo "the capsule never started working again" >&2
    exit 1
}
echo "    ok      resumed: cgroup.freeze is 0 and the capsule is working again"

echo "=== Quarantine freezes it and takes its network away ==="
[ "$(control quarantine)" = "200" ] || {
    echo "quarantine was refused: $(cat "$WORK/control.json")" >&2
    exit 1
}
freeze_is 1
systemctl --user is-active --quiet "$EGRESS_UNIT" && {
    echo "the egress broker is still running for a quarantined capsule" >&2
    exit 1
}
echo "    ok      quarantined: frozen, and its egress broker is gone"

# The part this gate was written for. Releasing a quarantine is refused by the owner, and a person
# has to be told that rather than told to try again: those are different sentences and used to be
# the same one.
echo "=== And a refusal says whose it is ==="
status="$(control resume)"
[ "$status" = "409" ] || {
    echo "releasing a quarantine answered $status where the owner's refusal was 409" >&2
    cat "$WORK/control.json" >&2
    exit 1
}
python3 - "$WORK/control.json" <<'PYTHON'
import json, sys
body = json.load(open(sys.argv[1]))
assert body["error"] == "agentActionNotEstablished", body
assert body["retryable"] is False, "a person was told to retry a refusal by design"
# The owner's own words, not the boundary's summary of them.
assert "quarantine" in body.get("detail", "").lower(), body
PYTHON
freeze_is 1
echo "    ok      409 with the owner's own reason, and the capsule stayed quarantined"

echo "=== Stopping it leaves nothing behind ==="
code="$(curl -sS --max-time 20 -o /dev/null -w '%{http_code}' -X DELETE "$BASE/api/v1/agents/$CAPSULE")"
[ "$code" = "204" ] || { echo "stop answered $code" >&2; exit 1; }
for _ in $(seq 1 100); do
    systemctl --user is-active --quiet "$CAPSULE_UNIT" || break
    sleep 0.1
done
systemctl --user is-active --quiet "$CAPSULE_UNIT" && {
    echo "the capsule unit is still running after stop" >&2
    exit 1
}
test ! -e "$CYBOU_AGENT_LEASE_ROOT/$CAPSULE.lease" || {
    echo "the lease outlived the session it granted" >&2
    exit 1
}
curl -fsS --max-time 5 "$BASE/api/v1/agents" >"$WORK/after.json"
python3 - "$WORK/after.json" "$CAPSULE" <<'PYTHON'
import json, sys
live = [
    s for s in json.load(open(sys.argv[1]))
    if s["capsuleId"] == sys.argv[2] and s["standing"] in ("launching", "running", "paused", "quarantined")
]
assert not live, live
PYTHON
echo "    ok      unit gone, lease gone, and the panel no longer shows it live"

echo "=== Agent desktop gate passed: a capsule driven through HTTP, checked against the kernel ==="
