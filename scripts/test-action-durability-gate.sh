#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Why did nginx restart on the fourteenth?
#
# That is the question durable authorization exists to answer, and it is answerable only if the
# proposal, the objections and the decision are still there after the process that made them is gone.
# Action1 held all three in memory, which made the causal chain a property of a process's uptime.
#
# The types and the replay were tested without a host. What was not tested is the part that only a
# host has: that Action1 actually writes to the Journal when Event1 is there, and actually reads its
# own history back before it answers anything. So this starts a real Event1, decides a real action
# against a real Action1, kills it, starts it again, and asks.
#
# Exit 3 means there is no session bus to run two daemons on, which is a check that did not run
# rather than one that passed.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

if [ -z "${CYBOU_ACTION_DURABILITY_SESSION:-}" ]; then
    if command -v dbus-run-session >/dev/null 2>&1; then
        exec env CYBOU_ACTION_DURABILITY_SESSION=1 dbus-run-session -- "$0" "$@"
    fi
    echo "==> action durability gate NOT RUN: dbus-run-session is not installed here" >&2
    exit 3
fi

command -v busctl >/dev/null || {
    echo "==> action durability gate NOT RUN: busctl is not available" >&2
    exit 3
}

WORK="$(mktemp -d)"
export XDG_STATE_HOME="$WORK/state"
export XDG_DATA_HOME="$WORK/data"
mkdir -p "$XDG_STATE_HOME/cybou" "$XDG_DATA_HOME/cybou"

eventd_pid=""
actiond_pid=""
cleanup() {
    test -z "$actiond_pid" || kill "$actiond_pid" 2>/dev/null || true
    test -z "$eventd_pid" || kill "$eventd_pid" 2>/dev/null || true
    wait 2>/dev/null || true
    rm -rf "$WORK"
}
trap cleanup EXIT

cargo build --quiet --locked -p cybou-eventd -p cybou-actiond
cargo build --quiet --locked -p cybou-actiond --example action-probe
BIN="$CARGO_TARGET_DIR/debug"
PROBE="$BIN/examples/action-probe"

wait_for_name() {
    for _ in $(seq 1 200); do
        if busctl --user list --no-legend 2>/dev/null | grep -q "^$1 "; then
            return 0
        fi
        sleep 0.1
    done
    echo "$1 never appeared on the bus" >&2
    return 1
}

"$BIN/cybou-eventd" >"$WORK/eventd.log" 2>&1 &
eventd_pid=$!
wait_for_name org.cybou.Mind.Event1 || {
    cat "$WORK/eventd.log" >&2
    exit 1
}

# The operator pre-authorizes the one operation this gate decides, so a decision is actually reached
# rather than refused for want of a standing policy. A refusal would be durable too, but it would not
# exercise the branch that issues a permit.
export CYBOU_PREAUTHORIZED_ACTIONS=service.restart

start_actiond() {
    "$BIN/cybou-actiond" >>"$WORK/actiond.log" 2>&1 &
    actiond_pid=$!
    wait_for_name org.cybou.Mind.Action1 || {
        cat "$WORK/actiond.log" >&2
        exit 1
    }
}
start_actiond

# One finding, decided. The insight is built the way Mind builds one, by this crate's own code, so
# what is recorded is a real lifecycle rather than a shape invented for a test.
PROPOSAL="$("$PROBE" decide)"
test -n "$PROPOSAL"

# It reached the Journal. Nothing else in this repository would put a PlanProposal there.
for _ in $(seq 1 100); do
    "$PROBE" journal >"$WORK/before" 2>/dev/null || echo 0 >"$WORK/before"
    [ "$(cat "$WORK/before")" -ge 2 ] && break
    sleep 0.1
done
recorded="$(cat "$WORK/before")"
[ "$recorded" -ge 2 ] || {
    echo "the lifecycle did not reach the Journal: $recorded contribution(s) from actiond" >&2
    cat "$WORK/actiond.log" >&2
    exit 1
}

# Now the process that decided it is gone.
kill "$actiond_pid"
wait "$actiond_pid" 2>/dev/null || true
actiond_pid=""
sleep 0.5

start_actiond
grep -q 'Restored 1 decided action' "$WORK/actiond.log" || {
    echo "the restarted owner did not read its own history back:" >&2
    cat "$WORK/actiond.log" >&2
    exit 1
}

# And it can still answer for what it authorized.
"$PROBE" record "$PROPOSAL" >"$WORK/remembered.json"
python3 - "$WORK/remembered.json" <<'PYTHON'
import json, sys

record = json.load(open(sys.argv[1]))
assert record["verdict"] == "granted", record
assert record["operation"] == "service.restart", record
assert record["checks"], "the criticism it was decided against is gone"
# The permit is deliberately not restored: a single-use capability reissued by a restart would have
# been granted by the crash rather than by anybody.
assert record["permitId"] is None, record["permitId"]
PYTHON

echo "=== Action durability gate passed: it still knows what it authorized ==="
