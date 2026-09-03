#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Somebody's own files, edited from a browser, without the browser's host ever holding them.
#
# The other account gates prove a terminal and a mailbox. This one is the surface where being wrong
# costs the most: a write lands in a person's home directory, as that person, and a save that
# overwrites what somebody else changed cannot be taken back.
#
# So the things checked here are the ones an editor depends on: what is read is what is on the disk,
# what is written is owned by the account and not by whatever ran the gateway, a save conditional on
# a digest that no longer matches writes nothing and says so as a conflict rather than as a host that
# is unavailable, and a path leaving the home is refused before the owner is asked.
#
# Exit 3 means this host cannot make accounts or run PAM, which is a check that did not run.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

not_run() { echo "==> host files gate NOT RUN: $1" >&2; exit 3; }
[ "$(uname -s)" = "Linux" ] || not_run "this needs a Linux host with PAM"
[ "$(id -u)" -eq 0 ] || not_run "creating an account needs root"
for tool in useradd userdel usermod groupadd chpasswd runuser curl python3 sha256sum; do
    command -v "$tool" >/dev/null 2>&1 || not_run "$tool is not installed"
done

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
ACCOUNT="cybou-files-gate"
PORT="${CYBOU_HOST_FILES_GATE_PORT:-8795}"
BASE="http://127.0.0.1:$PORT"
WORK="$(mktemp -d)"
SOCKET_DIR="$WORK/host-files"
AUTH_SOCKET="$WORK/auth.sock"
JAR="$WORK/cookies"
authd_pid=
owner_pid=
gateway_pid=

cleanup() {
    status=$?
    if [ "$status" -ne 0 ]; then
        for log in authd owner gateway; do
            [ -f "$WORK/$log.log" ] && { echo "--- $log ---" >&2; tail -20 "$WORK/$log.log" >&2; }
        done
    fi
    for pid in "$gateway_pid" "$owner_pid" "$authd_pid"; do
        [ -n "$pid" ] && kill "$pid" >/dev/null 2>&1 || true
    done
    wait >/dev/null 2>&1 || true
    userdel --remove "$ACCOUNT" >/dev/null 2>&1 || true
    rm -rf "$WORK"
    return "$status"
}
trap cleanup EXIT

echo "==> Building what this needs..."
cargo build --quiet --locked -p cybou-authd -p cybou-host-filesd -p cybou-web-gateway
install -m 0755 "$CARGO_TARGET_DIR/debug/cybou-host-filesd" "$WORK/cybou-host-filesd"

getent group cybou >/dev/null 2>&1 || groupadd --system cybou
getent group cybou-access >/dev/null 2>&1 || groupadd --system cybou-access
chmod 0711 "$WORK"

userdel --remove "$ACCOUNT" >/dev/null 2>&1 || true
useradd --create-home --shell /bin/sh "$ACCOUNT"
usermod --append --groups cybou-access "$ACCOUNT"
SECRET="$(head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n')"
printf '%s:%s\n' "$ACCOUNT" "$SECRET" | chpasswd
ACCOUNT_UID="$(id -u "$ACCOUNT")"
HOME_DIR="$(getent passwd "$ACCOUNT" | cut -d: -f6)"

# Something already in the home, written as the account, so the first read is of a real file this
# gate did not put there through the surface it is testing.
runuser -u "$ACCOUNT" -- sh -c "printf 'first line\n' > '$HOME_DIR/notes.txt'"

mkdir -p "$SOCKET_DIR/$ACCOUNT_UID"
chown "$ACCOUNT:cybou" "$SOCKET_DIR/$ACCOUNT_UID"
chmod 0750 "$SOCKET_DIR/$ACCOUNT_UID"

echo "==> Starting the credential helper..."
CYBOU_AUTH_SOCKET="$AUTH_SOCKET" "$CARGO_TARGET_DIR/debug/cybou-authd" >"$WORK/authd.log" 2>&1 &
authd_pid=$!
for _ in $(seq 1 50); do [ -S "$AUTH_SOCKET" ] && break; sleep 0.1; done
[ -S "$AUTH_SOCKET" ] || { echo "the credential helper opened no socket" >&2; exit 1; }

echo "==> Starting the account's own filesystem owner..."
CYBOU_HOST_FILES_HOME="$HOME_DIR" \
CYBOU_HOST_FILES_SOCKET="$SOCKET_DIR/$ACCOUNT_UID/owner.sock" \
    runuser -u "$ACCOUNT" --preserve-environment -- \
    "$WORK/cybou-host-filesd" >"$WORK/owner.log" 2>&1 &
owner_pid=$!
for _ in $(seq 1 100); do [ -S "$SOCKET_DIR/$ACCOUNT_UID/owner.sock" ] && break; sleep 0.1; done
[ -S "$SOCKET_DIR/$ACCOUNT_UID/owner.sock" ] || { echo "the owner opened no socket" >&2; exit 1; }

