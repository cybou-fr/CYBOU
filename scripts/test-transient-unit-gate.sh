#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# The mechanism behind installing software: a command runs as its own systemd unit, and this host
# can tell apart a command that succeeded, one that ran and disagreed, and one that never ran at
# all. Proven against a real service manager, so the executor's own sandbox never has to be opened
# up for a package manager.
#
# The user manager is used deliberately. What is being proven is the transient-unit mechanism and
# the reading of its result, and neither needs root — a proof that needed root would run less often
# and prove the same thing.
#
# Exit 3 means this host has no service manager to run it against.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

not_run() { echo "==> transient unit gate NOT RUN: $1" >&2; exit 3; }
command -v systemctl >/dev/null || not_run "systemctl is not installed"
systemctl --user is-system-running >/dev/null 2>&1 || not_run "there is no user service manager"
[ -n "${DBUS_SESSION_BUS_ADDRESS:-}" ] || not_run "there is no session bus"

cargo build --quiet --locked -p cybou-executord --example transient-unit
PROBE="$CARGO_TARGET_DIR/debug/examples/transient-unit"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

run_probe() {
    set +e
    "$PROBE" user "$@" >"$WORK/out" 2>"$WORK/err"
    echo $? >"$WORK/status"
    set -e
    cat "$WORK/out"
}

# A command that succeeds is reported as having run, and only after it actually finished.
answer="$(run_probe /bin/true)"
[ "$answer" = "ran" ] || { echo "a successful command was not reported as run: $answer" >&2; exit 1; }
[ "$(cat "$WORK/status")" = "0" ] || { echo "a successful command exited non-zero" >&2; exit 1; }

# A command that ran and disagreed is reported with its own exit status, not as a job failure.
answer="$(run_probe /bin/sh -c 'exit 7')"
case "$answer" in
    *"ran and exited 7"*) ;;
    *) echo "a command that exited 7 was reported as: $answer" >&2; exit 1 ;;
esac

# A command that never ran is a different sentence from one that ran and failed.
answer="$(run_probe /usr/bin/cybou-no-such-command)"
case "$answer" in
    *"could not execute its command"*) ;;
    *) echo "a command that could not be executed was reported as: $answer" >&2; exit 1 ;;
esac

# Nothing is left behind: every probe unit is reset after it is read.
if systemctl --user list-units --all --no-legend 'cybou-probe-*' 2>/dev/null | grep -q .; then
    echo "a probe unit was left loaded on this host" >&2
    systemctl --user list-units --all --no-legend 'cybou-probe-*' >&2
    exit 1
fi

echo "=== Transient unit gate passed: a command runs in its own unit and this host can say what it did ==="
