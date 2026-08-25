#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# A whole prompt turn, and the two things about it that are Cybou's rather than the protocol's.
#
# The protocol half — initialize, session/new, session/prompt, session/update — is the ACP SDK's job
# and this gate would be pointless if it only checked that. What it checks is what this client does
# with a turn that the reference clients do differently:
#
#   an agent's thought is not its answer      the two are kept apart, not concatenated
#   an agent asking permission is refused     and the refusal reaches the agent, not just a log
#
# The second is the one worth a gate. Every reference ACP client auto-approves, which is how a demo
# is written and not how a boundary is: it puts the decision in the hands of the thing being bounded.
# Cybou's answer is that inside its capsule an agent needs no permission, and outside it the answer is
# an ActionProposal a person decides — and this client can reach neither, so the only honest answer it
# has is no.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

command -v python3 >/dev/null || {
    echo "==> ACP session gate NOT RUN: no python3 to run a stand-in agent with" >&2
    exit 3
}

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
chmod 700 "$work"
mkdir "$work/workspace"

CYBOU_FAKE_AGENT_LOG="$work/agent.log" \
    cargo run --quiet --locked --example acp-turn -p cybou-acp -- \
    "$work/workspace" "say something" python3 fixtures/acp-agent-that-asks.py >"$work/turn.json"

read_field() {
    python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))[sys.argv[2]])' \
        "$work/turn.json" "$1"
}

test "$(read_field stopReason)" = "end_turn"
test -n "$(read_field sessionId)"

# The agent's words, whole, and nothing else folded in. A client that concatenated the thought would
# be presenting a draft as a conclusion.
message="$(read_field message)"
test "$message" = "the fake agent answered" || {
    echo "the turn's message was '$message'" >&2
    exit 1
}
case "$message" in
    *"deciding what to say"*)
        echo "the agent's internal reasoning was folded into its answer" >&2
        exit 1
        ;;
esac

# The thought is not discarded either. It is in the updates, keeping its own kind, which is what a
# live session surface needs and what a projection written too early would have thrown away.
python3 - "$work/turn.json" <<'PYTHON'
import json, sys

turn = json.load(open(sys.argv[1]))
kinds = [update["sessionUpdate"] for update in turn["updates"]]
assert kinds == [
    "agent_thought_chunk",
    "agent_message_chunk",
    "agent_message_chunk",
], kinds
assert turn["refusedPermissions"] == ["restart nginx.service"], turn["refusedPermissions"]
PYTHON

# And the refusal reached the agent. A client that recorded a refusal and answered yes on the wire
# would pass every check above and have granted the thing anyway.
grep -q '"cancelled"' "$work/agent.log" || {
    echo "the agent was not told it had been refused:" >&2
    cat "$work/agent.log" >&2
    exit 1
}
! grep -q '"selected"' "$work/agent.log"

echo "=== ACP session gate passed: one turn, and the agent was refused ==="
