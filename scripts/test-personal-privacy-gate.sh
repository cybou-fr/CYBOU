#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Two people, two mailboxes, one host — proven where it matters rather than where it is easy.
#
# The owner gate beside this one proves that two `cybou-personald` processes hold separate records.
# A gateway test proves that two sessions with different uids are answered separately from the
# in-process store. Neither is the deployed arrangement, which is the two of them joined: a person
# signs in through PAM, the gateway turns that into a session carrying their numeric identity, and
# that number chooses which owner's socket answers. Nothing had ever exercised that join, and it is
# the one place where getting a number wrong means one person reading another's mail.
#
# So this gate makes two real accounts, gives each its own owner, and asks the gateway — over HTTP,
# the way a browser does — for each of them in turn.
#
# Exit 3 means this host cannot make accounts or run PAM, which is a check that did not run.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

not_run() { echo "==> personal privacy gate NOT RUN: $1" >&2; exit 3; }
[ "$(uname -s)" = "Linux" ] || not_run "this needs a Linux host with PAM"
[ "$(id -u)" -eq 0 ] || not_run "creating accounts needs root"
for tool in useradd userdel usermod groupadd chpasswd runuser curl python3; do
    command -v "$tool" >/dev/null 2>&1 || not_run "$tool is not installed"
done

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target}"
FIRST="cybou-priv-one"
SECOND="cybou-priv-two"
PORT="${CYBOU_PERSONAL_GATE_PORT:-8793}"
BASE="http://127.0.0.1:$PORT"
WORK="$(mktemp -d)"
SOCKET_DIR="$WORK/personal"
AUTH_SOCKET="$WORK/auth.sock"
authd_pid=
gateway_pid=
first_pid=
second_pid=

cleanup() {
    status=$?
    if [ "$status" -ne 0 ]; then
        for log in authd gateway first second; do
            [ -f "$WORK/$log.log" ] && { echo "--- $log ---" >&2; tail -20 "$WORK/$log.log" >&2; }
        done
    fi
    for pid in "$gateway_pid" "$first_pid" "$second_pid" "$authd_pid"; do
        [ -n "$pid" ] && kill "$pid" >/dev/null 2>&1 || true
    done
    wait >/dev/null 2>&1 || true
    userdel --remove "$FIRST" >/dev/null 2>&1 || true
    userdel --remove "$SECOND" >/dev/null 2>&1 || true
    rm -rf "$WORK"
    return "$status"
}
trap cleanup EXIT

echo "==> Building what this needs..."
cargo build --quiet --locked -p cybou-authd -p cybou-personald -p cybou-web-gateway
# Copied where the accounts can reach it. A build directory belongs to whoever built it, and the
# owners here deliberately run as somebody else.
install -m 0755 "$CARGO_TARGET_DIR/debug/cybou-personald" "$WORK/cybou-personald"

getent group cybou >/dev/null 2>&1 || groupadd --system cybou
getent group cybou-access >/dev/null 2>&1 || groupadd --system cybou-access
chmod 0711 "$WORK"

make_account() {
    local account="$1"
    userdel --remove "$account" >/dev/null 2>&1 || true
    useradd --create-home --shell /bin/sh "$account"
    usermod --append --groups cybou-access "$account"
    # Generated here, never printed and never reused. PAM has to be given something true.
    head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n'
}

FIRST_SECRET="$(make_account "$FIRST")"
printf '%s:%s\n' "$FIRST" "$FIRST_SECRET" | chpasswd
SECOND_SECRET="$(make_account "$SECOND")"
printf '%s:%s\n' "$SECOND" "$SECOND_SECRET" | chpasswd
FIRST_UID="$(id -u "$FIRST")"
SECOND_UID="$(id -u "$SECOND")"

start_owner() {
    local account="$1" uid="$2" name="$3"
    mkdir -p "$SOCKET_DIR/$uid" "$WORK/stores/$uid"
    chown "$account:cybou" "$SOCKET_DIR/$uid"
    chmod 0750 "$SOCKET_DIR/$uid"
    chown "$account" "$WORK/stores/$uid"
    CYBOU_PERSONAL_SOCKET="$SOCKET_DIR/$uid/personal.sock" \
    CYBOU_PERSONAL_STORE="$WORK/stores/$uid/personal.sqlite3" \
        runuser -u "$account" --preserve-environment -- \
        "$WORK/cybou-personald" >"$WORK/$name.log" 2>&1 &
    for _ in $(seq 1 100); do
        [ -S "$SOCKET_DIR/$uid/personal.sock" ] && return 0
        sleep 0.1
    done
    echo "the owner for $account opened no socket" >&2
    return 1
}

echo "==> Starting the credential helper..."
CYBOU_AUTH_SOCKET="$AUTH_SOCKET" "$CARGO_TARGET_DIR/debug/cybou-authd" >"$WORK/authd.log" 2>&1 &
authd_pid=$!
for _ in $(seq 1 50); do [ -S "$AUTH_SOCKET" ] && break; sleep 0.1; done
[ -S "$AUTH_SOCKET" ] || { echo "the credential helper opened no socket" >&2; exit 1; }

