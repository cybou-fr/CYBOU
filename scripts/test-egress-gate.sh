#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# ADR-0042 step ten: the one way out of a capsule decides by name, and cannot be talked past.
#
# Against a running broker, over the socket a capsule would use. Every check below is something an
# agent would try in order to reach somewhere nobody granted it — asking for an address instead of a
# name, asking for a different port, asking for a name that resolves back to the host it is running
# on. The unit tests say the decision function is right; this says the broker is the decision.
#
# A refusal a capsule could not tell from a broken network would be a bad refusal, so each check
# reads the status the broker sent rather than merely observing that nothing connected.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

if ! command -v python3 > /dev/null 2>&1; then
    echo "==> egress gate NOT RUN: no python3 to speak to a Unix socket with" >&2
    exit 3
fi

cargo build --quiet -p cybou-egressd
BROKER="$CARGO_TARGET_DIR/debug/cybou-egressd"

WORK="$(mktemp -d)"
SOCKET="$WORK/egress.sock"
LOG="$WORK/broker.log"
cleanup() {
    [ -n "${BROKER_PID:-}" ] && kill "$BROKER_PID" 2> /dev/null || true
    rm -rf "$WORK"
}
trap cleanup EXIT

# `localhost` is granted on purpose. It is the check that matters most: a name the grant permits,
# which resolves to the machine the capsule is running on. A broker that checked the name correctly
# and connected anyway would have done everything right and handed over the host.
"$BROKER" --socket "$SOCKET" --host example.com --host localhost > "$LOG" 2>&1 &
BROKER_PID=$!

for _ in $(seq 1 50); do
    [ -S "$SOCKET" ] && break
    sleep 0.1
done
if [ ! -S "$SOCKET" ]; then
    echo "==> egress gate NOT RUN: the broker did not come up" >&2
    cat "$LOG" >&2
    exit 3
fi

failures=0

# Send one request line and print the status the broker answered with.
say() {
    python3 - "$SOCKET" "$1" <<'PYTHON'
import socket
import sys

path, line = sys.argv[1], sys.argv[2]
connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
connection.settimeout(20)
connection.connect(path)
connection.sendall((line + "\r\n\r\n").encode())
try:
    answer = connection.recv(4096).decode("utf-8", "replace")
except Exception as why:  # noqa: BLE001 - a timeout is an answer the gate must be able to see
    print(f"NOTHING:{why}")
    sys.exit(0)
print(answer.splitlines()[0] if answer else "NOTHING:the broker said nothing")
PYTHON
}

must() {
    local name="$1" expected="$2" line="$3"
    local answer
    answer="$(say "$line")"
    if [[ "$answer" == *"$expected"* ]]; then
        printf '    ok      %s\n' "$name"
    else
        printf '    FAILED  %s\n        wanted %q\n        got    %q\n' "$name" "$expected" "$answer"
        failures=$((failures + 1))
    fi
}

echo "=== The one way out decides by name ==="

# Whether this host can reach the internet at all. Without it, the permitted case below cannot tell
# a broker that refused from a broker that could not resolve — and a gate that reported the second as
# the first would be saying the boundary held when nothing had been tested.
if python3 -c "import socket; socket.getaddrinfo('example.com', 443)" > /dev/null 2>&1; then
    must "a granted host on the granted port is connected" "200" \
        "CONNECT example.com:443 HTTP/1.1"
else
    echo "    note: no name resolution here, so the permitted case was NOT checked" >&2
    skipped=1
fi

must "a host nobody granted is refused" "403" \
    "CONNECT deny.example:443 HTTP/1.1"
must "a granted host on another port is refused" "403" \
    "CONNECT example.com:22 HTTP/1.1"

# The refusal that makes the name mean something. An address cannot be checked against a grant, so a
# broker that accepted one would reach anything a capsule could resolve for itself — through a name
# check nobody had to use.
must "an address where a name belongs is refused" "400" \
    "CONNECT 140.82.121.4:443 HTTP/1.1"
must "an address in brackets is still an address" "400" \
    "CONNECT [2606:50c0:8000::153]:443 HTTP/1.1"

# A granted name that resolves to the host. This is the one where every check above passes and the
# machine is handed over anyway.
must "a granted name that resolves to the host is refused" "403" \
    "CONNECT localhost:443 HTTP/1.1"

# Not a proxy. Anything but CONNECT would mean the broker handling the capsule's payload.
must "this is a tunnel and not a proxy" "400" \
    "GET http://example.com/ HTTP/1.1"

# And the broker is still there. A capsule that can end it by sending something malformed has found
# a way to deny every other capsule the egress it was granted.
must "a malformed request does not take the broker down with it" "403" \
    "CONNECT deny.example:443 HTTP/1.1"

echo
if [ "$failures" -gt 0 ]; then
    echo "=== EGRESS GATE FAILED: $failures check(s) ==="
    echo "--- what the broker said ---"
    cat "$LOG"
    exit 1
fi
if [ "${skipped:-0}" = 1 ]; then
    echo "=== every egress check that ran passed; the permitted case was NOT RUN ==="
else
    echo "=== egress gate passed ==="
fi
