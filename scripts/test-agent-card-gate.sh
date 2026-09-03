#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# What a browser is told about running agents, and who tells it.
#
# The whole point of the route under test is that it is a proxy. It does not read the launch
# directory, ask a service manager, or assemble a session from a lease and a plan — the owner does
# all of that, and a second thing doing it would be a second answer to *what is running*. So this
# gate does not check that the endpoint produces plausible JSON. It compares the JSON against the
# owner's own answer, then stops that session through the browser route and requires the owner's
# retained final view. When the owner is gone, the endpoint must say so rather than inventing empty.
#
# An empty list is the failure worth guarding. "No agents are running" and "I could not ask" look
# identical on a card, and only one of them means a person can stop worrying.
#
# Exit 3 means this host cannot run an owner and a gateway together, which is a check that did not
# run rather than one that passed.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

not_run() {
    echo "==> agent card gate NOT RUN: $1" >&2
    exit 3
}

command -v systemd-run >/dev/null || not_run "systemd-run is not available"
command -v curl >/dev/null || not_run "curl is not available"
command -v python3 >/dev/null || not_run "no python3 to read a listing with"
systemctl --user is-system-running >/dev/null 2>&1 || not_run "there is no user service manager"
[ -n "${DBUS_SESSION_BUS_ADDRESS:-}" ] || not_run "there is no session bus to take a name on"

# The directory a deployment keeps under /run, or wherever this run can write one. The owner reads
# the same override, so a gate that hard-coded the path would test a directory the owner is not
# reading the moment anybody sets it.
LEASES="${CYBOU_AGENT_LEASE_ROOT:-/run/cybou-agent-leases}"
export CYBOU_AGENT_LEASE_ROOT="$LEASES"
mkdir -p "$LEASES" 2>/dev/null || not_run "$LEASES cannot be created here"
[ -w "$LEASES" ] || not_run "$LEASES is not writable by this user"

CAPSULE="$(cat /proc/sys/kernel/random/uuid)"
TASK="$(cat /proc/sys/kernel/random/uuid)"
CAPSULE_UNIT="cybou-capsule-$CAPSULE"
PORT=18787
WORK="$(mktemp -d)"
serve_pid=""
web_pid=""

cleanup() {
    test -z "$web_pid" || kill "$web_pid" 2>/dev/null || true
    test -z "$serve_pid" || kill "$serve_pid" 2>/dev/null || true
    systemctl --user stop "$CAPSULE_UNIT.service" 2>/dev/null || true
    rm -rf "$LEASES/$CAPSULE.lease" "$LEASES/$CAPSULE.env" "$WORK"
}
trap cleanup EXIT

cargo build --quiet --locked -p cybou-agentd -p cybou-web-gateway
BIN="$CARGO_TARGET_DIR/debug"

CYBOU_PROFILE_ID=card-gate \
CYBOU_CAPSULE_ID="$CAPSULE" \
CYBOU_AGENT=opencode \
CYBOU_AGENT_WORKSPACE="$WORK" \
CYBOU_AGENT_LEASE_SECONDS=600 \
CYBOU_CAPSULE_MEMORY_MIB=512 \
CYBOU_CAPSULE_CPUS=1 \
CYBOU_CAPSULE_TASKS_MAX=64 \
CYBOU_CAPSULE_MAY_EXECUTE=yes \
CYBOU_MODEL_CLASS=Strong \
CYBOU_MODEL_SPEND_LIMIT=100 \
    cargo run --quiet --locked -p cybou-capsule --example issue-lease -- "$LEASES/$CAPSULE.lease"

cat >"$LEASES/$CAPSULE.env" <<ENV
CYBOU_AGENT_TASK_ID=$TASK
CYBOU_MODEL_TOKEN_LIMIT=1000
CYBOU_MODEL_MAX_OUTPUT_TOKENS=32
CYBOU_MODEL_SENSITIVITY=1
ENV

systemd-run --user --quiet --collect --unit="$CAPSULE_UNIT" -- /bin/sleep 300

"$BIN/cybou-agentd" serve >"$WORK/serve.out" 2>&1 &
serve_pid=$!
for _ in $(seq 1 100); do
    grep -q 'Registered org.cybou.Runtime.Agent1' "$WORK/serve.out" 2>/dev/null && break
    sleep 0.1
done
grep -q 'Registered org.cybou.Runtime.Agent1' "$WORK/serve.out" || {
    cat "$WORK/serve.out" >&2
    exit 1
}

