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

cargo build --quiet -p cybou-egressd -p cybou-egress-bridge -p cybou-capsule-enter
BROKER="$CARGO_TARGET_DIR/debug/cybou-egressd"
BRIDGE="$CARGO_TARGET_DIR/debug/cybou-egress-bridge"
ENTRY="$CARGO_TARGET_DIR/debug/cybou-capsule-enter"

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
HOST_IPV4="$(ip -4 -o address show scope global 2>/dev/null | awk 'NR == 1 { sub(/\/.*/, "", $4); print $4 }')"
HOST_NAME=""
if [ -n "$HOST_IPV4" ]; then
    HOST_NAME="${HOST_IPV4//./-}.sslip.io"
fi

broker_arguments=(--socket "$SOCKET" --host example.com --host github.com --host localhost)
if [ -n "$HOST_NAME" ]; then
    broker_arguments+=(--host "$HOST_NAME")
fi
"$BROKER" "${broker_arguments[@]}" > "$LOG" 2>&1 &
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
if [ "$(stat -c '%a' "$WORK")" = 700 ] && [ "$(stat -c '%a' "$SOCKET")" = 600 ]; then
    printf '    ok      %s\n' "runtime directory and broker socket are private"
else
    printf '    FAILED  %s\n' "runtime directory and broker socket are private"
    failures=1
fi

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

if [ -n "$HOST_NAME" ] && getent ahostsv4 "$HOST_NAME" 2>/dev/null | grep -q "$HOST_IPV4"; then
    must "a granted name resolving to an exact host interface address is refused" "403" \
        "CONNECT $HOST_NAME:443 HTTP/1.1"
else
    echo "    note: no resolvable non-loopback host address, so the exact-interface case was NOT checked" >&2
    skipped=1
fi

# Not a proxy. Anything but CONNECT would mean the broker handling the capsule's payload.
must "this is a tunnel and not a proxy" "400" \
    "GET http://example.com/ HTTP/1.1"

# And the broker is still there. A capsule that can end it by sending something malformed has found
# a way to deny every other capsule the egress it was granted.
must "a malformed request does not take the broker down with it" "403" \
    "CONNECT deny.example:443 HTTP/1.1"

echo
echo "=== Ordinary clients use the broker from inside a capsule ==="

if command -v bwrap > /dev/null 2>&1 && command -v curl > /dev/null 2>&1; then
    CAPSULE_WORKSPACE="$WORK/workspace"
    mkdir "$CAPSULE_WORKSPACE"
    export CYBOU_CAPSULE_ENTRY="$ENTRY"
    export CYBOU_EGRESS_BRIDGE="$BRIDGE"
    export CYBOU_EGRESS_SOCKET="$SOCKET"
    export CYBOU_EGRESS_HOSTS="example.com,github.com"

    inside_network() {
        local script="$1"
        mapfile -t capsule_argv < <(cargo run --quiet -p cybou-capsule --example capsule-argv -- \
            "$CAPSULE_WORKSPACE" /bin/sh -c "$script")
        "${capsule_argv[@]}" 2>&1 || true
    }

    allowed="$(inside_network \
        "curl --fail --silent --show-error --max-time 20 https://example.com/ -o /workspace/allowed.html && test -s /workspace/allowed.html && echo ALLOWED")"
    if [[ "$allowed" == *"ALLOWED"* ]]; then
        printf '    ok      %s\n' "curl reaches a granted host through the bridge"
    else
        printf '    FAILED  %s\n        got %q\n' \
            "curl reaches a granted host through the bridge" "$allowed"
        failures=$((failures + 1))
    fi

    if command -v git > /dev/null 2>&1; then
        git_allowed="$(inside_network \
            "git ls-remote https://github.com/octocat/Hello-World.git HEAD >/workspace/remote.txt && grep -q HEAD /workspace/remote.txt && echo GIT-ALLOWED")"
        if [[ "$git_allowed" == *"GIT-ALLOWED"* ]]; then
            printf '    ok      %s\n' "git uses the same granted bridge"
        else
            printf '    FAILED  %s\n        got %q\n' \
                "git uses the same granted bridge" "$git_allowed"
            failures=$((failures + 1))
        fi
    else
        echo "    note: git is absent, so its ordinary-client case was NOT checked" >&2
        skipped=1
    fi

    denied="$(inside_network \
        "code=\$(curl --silent --insecure --max-time 10 -o /dev/null -w '%{http_connect}' https://deny.example/ 2>/dev/null || true); [ \"\$code\" = 403 ] && echo REFUSED || echo \"WRONG:\$code\"")"
    if [[ "$denied" == *"REFUSED"* ]]; then
        printf '    ok      %s\n' "curl is refused a host outside the grant"
    else
        printf '    FAILED  %s\n        got %q\n' \
            "curl is refused a host outside the grant" "$denied"
        failures=$((failures + 1))
    fi

    direct_ip="$(python3 - <<'PYTHON'
import socket
print(socket.getaddrinfo("example.com", 443, type=socket.SOCK_STREAM)[0][4][0])
PYTHON
)"
    direct="$(inside_network \
        "if env -u HTTPS_PROXY -u HTTP_PROXY -u NO_PROXY curl --insecure --silent --connect-timeout 2 --max-time 3 --noproxy '*' https://$direct_ip/ >/dev/null 2>&1; then echo ESCAPED; else echo NO-ROUTE; fi")"
    if [[ "$direct" == *"NO-ROUTE"* ]]; then
        printf '    ok      %s\n' "direct address access still has no route"
    else
        printf '    FAILED  %s\n        got %q\n' \
            "direct address access still has no route" "$direct"
        failures=$((failures + 1))
    fi

    OTHER_SOCKET="$WORK/other-capsule.sock"
    python3 - "$OTHER_SOCKET" <<'PYTHON' &
