#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

# B7 host lifecycle gate: a real ephemeral token and 0600 UDS drive the shared gateway and worker.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then . "$HOME/.cargo/env"; fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

for command in curl python3; do
  command -v "$command" >/dev/null || { echo "gateway gate NOT RUN: $command absent" >&2; exit 3; }
done

work="$(mktemp -d)"
proxy_pid=""
gateway_pid=""
trap 'test -z "$gateway_pid" || kill "$gateway_pid" 2>/dev/null || true; test -z "$proxy_pid" || kill "$proxy_pid" 2>/dev/null || true; rm -rf "$work"' EXIT
chmod 700 "$work"
mkdir "$work/workspace"
mkdir "$work/runtime"
chmod 755 "$work/runtime" # systemd creates it before ExecStart; the process tightens it.

python3 - "$work/proxy-port" <<'PYTHON' &
import http.server
import json
import sys

class Proxy(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_): pass
    def do_POST(self):
        length = int(self.headers.get("content-length", "0"))
        body = json.loads(self.rfile.read(length) or b"{}")
        assert self.headers.get("authorization") in ("Bearer gate-master", "Bearer gate-virtual")
        if self.path == "/key/generate":
            assert body["models"] == ["cybou-strong"] and body["max_parallel_requests"] == 1
            response = {"key": "gate-virtual"}
        elif self.path == "/key/delete":
            response = {}
        elif self.path == "/v1/chat/completions":
            assert body["model"] == "cybou-strong" and body["stream"] is False
            response = {"model":"provider/revision","choices":[{"message":{"content":"bounded answer"}}],"usage":{"prompt_tokens":3,"completion_tokens":2,"total_tokens":5}}
        else:
            self.send_error(404); return
        encoded = json.dumps(response).encode()
        self.send_response(200)
        if self.path == "/v1/chat/completions":
            self.send_header("x-litellm-response-cost", "0.0000101")
            self.send_header("x-litellm-model-id", "deployment-sha")
            self.send_header("x-litellm-model-group", "cybou-strong")
            self.send_header("x-litellm-call-id", "gate-call")
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(encoded)))
        self.end_headers(); self.wfile.write(encoded)

server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Proxy)
open(sys.argv[1], "w").write(str(server.server_port))
server.serve_forever()
PYTHON
proxy_pid=$!
for _ in $(seq 1 50); do [ -s "$work/proxy-port" ] && break; sleep .05; done
port="$(cat "$work/proxy-port")"

CYBOU_AGENT_RUNTIME_DIR="$work/runtime" \
CYBOU_CAPSULE_ID=00000000-0000-0000-0000-000000000701 \
CYBOU_AGENT_TASK_ID=00000000-0000-0000-0000-000000000702 \
CYBOU_AGENT_WORKSPACE="$work/workspace" \
CYBOU_AGENT_LEASE_SECONDS=120 \
CYBOU_MODEL_CLASS=Strong \
CYBOU_MODEL_SPEND_LIMIT=100 \
CYBOU_MODEL_TOKEN_LIMIT=1000 \
CYBOU_MODEL_MAX_OUTPUT_TOKENS=32 \
CYBOU_MODEL_SENSITIVITY=1 \
CYBOU_MODEL_MICROUSD_PER_UNIT=10 \
CYBOU_LITELLM_BASE_URL="http://127.0.0.1:$port" \
CYBOU_LITELLM_MASTER_KEY=gate-master \
CYBOU_LITELLM_PROVIDER=gate-litellm \
CYBOU_LITELLM_MODEL_GROUP=cybou-strong \
CYBOU_LITELLM_DEPLOYMENT_SHA256=5555555555555555555555555555555555555555555555555555555555555555 \
CYBOU_LITELLM_TIMEOUT_MS=2000 \
cargo run --quiet --locked -p cybou-agent-gateway >"$work/gateway.out" 2>"$work/gateway.err" &
gateway_pid=$!

for _ in $(seq 1 400); do [ -S "$work/runtime/model.sock" ] && [ -s "$work/runtime/model-token" ] && break; sleep .05; done
if [ ! -S "$work/runtime/model.sock" ]; then
  cat "$work/gateway.err" >&2
  exit 1
fi
test "$(stat -c %a "$work/runtime")" = 700
test "$(stat -c %a "$work/runtime/model.sock")" = 600
test "$(stat -c %a "$work/runtime/model-token")" = 600
token="$(cat "$work/runtime/model-token")"
case "$token" in cybou_*) ;; *) echo "ephemeral token has wrong form" >&2; exit 1;; esac

answer="$(curl --silent --show-error --fail --unix-socket "$work/runtime/model.sock" \
  -H "authorization: Bearer $token" -H 'content-type: application/json' \
  --data '{"model":"Strong","messages":[{"role":"user","content":"hello"}],"max_tokens":9}' \
  http://localhost/v1/chat/completions)"
grep -q 'bounded answer' <<<"$answer"
for artifact in "$work/runtime/model-token" "$work/gateway.out" "$work/gateway.err"; do
  ! grep -q 'gate-master' "$artifact"
done
echo "=== Per-capsule agent gateway gate passed ==="