(cd "$WORK" && CYBOU_SESSION_MODE=local-desktop CYBOU_GATEWAY_ADDR="127.0.0.1:$PORT" \
    "$BIN/cybou-web-gateway" >"$WORK/web.out" 2>&1) &
web_pid=$!
for _ in $(seq 1 100); do
    curl --silent --fail "http://127.0.0.1:$PORT/api/v1/agents" >/dev/null 2>&1 && break
    sleep 0.1
done

curl --silent --show-error --fail "http://127.0.0.1:$PORT/api/v1/agents" >"$WORK/card.json" || {
    echo "the agents route did not answer:" >&2
    cat "$WORK/web.out" >&2
    exit 1
}

# It is the owner's answer, not a second assembly of the same facts. Compared field by field against
# what the owner says directly, because "looks like a session" is exactly what a second assembler
# would also produce.
"$BIN/cybou-agentd" sessions >"$WORK/owner.json"
python3 - "$WORK/card.json" "$WORK/owner.json" "$CAPSULE" <<'PYTHON'
import json, sys

card = json.load(open(sys.argv[1]))
owner = json.load(open(sys.argv[2]))
assert card == owner, "the browser is being told something the owner did not say"

mine = [view for view in card if view["capsuleId"] == sys.argv[3]]
assert len(mine) == 1, f"expected one session, found {len(card)}"
view = mine[0]
assert view["standing"] == "running", view["standing"]
assert view["memoryMib"] == 512, view["memoryMib"]
# The ceilings are what was granted. Nothing here is a reading of what the capsule is using, and a
# card that showed one would be inventing the thing a person is watching for.
assert view["cpus"] == 1 and view["tasksMax"] == 64, view
# No gateway has published a ledger, so no figure is claimed. A nought would say, of a session that
# may have been billed, that it spent nothing.
assert view["spend"]["spent"] is None, view["spend"]
assert view["spendObservedAt"] is None, view["spendObservedAt"]
assert view["startedAt"] and view["expiresAt"], view
PYTHON

# HTTP success means the owner confirmed teardown. The next listing is the owner's final view rather
# than an optimistic browser-side transition.
status="$(curl --silent --output "$WORK/stopped-body.json" --write-out '%{http_code}' \
    --request DELETE "http://127.0.0.1:$PORT/api/v1/agents/$CAPSULE")"
test "$status" = "204" || {
    echo "the Stop route answered $status:" >&2
    cat "$WORK/stopped-body.json" >&2
    exit 1
}
systemctl --user is-active --quiet "$CAPSULE_UNIT.service" && {
    echo "the Stop route returned success while the capsule unit was still active" >&2
    exit 1
}

curl --silent --show-error --fail "http://127.0.0.1:$PORT/api/v1/agents" >"$WORK/ended-card.json"
"$BIN/cybou-agentd" sessions >"$WORK/ended-owner.json"
python3 - "$WORK/ended-card.json" "$WORK/ended-owner.json" "$CAPSULE" <<'PYTHON'
import json, sys

card = json.load(open(sys.argv[1]))
owner = json.load(open(sys.argv[2]))
assert card == owner, "the browser invented a final view the owner did not report"
mine = [view for view in card if view["capsuleId"] == sys.argv[3]]
assert len(mine) == 1, mine
assert mine[0]["standing"] == "ended", mine[0]
assert mine[0]["endedBecause"] == "the session was stopped", mine[0]
assert mine[0]["endedAt"], mine[0]
PYTHON

# And with the owner gone, the endpoint says it could not ask. An empty list here would be the one
# answer a person cannot act on: "nothing is running" and "I could not find out" look the same on a
# card, and only one of them means they can stop worrying.
kill "$serve_pid"
wait "$serve_pid" 2>/dev/null || true
serve_pid=""
sleep 0.5

status="$(curl --silent --output "$WORK/absent.json" --write-out '%{http_code}' \
    "http://127.0.0.1:$PORT/api/v1/agents")"
test "$status" = "503" || {
    echo "with no owner the endpoint answered $status:" >&2
    cat "$WORK/absent.json" >&2
    exit 1
}
grep -q 'agentRuntimeUnavailable' "$WORK/absent.json" || {
    echo "the refusal does not say what was wrong:" >&2
    cat "$WORK/absent.json" >&2
    exit 1
}
# And it says nothing about this host's insides to somebody who may not be entitled to know them.
! grep -qi 'dbus\|socket\|/run/' "$WORK/absent.json"

echo "=== Agent card gate passed: browser read and Stop stay the owner's canonical answer ==="
