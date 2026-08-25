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
    --model Strong --spend-limit 0 >"$work/plan"

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

# A zero *spending* ceiling is a real selection — cost nothing — and must plan. This is the free-model
# case the whole provider catalogue exists for, and an earlier version of this system refused it.
grep -q '^expires ' "$work/plan"

# A zero *token* ceiling is a bearer that permits nothing, and is refused before one exists. Distinct
# from the above, and confusing the two is how "use a free model" became "your capsule is finished".
if plan --token-limit 0 --max-output-tokens 4096 --sensitivity 1 \
    --model Strong --spend-limit 0 >/dev/null 2>"$work/empty.err"; then
    echo "a bearer that permits nothing was planned" >&2
    exit 1
fi
grep -q 'permits nothing' "$work/empty.err"

# A session with no model grant has no gateway to hold, and is refused whole rather than by whichever
# component first noticed. Half a session is the state that leaves runtime files nobody owns.
if plan --token-limit 200000 --max-output-tokens 4096 --sensitivity 1 \
    >/dev/null 2>"$work/nomodel.err"; then
    echo "a session with no model grant was planned" >&2
    exit 1
fi
grep -q 'grants no model' "$work/nomodel.err"

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

echo "=== Agent session ownership gate passed ==="
