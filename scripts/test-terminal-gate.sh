#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Prove, on a real Linux host, that a person who signs in gets a terminal running as themselves.
#
# ADR-0047 describes a chain with four links and nothing that exercises it end to end: PAM says the
# password is right, the gateway turns that into a seat with a uid, `cybou-ptyd` holds a
# pseudoterminal for that uid and nobody else, and the browser reaches it through one WebSocket.
# Each link has unit tests. Together they had none, which is why "the terminal does not work" was a
# sentence nobody could answer without a browser and a live deployment.
#
# The gate creates its own account, with a password it generates and never writes down, and removes
# the account, its home and its socket afterwards. It is the same shape as the action gates: a
# temporary thing installed on a real host because the thing being proved is not simulable.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

if [ "$(uname -s)" != "Linux" ]; then
    echo "==> terminal gate NOT RUN: this needs a Linux host with PAM" >&2
    exit 3
fi
if [ "$(id -u)" -ne 0 ]; then
    echo "==> terminal gate NOT RUN: creating an account and a socket directory needs root" >&2
    exit 3
fi
for tool in useradd userdel usermod chpasswd runuser curl; do
    command -v "$tool" >/dev/null 2>&1 || {
        echo "==> terminal gate NOT RUN: $tool is required" >&2
        exit 3
    }
done

ACCOUNT="cybou-term-gate"
PORT="${CYBOU_TERMINAL_GATE_PORT:-8791}"
BASE="http://127.0.0.1:$PORT"
WORK="$(mktemp -d)"
SOCKET_DIR="$WORK/pty"
AUTH_SOCKET="$WORK/auth.sock"
authd_pid=
ptyd_pid=
gateway_pid=

cleanup() {
    status=$?
    if [ "$status" -ne 0 ]; then
        for log in authd ptyd gateway; do
            [ -f "$WORK/$log.log" ] && { echo "--- $log ---" >&2; cat "$WORK/$log.log" >&2; }
        done
    fi
    [ -n "$gateway_pid" ] && kill "$gateway_pid" >/dev/null 2>&1 || true
    [ -n "$ptyd_pid" ] && kill "$ptyd_pid" >/dev/null 2>&1 || true
    [ -n "$authd_pid" ] && kill "$authd_pid" >/dev/null 2>&1 || true
    wait >/dev/null 2>&1 || true
    userdel --remove "$ACCOUNT" >/dev/null 2>&1 || true
    rm -rf "$WORK"
    return "$status"
}
trap cleanup EXIT

echo "==> Building the three processes this needs..."
cargo build --quiet -p cybou-authd -p cybou-ptyd -p cybou-web-gateway
cargo build --quiet -p cybou-ptyd --example pty-roundtrip

# An account that exists for the length of this gate. The password is generated here and is never
# printed, stored or reused: PAM has to be given something true, and nothing else has to know it.
echo "==> Making an account for the gate to be..."
userdel --remove "$ACCOUNT" >/dev/null 2>&1 || true
useradd --create-home --shell /bin/sh "$ACCOUNT"
SECRET="$(head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n')"
printf '%s:%s\n' "$ACCOUNT" "$SECRET" | chpasswd
ACCOUNT_UID="$(id -u "$ACCOUNT")"

# Two rules the deployment already has, found by this gate failing on them. Signing in is limited
# to one group — the helper says so at startup — and the owner has to be able to reach its own
# socket directory, which a private mkdtemp does not allow.
getent group cybou >/dev/null 2>&1 || groupadd --system cybou
getent group cybou-access >/dev/null 2>&1 || groupadd --system cybou-access
usermod --append --groups cybou-access "$ACCOUNT"
chmod 0711 "$WORK"

mkdir -p "$SOCKET_DIR/$ACCOUNT_UID"
chown "$ACCOUNT:cybou" "$SOCKET_DIR/$ACCOUNT_UID"
chmod 0750 "$SOCKET_DIR/$ACCOUNT_UID"

echo "==> Starting the credential helper..."
CYBOU_AUTH_SOCKET="$AUTH_SOCKET" target/debug/cybou-authd >"$WORK/authd.log" 2>&1 &
authd_pid=$!
for _ in $(seq 1 50); do [ -S "$AUTH_SOCKET" ] && break; sleep 0.1; done
[ -S "$AUTH_SOCKET" ] || { echo "ERROR: the credential helper opened no socket" >&2; exit 1; }

