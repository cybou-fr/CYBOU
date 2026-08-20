#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# What a Linux account does and does not get.
#
# Every other gate in this tree can run against fixtures. This one cannot: the whole point of the
# helper is that it consults the real shadow database through the real PAM stack, and a stub would
# prove only that the stub agrees with itself. So it creates throwaway accounts on the host it runs
# on, uses them, and removes them again.
#
# It must therefore run as root on a disposable machine — the local Debian builder, not a host
# anyone depends on. It refuses to run anywhere the accounts it would create already exist.

set -euo pipefail

if [ "$(id -u)" -ne 0 ]; then
    echo "ERROR: this gate creates and removes local accounts, so it needs root." >&2
    exit 2
fi

PERMITTED="cybou-gate-permitted"
REFUSED="cybou-gate-refused"
SECRET="not-a-real-password-$$"
WRONG="also-not-the-password"
SOCKET="/run/cybou-gate-auth.sock"

for account in "$PERMITTED" "$REFUSED"; do
    if id "$account" >/dev/null 2>&1; then
        echo "ERROR: $account already exists; refusing to touch an account this gate did not make." >&2
        exit 2
    fi
done

HELPER_PID=""
cleanup() {
    [ -n "$HELPER_PID" ] && kill "$HELPER_PID" 2>/dev/null || true
    userdel -r "$PERMITTED" 2>/dev/null || true
    userdel -r "$REFUSED" 2>/dev/null || true
    groupdel cybou-access 2>/dev/null || true
    rm -f "$SOCKET"
}
trap cleanup EXIT

BIN_DIR="${CARGO_TARGET_DIR:-target}/debug"
if [ ! -x "$BIN_DIR/cybou-authd" ]; then
    echo "ERROR: $BIN_DIR/cybou-authd is not built." >&2
    exit 2
fi

echo "==> Preparing the PAM stack and two throwaway accounts..."
install -m 0644 debian/pam-cybou /etc/pam.d/cybou
getent group cybou-access >/dev/null || groupadd --system cybou-access

useradd --no-create-home --shell /usr/sbin/nologin "$PERMITTED"
useradd --no-create-home --shell /usr/sbin/nologin "$REFUSED"
# chpasswd is how an administrator sets a password non-interactively; both accounts get the same
# one, so the only difference between them is the group.
printf '%s:%s\n' "$PERMITTED" "$SECRET" | chpasswd
printf '%s:%s\n' "$REFUSED" "$SECRET" | chpasswd
gpasswd -a "$PERMITTED" cybou-access >/dev/null

CYBOU_AUTH_SOCKET="$SOCKET" "$BIN_DIR/cybou-authd" >/tmp/cybou-authd-gate.log 2>&1 &
HELPER_PID="$!"

deadline=$((SECONDS + 15))
while [ "$SECONDS" -lt "$deadline" ] && [ ! -S "$SOCKET" ]; do
    sleep 1
done
if [ ! -S "$SOCKET" ]; then
    echo "ERROR: the helper never created its socket:" >&2
    cat /tmp/cybou-authd-gate.log >&2
    exit 1
fi
echo "    Helper is listening on $SOCKET"

# The socket must not be reachable by everyone on the host: a privileged helper anyone can talk to
# is a password oracle running as root.
mode="$(stat -c '%a' "$SOCKET")"
if [ "$mode" != "660" ]; then
    echo "ERROR: the helper socket is mode $mode; it must not be world-reachable." >&2
    exit 1
fi
echo "    Socket is mode $mode"

# Ask the helper directly, the way the gateway does. The request and answer are CBOR; both are
# small enough to build and read with printf and od rather than a program of their own.
ask() {
    local user="$1" secret="$2"
    python3 - "$user" "$secret" "$SOCKET" <<'PY'
import socket, sys

def text(value):
    raw = value.encode()
    if len(raw) < 24:
        return bytes([0x60 + len(raw)]) + raw
    return b"\x78" + bytes([len(raw)]) + raw

user, secret, path = sys.argv[1], sys.argv[2], sys.argv[3]
request = b"\xa2" + text("username") + text(user) + text("password") + text(secret)

connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
connection.connect(path)
connection.sendall(request)
connection.shutdown(socket.SHUT_WR)
answer = connection.recv(64)
# A one-entry map whose only value is true or false: 0xf5 is true, 0xf4 is false.
print("true" if answer.endswith(b"\xf5") else "false")
PY
}

echo "==> Verifying what an account does and does not get..."
if [ "$(ask "$PERMITTED" "$SECRET")" != "true" ]; then
    echo "ERROR: an account in the group, with the right password, was refused." >&2
    exit 1
fi
echo "    A permitted account with the right password is accepted"

if [ "$(ask "$PERMITTED" "$WRONG")" != "false" ]; then
    echo "ERROR: an account in the group was accepted with the wrong password." >&2
    exit 1
fi
echo "    The same account with the wrong password is refused"

# The one that matters: being a valid Linux account with a correct password is not enough.
if [ "$(ask "$REFUSED" "$SECRET")" != "false" ]; then
    echo "ERROR: an account outside $PERMITTED's group authenticated." >&2
    exit 1
fi
echo "    A valid account outside the group is refused despite a correct password"

if [ "$(ask "root" "$SECRET")" != "false" ]; then
    echo "ERROR: root authenticated through the helper." >&2
    exit 1
fi
echo "    root is refused"

if [ "$(ask "$PERMITTED" "")" != "false" ]; then
    echo "ERROR: an empty password was accepted." >&2
    exit 1
fi
echo "    An empty password is refused"

if [ "$(ask "no-such-account-here" "$SECRET")" != "false" ]; then
    echo "ERROR: an account that does not exist authenticated." >&2
    exit 1
fi
echo "    An account that does not exist is refused"

# Revocation has to be real: locking the account must close the door, which is what the `account`
# line in the PAM stack is for.
usermod -L "$PERMITTED"
if [ "$(ask "$PERMITTED" "$SECRET")" != "false" ]; then
    echo "ERROR: a locked account still authenticated." >&2
    exit 1
fi
echo "    A locked account is refused"
usermod -U "$PERMITTED"

# And removing the group membership closes it too, which is how access is taken back without
# touching the account itself.
gpasswd -d "$PERMITTED" cybou-access >/dev/null
if [ "$(ask "$PERMITTED" "$SECRET")" != "false" ]; then
    echo "ERROR: an account removed from the group still authenticated." >&2
    exit 1
fi
echo "    An account removed from the group is refused"

# Nothing about the secret may reach the log.
if grep -q "$SECRET" /tmp/cybou-authd-gate.log; then
    echo "ERROR: the helper wrote a password to its log." >&2
    exit 1
fi
echo "    No password reached the helper's output"

echo "==> PAM access gate PASSED"
