#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Agent1 controls are physical, ordered, and complete.
#
# This host gate recovers a real running session, drives Agent1 over D-Bus, and checks the kernel and
# both outbound runtime surfaces rather than believing the owner's projection. Exit 3 means the
# deployed-host preconditions are absent; a proof that did not run is never reported as a pass.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

not_run() {
    echo "==> agent control gate NOT RUN: $1" >&2
    exit 3
}

command -v gdbus >/dev/null || not_run "gdbus is not installed"
command -v systemd-run >/dev/null || not_run "systemd-run is not available"
command -v python3 >/dev/null || not_run "python3 is not installed"
systemctl --user is-system-running >/dev/null 2>&1 || not_run "there is no user service manager"
[ -n "${DBUS_SESSION_BUS_ADDRESS:-}" ] || not_run "there is no session bus"
test -f /etc/systemd/system/cybou-agent-gateway@.service || not_run "the gateway template is not deployed"
test -s /etc/cybou/provider.env || not_run "no provider policy is configured"
test -s /etc/cybou/litellm-master-key || not_run "no provider credential is installed"

LEASES=/run/cybou-agent-leases
mkdir -p "$LEASES" 2>/dev/null || not_run "$LEASES cannot be created"
[ -w "$LEASES" ] || not_run "$LEASES is not writable"

CAPSULE="$(cat /proc/sys/kernel/random/uuid)"
TASK="$(cat /proc/sys/kernel/random/uuid)"
CAPSULE_UNIT="cybou-capsule-$CAPSULE.service"
EGRESS_UNIT="cybou-egress-$CAPSULE.service"
GATEWAY_UNIT="cybou-agent-gateway@$CAPSULE.service"
RUNTIME="/run/cybou-agent-$CAPSULE"
WORK="$(mktemp -d)"
TICKS="$WORK/ticks"
serve_pid=""

cleanup() {
    test -z "$serve_pid" || kill "$serve_pid" 2>/dev/null || true
    systemctl --user thaw "$CAPSULE_UNIT" 2>/dev/null || true
    systemctl --user stop "$CAPSULE_UNIT" "$EGRESS_UNIT" 2>/dev/null || true
    systemctl stop "$GATEWAY_UNIT" 2>/dev/null || true
    rm -rf "$LEASES/$CAPSULE.lease" "$LEASES/$CAPSULE.env" "$WORK"
}
trap cleanup EXIT

cargo build --quiet --locked -p cybou-agentd
AGENTD="$CARGO_TARGET_DIR/debug/cybou-agentd"

CYBOU_PROFILE_ID=control-gate \
CYBOU_CAPSULE_ID="$CAPSULE" \
CYBOU_AGENT=opencode \
CYBOU_AGENT_WORKSPACE="$WORK" \
CYBOU_AGENT_LEASE_SECONDS=600 \
CYBOU_CAPSULE_MEMORY_MIB=512 \
CYBOU_CAPSULE_CPUS=1 \
CYBOU_CAPSULE_TASKS_MAX=64 \
CYBOU_CAPSULE_MAY_EXECUTE=yes \
CYBOU_EGRESS_HOSTS=example.com \
CYBOU_MODEL_CLASS=Strong \
CYBOU_MODEL_SPEND_LIMIT=100 \
    cargo run --quiet --locked -p cybou-capsule --example issue-lease -- "$LEASES/$CAPSULE.lease"

cat >"$LEASES/$CAPSULE.env" <<ENV
CYBOU_AGENT_TASK_ID=$TASK
CYBOU_MODEL_TOKEN_LIMIT=1000
CYBOU_MODEL_MAX_OUTPUT_TOKENS=32
CYBOU_MODEL_SENSITIVITY=1
ENV

: >"$TICKS"
systemd-run --user --quiet --collect --unit="${CAPSULE_UNIT%.service}" -- \
    /bin/sh -c "while true; do echo tick >> '$TICKS'; sleep 0.2; done"
systemd-run --user --quiet --collect --unit="${EGRESS_UNIT%.service}" -- /bin/sleep 600
systemctl start "$GATEWAY_UNIT" || {
    echo "the deployed model gateway could not be started through its policy" >&2
    exit 1
}

for _ in $(seq 1 100); do
    test -S "$RUNTIME/model.sock" && test -s "$RUNTIME/model-token" && break
    sleep 0.1
done
test -S "$RUNTIME/model.sock" && test -s "$RUNTIME/model-token" || {
    echo "the model gateway did not publish its socket and bearer" >&2
    exit 1
}

"$AGENTD" serve >"$WORK/serve.out" 2>&1 &
serve_pid=$!
for _ in $(seq 1 100); do
    grep -q 'Registered org.cybou.Runtime.Agent1' "$WORK/serve.out" 2>/dev/null && break
    sleep 0.1
done
grep -q 'Registered org.cybou.Runtime.Agent1' "$WORK/serve.out" || {
    cat "$WORK/serve.out" >&2
    exit 1
}

control() {
    local action="$1"
    gdbus call --session \
        --dest org.cybou.Runtime.Agent1 \
        --object-path /org/cybou/Runtime/Agent1 \
        --method org.cybou.Runtime.Agent1.Action "$CAPSULE" "$action" | grep -q true
}

standing_is() {
    local expected="$1"
    "$AGENTD" sessions >"$WORK/sessions.json"
    python3 - "$WORK/sessions.json" "$CAPSULE" "$expected" <<'PYTHON'
import json, sys
mine = [v for v in json.load(open(sys.argv[1])) if v["capsuleId"] == sys.argv[2]]
assert len(mine) == 1, mine
assert mine[0]["standing"] == sys.argv[3], mine[0]
PYTHON
}

sleep 1
before="$(wc -l <"$TICKS")"
control freeze
sleep 1
after_freeze="$(wc -l <"$TICKS")"
cgroup="$(systemctl --user show "$CAPSULE_UNIT" --property=ControlGroup --value)"
test "$(cat "/sys/fs/cgroup$cgroup/cgroup.freeze")" = 1
test "$((after_freeze - before))" -le 1
standing_is paused

control resume
sleep 1
after_resume="$(wc -l <"$TICKS")"
test "$(cat "/sys/fs/cgroup$cgroup/cgroup.freeze")" = 0
test "$after_resume" -gt "$after_freeze"
standing_is running

control quarantine
test "$(cat "/sys/fs/cgroup$cgroup/cgroup.freeze")" = 1
! systemctl --user is-active --quiet "$EGRESS_UNIT"
! systemctl is-active --quiet "$GATEWAY_UNIT"
test ! -e "$RUNTIME/model.sock"
test ! -e "$RUNTIME/model-token"
standing_is quarantined

"$AGENTD" stop --capsule-id "$CAPSULE" | grep -q stopped
! systemctl --user is-active --quiet "$CAPSULE_UNIT"
! systemctl --user is-active --quiet "$EGRESS_UNIT"
! systemctl is-active --quiet "$GATEWAY_UNIT"
test ! -e "$LEASES/$CAPSULE.lease"
test ! -e "$LEASES/$CAPSULE.env"

echo "=== Agent control gate passed: freeze, resume, quarantine and stop reached every boundary ==="
