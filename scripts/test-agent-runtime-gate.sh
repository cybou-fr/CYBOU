#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# The owner comes back and finds the agent that kept working.
#
# Everything about that claim had been checked without a host: the registry decides correctly, the
# parser reads what the writer wrote, the service answers when called directly. None of it had ever
# been *run* — no bus name taken, no launch directory read, no service manager asked. Code that is
# only ever right on paper accumulates, and the first real host is a bad place to find out which part
# of it was wrong.
#
# So this starts a real daemon against a real user service manager: it writes a session's two files
# the way a launch would, starts a unit named the way a capsule's is, and then asks the owner over
# D-Bus what is running. Then it publishes a ledger and checks the figure arrives with the instant it
# was observed. Then it stops the session and checks nothing is left.
#
# Exit 3 means this host cannot run one, which is a check that did not run rather than one that
# passed.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

not_run() {
    echo "==> agent runtime gate NOT RUN: $1" >&2
    exit 3
}

command -v systemd-run >/dev/null || not_run "systemd-run is not available"
command -v python3 >/dev/null || not_run "no python3 to read a listing with"
systemctl --user is-system-running >/dev/null 2>&1 || not_run "there is no user service manager"
[ -n "${DBUS_SESSION_BUS_ADDRESS:-}" ] || not_run "there is no session bus to take a name on"

LEASES=/run/cybou-agent-leases
mkdir -p "$LEASES" 2>/dev/null || not_run "$LEASES cannot be created here"
[ -w "$LEASES" ] || not_run "$LEASES is not writable by this user"

CAPSULE="$(cat /proc/sys/kernel/random/uuid)"
TASK="$(cat /proc/sys/kernel/random/uuid)"
CAPSULE_UNIT="cybou-capsule-$CAPSULE"
RUNTIME="/run/cybou-agent-$CAPSULE"
serve_pid=""

cleanup() {
    test -z "$serve_pid" || kill "$serve_pid" 2>/dev/null || true
    systemctl --user stop "$CAPSULE_UNIT.service" 2>/dev/null || true
    rm -rf "$LEASES/$CAPSULE.lease" "$LEASES/$CAPSULE.env" "$RUNTIME" "$WORK"
}
WORK="$(mktemp -d)"
trap cleanup EXIT

cargo build --quiet --locked -p cybou-agentd
AGENTD="$CARGO_TARGET_DIR/debug/cybou-agentd"

# The lease is minted by the one public mint, exactly as a launch would mint it.
CYBOU_PROFILE_ID=runtime-gate \
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

# The launch file, in exactly the shape plan() writes it. If this ever stops matching, the parser's
# round-trip test fails first and this gate fails second, which is the order that names the cause.
cat >"$LEASES/$CAPSULE.env" <<ENV
CYBOU_AGENT_TASK_ID=$TASK
CYBOU_MODEL_TOKEN_LIMIT=1000
CYBOU_MODEL_MAX_OUTPUT_TOKENS=32
CYBOU_MODEL_SENSITIVITY=1
ENV

# A unit named the way a capsule's is. Standing in for a capsule rather than being one: what is under
# test here is that the owner asks the service manager and believes the answer, and a real capsule
# would test bubblewrap instead.
systemd-run --user --quiet --collect --unit="$CAPSULE_UNIT" -- /bin/sleep 300

"$AGENTD" serve >"$WORK/serve.out" 2>&1 &
serve_pid=$!
for _ in $(seq 1 100); do
    grep -q 'Registered org.cybou.Runtime.Agent1' "$WORK/serve.out" 2>/dev/null && break
    sleep 0.1
done
grep -q 'Registered org.cybou.Runtime.Agent1' "$WORK/serve.out" || {
    echo "the owner never took its bus name:" >&2
    cat "$WORK/serve.out" >&2
    exit 1
}

listing() {
    "$AGENTD" sessions
}

# It found the session that was already running, which is the whole claim.
listing >"$WORK/listing.json"
python3 - "$WORK/listing.json" "$CAPSULE" <<'PYTHON'
import json, sys

views = json.load(open(sys.argv[1]))
mine = [view for view in views if view["capsuleId"] == sys.argv[2]]
assert len(mine) == 1, f"expected one session, found {len(views)}"
view = mine[0]
assert view["standing"] == "running", view["standing"]
assert view["agent"] == "opencode", view["agent"]
assert view["profile"] == "runtime-gate", view["profile"]
assert view["modelClass"] == "Strong", view["modelClass"]
# Nothing has published a ledger, so nothing is claimed about spending. A nought here would state,
# of a session that may have been billed, that it spent nothing.
assert view["spend"]["spent"] is None, view["spend"]
assert view["spendObservedAt"] is None, view["spendObservedAt"]
assert view["startedAt"] and view["expiresAt"], view
PYTHON

# Now a gateway publishes what it spent, and the figure arrives with the instant it was observed.
mkdir -p "$RUNTIME"
cat >"$RUNTIME/model-usage.json" <<USAGE
{"capsuleId":"$CAPSULE","spendUnits":42,"tokens":1234,"completions":3,"observedAt":"2026-08-26T10:00:00Z"}
USAGE

for _ in $(seq 1 60); do
    listing >"$WORK/spent.json" 2>/dev/null || true
    grep -q '"spent": 42' "$WORK/spent.json" 2>/dev/null && break
    sleep 0.5
done
python3 - "$WORK/spent.json" "$CAPSULE" <<'PYTHON'
import json, sys

view = [v for v in json.load(open(sys.argv[1])) if v["capsuleId"] == sys.argv[2]][0]
assert view["spend"]["spent"] == 42, view["spend"]
assert view["spendObservedAt"].startswith("2026-08-26T10:00:00"), view["spendObservedAt"]
PYTHON

# And stopping it through the owner ends the unit and leaves nothing behind.
"$AGENTD" stop --capsule-id "$CAPSULE" | grep -q "stopped"

for _ in $(seq 1 60); do
    systemctl --user is-active --quiet "$CAPSULE_UNIT.service" || break
    sleep 0.5
done
if systemctl --user is-active --quiet "$CAPSULE_UNIT.service"; then
    echo "the capsule unit is still running after Stop" >&2
    exit 1
fi
for leftover in "$LEASES/$CAPSULE.lease" "$LEASES/$CAPSULE.env"; do
    test ! -e "$leftover" || {
        echo "teardown left $leftover behind" >&2
        exit 1
    }
done

listing >"$WORK/after.json"
python3 - "$WORK/after.json" "$CAPSULE" <<'PYTHON'
import json, sys

assert not [v for v in json.load(open(sys.argv[1])) if v["capsuleId"] == sys.argv[2]], (
    "a stopped session is still listed as running"
)
PYTHON

echo "=== Agent runtime gate passed: the owner found it, reported it, and ended it ==="
