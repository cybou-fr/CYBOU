#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# A whole launch, carried out, with no model anywhere in it.
#
# This is the case that says an Agent Capsule is a bounded place to compute rather than a container
# that only exists around a model — and for a while it was not merely untested but impossible, because
# planning refused a lease with no model grant outright. Every local, unplugged, model-free capsule
# was unlaunchable and nothing noticed, because nothing ran a launch.
#
# It also needs no provider, no credential and no gateway, which is why it is the part of `launch`
# that can be proven on an ordinary host. `scripts/test-agent-launch-gate.sh` covers the half that
# needs a deployed gateway; this covers the half that does not, and between them the launch path is
# not a thing that has only ever been read.
#
# Exit 3 means the capsule's host programs are not installed here, which is a check that did not run
# rather than one that passed.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

not_run() {
    echo "==> capsule launch gate NOT RUN: $1" >&2
    exit 3
}

command -v bwrap >/dev/null || not_run "bubblewrap is not installed here"
command -v systemd-run >/dev/null || not_run "systemd-run is not available"
systemctl --user is-system-running >/dev/null 2>&1 || not_run "there is no user service manager"
test -x /usr/libexec/cybou/cybou-capsule-enter ||
    not_run "the capsule entry program is not installed"

# The directory a deployment keeps under /run, or wherever this run can write one. The owner reads
# the same override, so a gate that hard-coded the path would test a directory the owner is not
# reading the moment anybody sets it.
LEASES="${CYBOU_AGENT_LEASE_ROOT:-/run/cybou-agent-leases}"
export CYBOU_AGENT_LEASE_ROOT="$LEASES"
mkdir -p "$LEASES" 2>/dev/null || not_run "$LEASES cannot be created here"
[ -w "$LEASES" ] || not_run "$LEASES is not writable by this user"

WORKSPACE="$(mktemp -d)"
OUTSIDE="$(mktemp -d)"
trap 'rm -rf "$WORKSPACE" "$OUTSIDE"' EXIT
chmod 700 "$WORKSPACE"
printf '%s\n' "not the agent's to change" >"$OUTSIDE/untouchable"

cargo build --quiet --locked -p cybou-agentd
AGENTD="$CARGO_TARGET_DIR/debug/cybou-agentd"

CAPSULE="$(cat /proc/sys/kernel/random/uuid)"

# No --model, and no token ceilings either: they bound a bearer, and there is no bearer. A launch
# that demanded them here would be refusing every capsule that was never going to ask a model
# anything, which is the ordinary case on a host with no provider at all.
"$AGENTD" launch \
    --profile capsule-launch-gate --agent gate --workspace "$WORKSPACE" \
    --memory-mib 512 --cpus 1 --tasks-max 64 --lifetime-seconds 120 \
    --token-limit 1 --max-output-tokens 1 --sensitivity 0 \
    --may-execute --capsule-id "$CAPSULE" \
    -- /bin/sh -c 'echo reached >/workspace/proof' >"$WORKSPACE/../launch.out" 2>&1 ||
    {
        echo "the launch failed:" >&2
        cat "$WORKSPACE/../launch.out" >&2
        exit 1
    }

# It ran, and it ran in the directory it was granted.
grep -q reached "$WORKSPACE/proof" || {
    echo "the capsule did not write inside its granted workspace" >&2
    cat "$WORKSPACE/../launch.out" >&2
    exit 1
}

# The session reported itself with no model and no gateway. A gateway named here would be a unit
# nobody started, and a spend of nought would be a claim about a bearer that never existed.
python3 - "$WORKSPACE/../launch.out" "$CAPSULE" <<'PYTHON'
import json, sys

lines = [line for line in open(sys.argv[1]) if line.startswith("{")]
assert lines, "the launch printed no session at all"
views = [json.loads(line) for line in lines]
assert all(view["capsuleId"] == sys.argv[2] for view in views), views
first, last = views[0], views[-1]
assert first["standing"] == "running", first["standing"]
assert last["standing"] == "ended", last["standing"]
assert last["endedBecause"] == "the agent finished", last["endedBecause"]
for view in views:
    assert view["modelClass"] is None, view["modelClass"]
    assert view["spend"] is None, view["spend"]
    assert view["spendObservedAt"] is None, view["spendObservedAt"]
    assert not [unit for unit in view["units"] if "gateway" in unit], view["units"]
    assert not [unit for unit in view["units"] if "egress" in unit], view["units"]
PYTHON

# A clean run says nothing about teardown. A capsule run to completion has already exited, so a
# teardown that judged itself by a stop command's exit code printed a failure every time — which is
# how an operator learns to ignore the one that matters.
! grep -q 'did not complete' "$WORKSPACE/../launch.out" || {
    echo "a clean session reported a teardown failure:" >&2
    grep 'did not complete' "$WORKSPACE/../launch.out" >&2
    exit 1
}

# And nothing is left on the host.
for leftover in "$LEASES/$CAPSULE.lease" "$LEASES/$CAPSULE.env" "/run/cybou-session-$CAPSULE"; do
    test ! -e "$leftover" || {
        echo "teardown left $leftover behind" >&2
        exit 1
    }
done
! systemctl --user is-active --quiet "cybou-capsule-$CAPSULE.service" || {
    echo "the capsule unit is still running" >&2
    exit 1
}

# What was outside the grant stayed outside it. Not a substitute for the escape gate, which attacks a
# capsule properly — this only asserts that an ordinary launch grants one directory and not two.
grep -q "not the agent's to change" "$OUTSIDE/untouchable"

echo "=== Capsule launch gate passed: a session with no model ran, and left nothing ==="
