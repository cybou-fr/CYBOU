#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Two accounts, two Personal Core owners, two stores. A note written through one account's owner is
# not readable through the other's, nothing about one account is written into the other's store, and
# an account whose owner is not running is unreachable rather than answering an empty mailbox.
#
# Real processes, real Unix sockets, the same CBOR the gateway speaks. Exit 3 means the host
# boundary needed to run the proof is absent.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

not_run() { echo "==> personal owner gate NOT RUN: $1" >&2; exit 3; }
command -v cargo >/dev/null || not_run "cargo is not installed"
[ "$(id -u)" != "0" ] || not_run "the owner refuses to run as root, so this proof cannot run as root"

WORK="$(mktemp -d)"
ALICE=1000
BOB=1001

cleanup() {
    for pidfile in "$WORK"/*.pid; do
        [ -e "$pidfile" ] || continue
        kill "$(cat "$pidfile")" 2>/dev/null || true
    done
    rm -rf "$WORK"
}
trap cleanup EXIT

cargo build --quiet --locked -p cybou-personald --example ask-owner
cargo build --quiet --locked -p cybou-personald
PERSONALD="$CARGO_TARGET_DIR/debug/cybou-personald"
ASK="$CARGO_TARGET_DIR/debug/examples/ask-owner"

socket_for() { echo "$WORK/sockets/$1/personal.sock"; }

start_owner() {
    local uid="$1"
    mkdir -p "$WORK/sockets/$uid" "$WORK/stores/$uid"
    CYBOU_PERSONAL_STORE="$WORK/stores/$uid/personal.sqlite3" \
    CYBOU_PERSONAL_SOCKET="$(socket_for "$uid")" \
        "$PERSONALD" >"$WORK/$uid.log" 2>&1 &
    echo $! >"$WORK/$uid.pid"
    for _ in $(seq 1 100); do
        [ -S "$(socket_for "$uid")" ] && return 0
        sleep 0.1
    done
    cat "$WORK/$uid.log" >&2
    exit 1
}

# An account whose owner is not running has no socket to answer from. Unreachable is the honest
# state; an empty notes list would be a claim that the person has no notes.
mkdir -p "$WORK/sockets/$BOB"
if "$ASK" "$(socket_for "$BOB")" notes >/dev/null 2>&1; then
    echo "an owner that is not running still answered" >&2
    exit 1
fi

start_owner "$ALICE"
start_owner "$BOB"

"$ASK" "$(socket_for "$ALICE")" create "Alice only" >"$WORK/created"
grep -qx "Alice only" "$WORK/created" || { echo "the owner did not create the note" >&2; exit 1; }

"$ASK" "$(socket_for "$ALICE")" notes >"$WORK/alice"
"$ASK" "$(socket_for "$BOB")" notes >"$WORK/bob"

grep -qx "Alice only" "$WORK/alice" || {
    echo "the note is missing from the account that wrote it" >&2
    exit 1
}
if grep -qx "Alice only" "$WORK/bob"; then
    echo "one account's note was answered by another account's owner" >&2
    exit 1
fi

# Two accounts, two files. There is no shared store to partition wrongly.
test -s "$WORK/stores/$ALICE/personal.sqlite3" || {
    echo "the records were not written to the account's own store" >&2
    exit 1
}
if grep -qa "Alice only" "$WORK/stores/$BOB/personal.sqlite3" 2>/dev/null; then
    echo "one account's note was written into another account's store" >&2
    exit 1
fi

# A restarted owner still holds its own records and only its own.
kill "$(cat "$WORK/$ALICE.pid")"
sleep 0.2
rm -f "$(socket_for "$ALICE")"
start_owner "$ALICE"
"$ASK" "$(socket_for "$ALICE")" notes >"$WORK/alice-again"
grep -qx "Alice only" "$WORK/alice-again" || {
    echo "the account's records did not survive its owner restarting" >&2
    exit 1
}

echo "=== Personal owner gate passed: each account's records live in, and answer from, its own owner ==="