echo "==> Giving each account its own Personal Core owner..."
start_owner "$FIRST" "$FIRST_UID" first
first_pid=$!
start_owner "$SECOND" "$SECOND_UID" second
second_pid=$!

echo "==> Starting the gateway in front of both..."
CYBOU_AUTH_SOCKET="$AUTH_SOCKET" \
CYBOU_PERSONAL_SOCKET_DIR="$SOCKET_DIR" \
CYBOU_GATEWAY_ADDR="127.0.0.1:$PORT" \
    "$CARGO_TARGET_DIR/debug/cybou-web-gateway" >"$WORK/gateway.log" 2>&1 &
gateway_pid=$!
for _ in $(seq 1 100); do curl -fs -o /dev/null "$BASE/api/v1/session" && break; sleep 0.2; done
curl -fs -o /dev/null "$BASE/api/v1/session" || { echo "the gateway never answered" >&2; exit 1; }

sign_in() {
    local account="$1" secret="$2" jar="$3"
    local status
    status="$(curl -s -o "$WORK/login.json" -w '%{http_code}' -c "$jar" \
        -H 'content-type: application/json' \
        -d "{\"username\":\"$account\",\"password\":\"$secret\"}" "$BASE/api/v1/login")"
    [ "$status" = "200" ] || {
        echo "signing in as $account answered HTTP $status" >&2
        cat "$WORK/login.json" >&2
        return 1
    }
}

notes_for() {
    curl -s -b "$1" -o "$WORK/notes.json" -w '%{http_code}' "$BASE/api/v1/personal/notes"
}

echo "=== Both accounts sign in, and each is a different person to this host ==="
sign_in "$FIRST" "$FIRST_SECRET" "$WORK/first.jar"
sign_in "$SECOND" "$SECOND_SECRET" "$WORK/second.jar"
echo "    ok      PAM accepted both and the gateway issued a session for each"

echo "=== One of them writes something private ==="
status="$(curl -s -b "$WORK/first.jar" -o "$WORK/created.json" -w '%{http_code}' \
    -H 'content-type: application/json' \
    -d '{"title":"Only mine","contentMarkdown":"the first account wrote this","tags":[],"isPinned":false}' \
    "$BASE/api/v1/personal/notes")"
[ "$status" = "200" ] || { echo "writing a note answered HTTP $status" >&2; cat "$WORK/created.json" >&2; exit 1; }

[ "$(notes_for "$WORK/first.jar")" = "200" ] || { echo "the writer could not read their own notes" >&2; exit 1; }
python3 - "$WORK/notes.json" <<'PYTHON'
import json, sys
notes = json.load(open(sys.argv[1]))["notes"]
assert [note["title"] for note in notes] == ["Only mine"], notes
PYTHON
echo "    ok      the account that wrote it reads it back"

echo "=== And the other account is a different mailbox, not a filtered view of the same one ==="
[ "$(notes_for "$WORK/second.jar")" = "200" ] || { echo "the second account could not read its own notes" >&2; exit 1; }
python3 - "$WORK/notes.json" <<'PYTHON'
import json, sys
notes = json.load(open(sys.argv[1]))["notes"]
assert notes == [], f"the second account was shown somebody else's notes: {notes}"
PYTHON
echo "    ok      empty, because it is a different owner and not the same store filtered"

echo "=== On disk too: each account's records are in that account's own file ==="
# The whole of each account's storage, write-ahead log included: a running owner may not have
# checkpointed yet, and a check that read only the main file would pass for the wrong reason.
grep -qa "the first account wrote this" "$WORK/stores/$FIRST_UID/"* || {
    echo "the note is not in the storage of the account that wrote it" >&2
    exit 1
}
if grep -qa "the first account wrote this" "$WORK/stores/$SECOND_UID/"* 2>/dev/null; then
    echo "one account's note was written into the other's storage" >&2
    exit 1
fi
echo "    ok      one file each, and neither holds the other's words"

echo "=== And somebody who signed in as nobody gets nobody's records ==="
status="$(curl -s -o "$WORK/stranger.json" -w '%{http_code}' "$BASE/api/v1/personal/notes")"
[ "$status" = "403" ] || [ "$status" = "401" ] || {
    echo "a request with no session answered HTTP $status" >&2
    cat "$WORK/stranger.json" >&2
    exit 1
}
echo "    ok      refused with $status, before any owner was asked anything"

echo "=== An account whose owner is gone is unavailable, never an empty mailbox ==="
kill "$first_pid" 2>/dev/null || true
wait "$first_pid" 2>/dev/null || true
rm -f "$SOCKET_DIR/$FIRST_UID/personal.sock"
status="$(notes_for "$WORK/first.jar")"
[ "$status" = "503" ] || {
    echo "an account with no owner running answered HTTP $status where 503 was the truth" >&2
    cat "$WORK/notes.json" >&2
    exit 1
}
echo "    ok      503: the host says it cannot reach that person's records, rather than saying there are none"

echo "=== personal privacy gate passed: two accounts, two owners, one number deciding which ==="