echo "==> Starting the terminal owner as $ACCOUNT..."
CYBOU_PTY_SOCKET="$SOCKET_DIR/$ACCOUNT_UID/owner.sock" \
CYBOU_PTY_SHELL="/bin/sh" \
    runuser -u "$ACCOUNT" --preserve-environment -- \
    "$REPO_ROOT/target/debug/cybou-ptyd" >"$WORK/ptyd.log" 2>&1 &
ptyd_pid=$!
for _ in $(seq 1 50); do [ -S "$SOCKET_DIR/$ACCOUNT_UID/owner.sock" ] && break; sleep 0.1; done
[ -S "$SOCKET_DIR/$ACCOUNT_UID/owner.sock" ] || {
    echo "ERROR: the terminal owner opened no socket" >&2
    exit 1
}

echo "=== The directory is the boundary, and it says so ==="
# The socket itself is world-writable on purpose: what stops a stranger is that they cannot reach
# the path. That makes these two modes the whole of the access control, and a deployment that
# loosened one while trusting the other would be open without anything looking different.
parent_mode="$(stat -c '%a' "$SOCKET_DIR")"
own_mode="$(stat -c '%a %U:%G' "$SOCKET_DIR/$ACCOUNT_UID")"
if [ "$parent_mode" != "755" ] && [ "$parent_mode" != "711" ]; then
    echo "ERROR: $SOCKET_DIR is $parent_mode; the account cannot reach its own directory" >&2
    exit 1
fi
if [ "$own_mode" != "750 $ACCOUNT:cybou" ]; then
    echo "ERROR: the account's socket directory is '$own_mode', not '750 $ACCOUNT:cybou'" >&2
    exit 1
fi
echo "    ok      $parent_mode above, '$own_mode' below"

# ------------------------------------------------------------------ the owner, on its own
echo "=== A pseudoterminal runs the account's own shell ==="
CYBOU_PTY_GATE_SOCKET="$SOCKET_DIR/$ACCOUNT_UID/owner.sock" \
    runuser -u "$ACCOUNT" --preserve-environment -- \
    "$REPO_ROOT/target/debug/examples/pty-roundtrip"
echo "    ok      the owner opened a terminal, ran a command and gave back its output"

# ------------------------------------------------------------------ the whole chain
echo "==> Starting the gateway..."
CYBOU_AUTH_SOCKET="$AUTH_SOCKET" \
CYBOU_PTY_SOCKET_DIR="$SOCKET_DIR" \
CYBOU_GATEWAY_ADDR="127.0.0.1:$PORT" \
    target/debug/cybou-web-gateway >"$WORK/gateway.log" 2>&1 &
gateway_pid=$!
for _ in $(seq 1 50); do curl -fs -o /dev/null "$BASE/api/v1/session" && break; sleep 0.2; done

echo "=== A person signs in with an account on this machine ==="
cookie_jar="$WORK/cookies"
login_status="$(curl -s -o "$WORK/login.json" -w '%{http_code}' -c "$cookie_jar" \
    -H 'content-type: application/json' \
    -d "{\"username\":\"$ACCOUNT\",\"password\":\"$SECRET\"}" \
    "$BASE/api/v1/login")"
if [ "$login_status" != "200" ]; then
    echo "ERROR: signing in as $ACCOUNT answered HTTP $login_status" >&2
    cat "$WORK/login.json" >&2
    exit 1
fi
echo "    ok      PAM accepted the account and the gateway issued a session"

echo "=== The gateway hands that session its own terminal ==="
# A WebSocket upgrade, by hand: what matters is that the gateway found this uid's owner socket and
# switched protocols rather than refusing. A body is not read here — the frames are CBOR and the
# owner has already been driven directly above.
upgrade="$(curl -s -i -N --max-time 10 -b "$cookie_jar" \
    -H 'Connection: Upgrade' -H 'Upgrade: websocket' \
    -H 'Sec-WebSocket-Version: 13' -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' \
    "$BASE/api/v1/terminal" | head -1 || true)"
case "$upgrade" in
    *101*) echo "    ok      the socket was accepted for the signed-in account" ;;
    *)
        echo "ERROR: the terminal socket answered '$upgrade' for a signed-in account" >&2
        exit 1
        ;;
esac

echo "=== A stranger gets no terminal ==="
refused="$(curl -s -i -N --max-time 10 \
    -H 'Connection: Upgrade' -H 'Upgrade: websocket' \
    -H 'Sec-WebSocket-Version: 13' -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' \
    "$BASE/api/v1/terminal" | head -1 || true)"
case "$refused" in
    *101*)
        echo "ERROR: a request with no session was given a terminal" >&2
        exit 1
        ;;
    *) echo "    ok      refused: '$refused'" ;;
esac

echo "=== terminal gate passed ==="
