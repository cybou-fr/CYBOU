#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# One selection, one lease, one set of names.
#
# The thing this gate exists to prevent has already happened once: the model gateway rebuilt its own
# lease from environment values, so a launch file and a running capsule could each be internally
# valid and still describe different permissions. Nothing downstream could say which of the two a
# person had approved.
#
# So the checks below are not "does the planner produce output". They are: does every runtime name
# come from the same identity, does the launch file carry nothing that is authority, and is the
# teardown ordered so the untrusted party loses its hands before anything else is taken away.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

CAPSULE=00000000-0000-0000-0000-0000000007a1
TASK=00000000-0000-0000-0000-0000000007a2

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

plan() {
    cargo run --quiet --locked -p cybou-agentd -- plan \
        --profile sandboxed-autonomous --agent opencode --workspace /srv/project \
        --memory-mib 4096 --cpus 2 --tasks-max 512 --lifetime-seconds 14400 \
        --capsule-id "$CAPSULE" --task-id "$TASK" --may-execute \
        --host github.com --host registry.npmjs.org "$@"
}

plan --token-limit 200000 --max-output-tokens 4096 --sensitivity 1 \
    --model Strong --spend-limit zero-cost >"$work/plan"

# Every name is the capsule's own identity. A gateway instance named separately from the capsule it
# serves is a pair nobody can match up again from a service manager's list.
for line in gateway-unit capsule-unit egress-unit lease-file launch-file model-socket model-token egress-socket; do
    grep -q "^$line .*$CAPSULE" "$work/plan" || {
        echo "$line does not carry the session identity" >&2
        exit 1
    }
done

# The launch file carries nothing the lease already says. Each name below, written here, could
# disagree with the approved lease.
for authority in CYBOU_CAPSULE_ID CYBOU_AGENT_WORKSPACE CYBOU_AGENT_LEASE_SECONDS \
    CYBOU_MODEL_CLASS CYBOU_MODEL_SPEND_LIMIT; do
    if grep "^launch-env " "$work/plan" | grep -q "$authority"; then
        echo "$authority is authority and must live on the lease alone" >&2
        exit 1
    fi
done
grep -q "^launch-env CYBOU_AGENT_TASK_ID=$TASK\$" "$work/plan"

# The capsule stops first. Taking its gateway away first is a refusal it can see and retry; taking
# the capsule away first is an ending it cannot.
grep '^teardown ' "$work/plan" | cut -d' ' -f2 >"$work/order"
expected=$'stop-capsule
stop-gateway
stop-egress
remove
remove
remove'
test "$(cat "$work/order")" = "$expected" || {
    echo "teardown is out of order:" >&2
    cat "$work/order" >&2
    exit 1
}

# `--spend-limit zero-cost` is a real selection — spend nothing, on a route that costs nothing — and
# it plans. This is the free-model case the whole provider catalogue exists for, and it is the one
# selection an earlier version of this system could never serve: as the number nought it was
# indistinguishable from a budget somebody had already spent, so every worker refused it.
grep -q '^expires ' "$work/plan"

# A zero *token* ceiling is a bearer that permits nothing, and is refused before one exists. Distinct
# from the above, and confusing the two is how "use a free model" became "your capsule is finished".
if plan --token-limit 0 --max-output-tokens 4096 --sensitivity 1 \
    --model Strong --spend-limit zero-cost >/dev/null 2>"$work/empty.err"; then
    echo "a bearer that permits nothing was planned" >&2
    exit 1
fi
grep -q 'permits nothing' "$work/empty.err"

# A session with no model grant is an ordinary session, and this is the check that says an Agent
# Capsule is a bounded place to compute rather than a container that only exists around a model.
# Refusing it was a real defect: every local, unplugged, model-free capsule was unlaunchable.
plan --token-limit 200000 --max-output-tokens 4096 --sensitivity 1 >"$work/nomodel"

grep -q '^gateway-unit none' "$work/nomodel" || {
    echo "a capsule with no model grant was given a gateway" >&2
    cat "$work/nomodel" >&2
    exit 1
}
if grep -q '^model-socket \|^model-token \|^teardown stop-gateway ' "$work/nomodel"; then
    echo "a capsule with no model grant names a surface nobody started" >&2
    exit 1
fi
# It keeps everything else it was granted. The absent gateway is one withheld grant, not a
# diminished session.
grep -q '^teardown stop-egress ' "$work/nomodel"
grep -q '^teardown stop-capsule ' "$work/nomodel"

# A class named without a ceiling beside it is half a selection, and is refused rather than completed
# with an invented bound.
if plan --token-limit 200000 --max-output-tokens 4096 --sensitivity 1 \
    --model Strong >/dev/null 2>"$work/half.err"; then
    echo "a model class was granted with no spending ceiling" >&2
    exit 1
fi
grep -q 'spend-limit' "$work/half.err"

# The broker is a unit of its own, torn down with the rest. A way out that lives inside the
# coordinator survives exactly as long as the coordinator does.
grep -q '^teardown stop-egress ' "$work/plan"

# A spending selection that is neither an integer nor the word is refused rather than guessed at.
if plan --token-limit 200000 --max-output-tokens 4096 --sensitivity 1 \
    --model Strong --spend-limit cheap >/dev/null 2>"$work/spend.err"; then
    echo "an unreadable spending selection was accepted" >&2
    exit 1
fi
grep -q 'zero-cost' "$work/spend.err"

echo "=== Agent session ownership gate passed ==="