echo "==> Starting the gateway in front of it..."
CYBOU_AUTH_SOCKET="$AUTH_SOCKET" \
CYBOU_HOST_FILES_SOCKET_DIR="$SOCKET_DIR" \
CYBOU_GATEWAY_ADDR="127.0.0.1:$PORT" \
    "$CARGO_TARGET_DIR/debug/cybou-web-gateway" >"$WORK/gateway.log" 2>&1 &
gateway_pid=$!
for _ in $(seq 1 100); do curl -fs -o /dev/null "$BASE/api/v1/session" && break; sleep 0.2; done
curl -fs -o /dev/null "$BASE/api/v1/session" || { echo "the gateway never answered" >&2; exit 1; }

post() {
    curl -s -b "$JAR" -o "$WORK/answer.json" -w '%{http_code}' \
        -H 'content-type: application/json' -d "$2" "$BASE/api/v1/host-files/$1"
}

echo "=== A stranger reaches nobody's files ==="
status="$(curl -s -o "$WORK/stranger.json" -w '%{http_code}' -H 'content-type: application/json' \
    -d "{\"path\":\"$HOME_DIR/notes.txt\"}" "$BASE/api/v1/host-files/read")"
[ "$status" = "401" ] || [ "$status" = "403" ] || {
    echo "a request with no session answered HTTP $status" >&2
    exit 1
}
echo "    ok      refused with $status, before the owner was asked anything"

echo "=== The account signs in and reads its own file ==="
login="$(curl -s -o "$WORK/login.json" -w '%{http_code}' -c "$JAR" \
    -H 'content-type: application/json' \
    -d "{\"username\":\"$ACCOUNT\",\"password\":\"$SECRET\"}" "$BASE/api/v1/login")"
[ "$login" = "200" ] || { echo "signing in answered HTTP $login" >&2; cat "$WORK/login.json" >&2; exit 1; }

[ "$(post read "{\"path\":\"$HOME_DIR/notes.txt\"}")" = "200" ] || {
    echo "reading the file answered a refusal" >&2
    cat "$WORK/answer.json" >&2
    exit 1
}
DIGEST="$(python3 - "$WORK/answer.json" <<'PYTHON'
import json, sys
projection = json.load(open(sys.argv[1]))
assert projection["text"] == "first line\n", projection
print(projection["contentSha256"])
PYTHON
)"
ON_DISK="$(sha256sum "$HOME_DIR/notes.txt" | cut -d' ' -f1)"
[ "$DIGEST" = "$ON_DISK" ] || {
    echo "the digest the panel was given is not the digest of the file on disk" >&2
    exit 1
}
echo "    ok      what was read is what is on the disk, digest and all"

echo "=== A save conditional on that digest lands as the account ==="
[ "$(post write "{\"path\":\"$HOME_DIR/notes.txt\",\"expectedSha256\":\"$DIGEST\",\"text\":\"second line\n\"}")" = "200" ] || {
    echo "the conditional save was refused" >&2
    cat "$WORK/answer.json" >&2
    exit 1
}
[ "$(cat "$HOME_DIR/notes.txt")" = "second line" ] || {
    echo "the file on disk does not hold what was saved" >&2
    exit 1
}
owner_of_file="$(stat -c '%U' "$HOME_DIR/notes.txt")"
[ "$owner_of_file" = "$ACCOUNT" ] || {
    echo "the file is owned by $owner_of_file after the save, not by the account" >&2
    exit 1
}
echo "    ok      the bytes changed, and the file still belongs to the person"

echo "=== A save conditional on a digest that no longer matches writes nothing ==="
# What an editor does when somebody else — or the same person in another tab — got there first.
status="$(post write "{\"path\":\"$HOME_DIR/notes.txt\",\"expectedSha256\":\"$DIGEST\",\"text\":\"third line\n\"}")"
[ "$status" = "409" ] || {
    echo "a stale save answered HTTP $status where a conflict was the truth" >&2
    cat "$WORK/answer.json" >&2
    exit 1
}
python3 - "$WORK/answer.json" <<'PYTHON'
import json, sys
body = json.load(open(sys.argv[1]))
assert body["error"] == "hostUserFileChanged", body
assert body["retryable"] is False, "a person was told to retry a save that will conflict again"
assert "changed" in body.get("detail", ""), body
PYTHON
[ "$(cat "$HOME_DIR/notes.txt")" = "second line" ] || {
    echo "a refused save changed the file anyway" >&2
    exit 1
}
echo "    ok      409, nothing written, and the file is exactly as it was"

echo "=== And a path leaving the home is refused ==="
for escape in "/etc/passwd" "$HOME_DIR/../../etc/passwd"; do
    status="$(post read "{\"path\":\"$escape\"}")"
    [ "$status" != "200" ] || {
        echo "reading $escape through the panel answered 200" >&2
        exit 1
    }
done
echo "    ok      outside the home is not addressable from this surface"

echo "=== host files gate passed: read, write, conflict and confinement, as the account ==="
