#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

# B7 live gate. The operator supplies an already-running per-capsule model-gateway UDS and the
# matching ephemeral lease-token file. Provider credentials remain in the worker and are never read
# here. Exit 3 means the deployment precondition is absent, never that a live provider passed.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

required=(CYBOU_MODEL_SOCKET CYBOU_MODEL_TOKEN_FILE CYBOU_MODEL_CLASS)
for name in "${required[@]}"; do
  if [ -z "${!name:-}" ]; then
    echo "==> B7 live provider gate NOT RUN: $name is unset" >&2
    exit 3
  fi
done
if [ ! -S "$CYBOU_MODEL_SOCKET" ] || [ ! -r "$CYBOU_MODEL_TOKEN_FILE" ]; then
  echo "==> B7 live provider gate NOT RUN: gateway socket or token file is unavailable" >&2
  exit 3
fi
if [ "$(stat -c '%a' "$CYBOU_MODEL_SOCKET")" != 600 ] || \
   [ "$(stat -c '%a' "$CYBOU_MODEL_TOKEN_FILE")" != 600 ]; then
  echo "ERROR: gateway socket and ephemeral token file must both have mode 0600" >&2
  exit 1
fi

PACK=/usr/local/libexec/cybou/agents/opencode/1.18.23
test -x "$PACK/opencode"
cargo build --quiet --locked -p cybou-capsule-enter -p cybou-model-bridge

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
chmod 700 "$work"
mkdir -p "$work/workspace/.cybou"
cargo run --quiet --locked -p cybou-agent-opencode --example render-config -- "$CYBOU_MODEL_CLASS" \
  >"$work/workspace/.cybou/opencode.json"

export CYBOU_CAPSULE_ENTRY="$CARGO_TARGET_DIR/debug/cybou-capsule-enter"
export CYBOU_MODEL_BRIDGE="$CARGO_TARGET_DIR/debug/cybou-model-bridge"
# The agent's own ACP entrypoint, driven by Cybou's ACP client. Deliberately not `opencode run`,
# which would prove only that OpenCode could reach the gateway. What has to be true for B7 is that
# Cybou drives the agent: initialize, session/new, prompt, and an answer that came back over the
# protocol from a real provider.
mapfile -t capsule_argv < <(cargo run --quiet --locked -p cybou-capsule --example capsule-argv -- \
  "$work/workspace" /usr/bin/env OPENCODE_CONFIG=/workspace/.cybou/opencode.json \
  "$PACK/opencode" acp --cwd /workspace)

turn="$(CYBOU_ACP_TURN_SECONDS=180 cargo run --quiet --locked -p cybou-acp --example acp-turn -- \
  "$work/workspace" "Reply with exactly CYBOU_B7_LIVE and do not use a tool." "${capsule_argv[@]}")"

python3 -c 'import json,sys; turn=json.load(sys.stdin); assert turn["stopReason"] == "end_turn", turn["stopReason"]; assert turn["sessionId"], "no session was opened"; print(turn["message"])' <<<"$turn" >"$work/message"
grep -q 'CYBOU_B7_LIVE' "$work/message"
echo "=== B7 live provider gate passed: Cybou drove OpenCode over ACP inside its capsule ==="
