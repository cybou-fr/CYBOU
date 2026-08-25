#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

# B7 credential-free gate. It proves the pinned artifact speaks ACP from inside the real capsule and
# that the model endpoint exists only through the per-capsule Unix channel. A provider-backed prompt
# is deliberately a separate opt-in live gate: absence of an operator credential is not a pass.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

cargo test -p cybou-agent-opencode -p cybou-model-bridge -p cybou-capsule --locked

for command in bwrap python3; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "==> OpenCode pack gate NOT RUN: $command is absent" >&2
    exit 3
  fi
done

cargo build --quiet --locked -p cybou-capsule-enter -p cybou-model-bridge
cargo build --quiet --locked -p cybou-acp --example probe-agent
ENTRY="$CARGO_TARGET_DIR/debug/cybou-capsule-enter"
BRIDGE="$CARGO_TARGET_DIR/debug/cybou-model-bridge"
PROBE="$CARGO_TARGET_DIR/debug/examples/probe-agent"
PACK=/usr/local/libexec/cybou/agents/opencode/1.18.23
if [ ! -x "$PACK/opencode" ]; then
  echo "==> OpenCode pack gate NOT RUN: install it with sudo bash scripts/install-opencode-pack.sh" >&2
  exit 3
fi

work="$(mktemp -d)"
gateway_pid=""
trap 'test -z "$gateway_pid" || kill "$gateway_pid" 2>/dev/null || true; rm -rf "$work"' EXIT
chmod 700 "$work"
mkdir -p "$work/workspace/.cybou"
token=lease-token-for-credential-free-gate
printf '%s' "$token" >"$work/model-token"
chmod 600 "$work/model-token"

# A minimal UDS HTTP peer is enough for the handshake: it is kept alive so an agent that probes its
# configured endpoint sees the same transport it will use in the live gate.
python3 - "$work/model.sock" <<'PYTHON' &
import os
import socket
import sys

path = sys.argv[1]
try:
    os.unlink(path)
except FileNotFoundError:
    pass
listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
listener.bind(path)
os.chmod(path, 0o600)
listener.listen()
while True:
    connection, _ = listener.accept()
    with connection:
        connection.recv(65536)
        body = b'{"data":[{"id":"Strong","object":"model"}],"object":"list"}'
        connection.sendall(b'HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: ' + str(len(body)).encode() + b'\r\n\r\n' + body)
PYTHON
gateway_pid=$!
for _ in $(seq 1 50); do [ -S "$work/model.sock" ] && break; sleep .05; done
test -S "$work/model.sock"
test "$(stat -c '%a' "$work/model.sock")" = 600
test "$(stat -c '%a' "$work/model-token")" = 600

# Generate the reviewed config through the pack crate instead of duplicating it in this gate.
cargo run --quiet --locked -p cybou-agent-opencode --example render-config -- Strong \
  >"$work/workspace/.cybou/opencode.json"

export CYBOU_CAPSULE_ENTRY="$ENTRY"
export CYBOU_MODEL_BRIDGE="$BRIDGE"
export CYBOU_MODEL_SOCKET="$work/model.sock"
export CYBOU_MODEL_TOKEN_FILE="$work/model-token"
export CYBOU_MODEL_CLASS=Strong
mapfile -t capsule_argv < <(cargo run --quiet --locked -p cybou-capsule --example capsule-argv -- \
  "$work/workspace" /usr/bin/env OPENCODE_CONFIG=/workspace/.cybou/opencode.json \
  "$PACK/opencode" acp --cwd /workspace)

handshake="$($PROBE "${capsule_argv[@]}")"
grep -q '"protocolVersion": 1' <<<"$handshake"
grep -qi 'opencode' <<<"$handshake"
echo "=== OpenCode ACP handshake passed inside a model-granted capsule ==="