import socket
import sys
import time

listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
listener.bind(sys.argv[1])
listener.listen()
time.sleep(30)
PYTHON
    OTHER_SOCKET_PID=$!
    for _ in $(seq 1 20); do [ -S "$OTHER_SOCKET" ] && break; sleep .05; done
    cross="$(inside_network \
        "test -e /run/cybou/other-capsule.sock && echo CROSSED || echo ISOLATED")"
    kill "$OTHER_SOCKET_PID" 2>/dev/null || true
    if [[ "$cross" == *"ISOLATED"* ]]; then
        printf '    ok      %s\n' "another capsule's pathname socket is absent"
    else
        printf '    FAILED  %s\n        got %q\n' \
            "another capsule's pathname socket is absent" "$cross"
        failures=$((failures + 1))
    fi

    bridge_death="$(inside_network \
        "bridge=\$(pgrep -f '^/.cybou-egress-bridge ' | head -n1); kill \"\$bridge\"; sleep .1; code=\$(curl --silent --insecure --connect-timeout 2 --max-time 3 -o /dev/null -w '%{http_connect}' https://example.com/ 2>/dev/null || true); direct=blocked; env -u HTTPS_PROXY -u HTTP_PROXY -u NO_PROXY curl --insecure --silent --connect-timeout 1 --max-time 2 --noproxy '*' https://$direct_ip/ >/dev/null 2>&1 && direct=ESCAPED; [ \"\$code\" = 000 ] && [ \"\$direct\" = blocked ] && echo CONTAINED || echo \"WRONG:\$code:\$direct\"")"
    if [[ "$bridge_death" == *"CONTAINED"* ]]; then
        printf '    ok      %s\n' "killing the bridge removes egress and opens no direct route"
    else
        printf '    FAILED  %s\n        got %q\n' \
            "killing the bridge removes egress and opens no direct route" "$bridge_death"
        failures=$((failures + 1))
    fi
else
    echo "    note: bubblewrap or curl is absent, so the capsule-to-Internet cases were NOT checked" >&2
    skipped=1
fi

echo
echo "=== Broker resource amplification is bounded ==="

resource_answer="$(python3 - "$SOCKET" <<'PYTHON'
import socket
import sys
import time

path = sys.argv[1]
held = []
for _ in range(64):
    connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    connection.connect(path)
    connection.sendall(b"C")
    held.append(connection)
time.sleep(0.2)
excess = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
excess.settimeout(5)
excess.connect(path)
excess.sendall(b"CONNECT example.com:443 HTTP/1.1\r\n\r\n")
answer = excess.recv(4096).decode("utf-8", "replace")
print(answer.splitlines()[0] if answer else "NOTHING")
for connection in held:
    connection.close()
PYTHON
)"
if [[ "$resource_answer" == *"503"* ]]; then
    printf '    ok      %s\n' "connections above the per-capsule ceiling are refused"
else
    printf '    FAILED  %s\n        got %q\n' \
        "connections above the per-capsule ceiling are refused" "$resource_answer"
    failures=$((failures + 1))
fi

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
