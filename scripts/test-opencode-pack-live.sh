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
mapfile -t capsule_argv < <(cargo run --quiet --locked -p cybou-capsule --example capsule-argv -- \
  "$work/workspace" /usr/bin/env OPENCODE_CONFIG=/workspace/.cybou/opencode.json \
  "$PACK/opencode" run --format json \
  "Reply with exactly CYBOU_B7_LIVE and do not use a tool.")

answer="$("${capsule_argv[@]}")"
grep -q 'CYBOU_B7_LIVE' <<<"$answer"
echo "=== B7 live provider gate passed through OpenCode inside its capsule ==="
