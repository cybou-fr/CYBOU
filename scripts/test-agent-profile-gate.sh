#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# The door a caller may be given, tested against a catalogue on disk.
#
# `launch` takes ceilings as arguments, which is right for bring-up by somebody sitting at the host.
# `start` is the shape a bus method or a web endpoint can have: name a profile, an agent, a workspace
# and one of the models that profile offers, and every bound comes from a file only root can write.
#
# The refusals are the substance. A caller that could name a workspace freely could name `/etc`; one
# that could name any agent could run a pack the ceilings were never approved for; one that could
# name any model could pick a class whose spending policy was written for a different class. Each of
# those is checked here against a real file rather than a fixture in a test, because the parsing, the
# lookup and the lexical path check all sit between the caller and the grant.
#
# Exit 3 means the catalogue cannot be placed where this build reads it, which is a check that did not
# run rather than one that passed.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

CATALOGUE=/etc/cybou/agent-profiles.json
ROOTS=/tmp/cybou-profile-gate

not_run() {
    echo "==> agent profile gate NOT RUN: $1" >&2
    exit 3
}

command -v bwrap >/dev/null || not_run "bubblewrap is not installed here"
command -v systemd-run >/dev/null || not_run "systemd-run is not available"
systemctl --user is-system-running >/dev/null 2>&1 || not_run "there is no user service manager"
test -x /usr/libexec/cybou/cybou-capsule-enter ||
    not_run "the capsule entry program is not installed"
mkdir -p /etc/cybou 2>/dev/null || not_run "/etc/cybou cannot be created here"
test -w /etc/cybou || not_run "/etc/cybou is not writable by this user"
# The directory a deployment keeps under /run, or wherever this run can write one. The owner reads
# the same override, so a gate that hard-coded the path would test a directory the owner is not
# reading the moment anybody sets it.
LEASES="${CYBOU_AGENT_LEASE_ROOT:-/run/cybou-agent-leases}"
export CYBOU_AGENT_LEASE_ROOT="$LEASES"
mkdir -p "$LEASES" 2>/dev/null || not_run "$LEASES cannot be created here"
[ -w "$LEASES" ] || not_run "$LEASES is not writable by this user"

RESTORE=""
if [ -e "$CATALOGUE" ]; then
    RESTORE="$(mktemp)"
    cp "$CATALOGUE" "$RESTORE"
fi
cleanup() {
    if [ -n "$RESTORE" ]; then
        cp "$RESTORE" "$CATALOGUE"
        rm -f "$RESTORE"
    else
        rm -f "$CATALOGUE"
    fi
    rm -rf "$ROOTS"
}
trap cleanup EXIT

install -m 0644 fixtures/agent-profiles-gate.json "$CATALOGUE"
mkdir -p "$ROOTS/app"

cargo build --quiet --locked -p cybou-agentd
AGENTD="$CARGO_TARGET_DIR/debug/cybou-agentd"

refused() {
    local why="$1"
    shift
    local output
    if output="$("$AGENTD" start "$@" -- /bin/true 2>&1)"; then
        echo "a request that should have been refused was granted: $*" >&2
        exit 1
    fi
    grep -q "$why" <<<"$output" || {
        echo "refused for the wrong reason; wanted '$why', got:" >&2
        echo "$output" >&2
        exit 1
    }
}

# The workspace is the one directory an agent may change. A caller supplying it freely could supply
# /etc, and a path that climbs out of a permitted root is inside it by spelling and outside it by
# meaning — which is the one that has to decide.
refused 'not under a directory this profile permits' \
    --profile local-work --agent gate --workspace /etc
refused 'not under a directory this profile permits' \
    --profile local-work --agent gate --workspace "$ROOTS/../../etc"

# Ceilings are approved for a pack, not in the abstract.
refused "does not run 'intruder'" \
    --profile local-work --agent intruder --workspace "$ROOTS/app"

# A class the profile does not offer, and a profile nobody approved. Both name the thing that was
# wrong rather than substituting something plausible.
refused "does not offer a 'Strong' model" \
    --profile local-work --agent gate --workspace "$ROOTS/app" --model Strong
refused "is not an approved profile" \
    --profile whatever-is-handy --agent gate --workspace "$ROOTS/app"

# And the request the catalogue does permit runs, with bounds the caller never named. No ceilings
# were asked for either: they bound a bearer, and this profile offers no model, so there is none.
CAPSULE="$(cat /proc/sys/kernel/random/uuid)"
"$AGENTD" start --profile local-work --agent gate --workspace "$ROOTS/app" \
    --capsule-id "$CAPSULE" \
    -- /bin/sh -c 'echo reached >/workspace/proof' >"$ROOTS/start.out" 2>&1 || {
    echo "the permitted request failed:" >&2
    cat "$ROOTS/start.out" >&2
    exit 1
}

grep -q reached "$ROOTS/app/proof" || {
    echo "the capsule did not run inside the workspace the profile permitted" >&2
    exit 1
}

python3 - "$ROOTS/start.out" <<'PYTHON'
import json, sys

views = [json.loads(line) for line in open(sys.argv[1]) if line.startswith("{")]
assert views, "the start printed no session"
view = views[0]
# Every one of these came from the file, and none from the command line.
assert view["profile"] == "local-work", view["profile"]
assert view["memoryMib"] == 512, view["memoryMib"]
assert view["cpus"] == 1, view["cpus"]
assert view["tasksMax"] == 64, view["tasksMax"]
assert view["hosts"] == [], view["hosts"]
assert view["modelClass"] is None, view["modelClass"]
PYTHON

test ! -e "$LEASES/$CAPSULE.lease" || {
    echo "teardown left the lease behind" >&2
    exit 1
}

echo "=== Agent profile gate passed: the profile decided, and the caller did not ==="
