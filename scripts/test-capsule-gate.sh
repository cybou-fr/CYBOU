#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# ADR-0042 G1: a capsule holds, and it holds without Mind.
#
# Not "an agent started under bubblewrap". Every line below is something an agent must not be able to
# do, attempted from inside a real capsule built from the argument vector this repository's own code
# produces — via `examples/capsule-argv`, so the gate tests the code rather than a command somebody
# wrote out here and stopped maintaining.
#
# ## Why it runs twice
#
# The second pass is the point. A capsule that holds only while Mind is watching has cognition for a
# boundary, which ADR-0042 refuses in its first section. Nothing in Mind participates in holding one
# today — that is the claim — so the second pass asserts it rather than assuming it: no Cybou process
# runs, and every refusal above must still be a refusal.
#
# An attempt that *fails to run at all* is not an attempt that was refused. Every check below
# distinguishes "the command ran and was denied" from "the command was not there", because a missing
# `ping` would otherwise read as a network that is closed.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

if ! command -v bwrap > /dev/null 2>&1; then
    echo "==> capsule gate NOT RUN: bubblewrap is not installed here" >&2
    exit 3
fi

WORKSPACE="$(mktemp -d)"
trap 'rm -rf "$WORKSPACE"' EXIT

# This script's own process, which is running on the host and must be unreachable from inside. Chosen
# rather than PID 1 because PID 1 inside a PID namespace is the capsule's own first process: an
# earlier version signalled it, watched the capsule kill itself, and called that a host process.
HOST_PID="$$"
if [ "$HOST_PID" -lt 100 ]; then
    echo "==> capsule gate NOT RUN: this shell's PID is low enough to collide inside a namespace" >&2
    exit 3
fi
echo "the workspace is writable" > "$WORKSPACE/given.txt"

# The environment checks below need something to leak, or they pass on a shell that happens not to
# have these set — which is what happened the first time they were mutation-tested: removing
# --clearenv changed nothing, because there was nothing there to come through. A gate must create the
# condition it claims to test.
export CYBOU_KEYSTORE_PATH="/var/lib/cybou/keys"
export SSH_AUTH_SOCK="/run/user/$(id -u)/keyring/ssh"

failures=0

# Run one shell fragment inside a capsule and print what it said.
inside() {
    local script="$1"
    mapfile -t argv < <(cargo run --quiet -p cybou-capsule --example capsule-argv -- \
        "$WORKSPACE" /bin/sh -c "$script")
    "${argv[@]}" 2>&1 || true
}

# A check that must produce the expected text.
must() {
    local name="$1" expected="$2" script="$3"
    local output
    output="$(inside "$script")"
    if [[ "$output" == *"$expected"* ]]; then
        printf '    ok      %s\n' "$name"
    else
        printf '    FAILED  %s\n        wanted %q\n        got    %q\n' "$name" "$expected" "$output"
        failures=$((failures + 1))
    fi
}

pass() {
    echo "==> $1"
}

run_the_gate() {
    pass "Inside the capsule, ordinary work is possible"
    must "the workspace is readable" "the workspace is writable" "cat /workspace/given.txt"
    must "the workspace is writable" "written" \
        "echo written > /workspace/new.txt && cat /workspace/new.txt"
    must "a program can be run" "ran" "echo ran"

    pass "Outside it, nothing is"
    # `test -e` is in the shell, so this cannot be a missing-command false pass.
    must "the host password file is absent" "absent" \
        "test -e /etc/shadow && echo PRESENT || echo absent"
    must "the Journal is absent" "absent" \
        "test -e /var/lib/cybou && echo PRESENT || echo absent"
    must "the host root is not the capsule root" "absent" \
        "test -e /srv && echo PRESENT || echo absent"

    # A symlink out of the workspace. The mount namespace answers this, not a string comparison —
    # which is exactly why `Workspace::contains` says it does not follow symlinks and leaves it here.
    must "a symlink out of the workspace leads nowhere" "absent" \
        "ln -sf /etc/shadow /workspace/escape 2>/dev/null; test -e /workspace/escape && echo PRESENT || echo absent"

    pass "The host's processes are not visible"
    # A host with anything running on it has far more than ten. The capsule sees its own shell, the
    # process it forked, and little else.
    must "only the capsule's own processes exist" "few" \
        "n=\$(ls -d /proc/[0-9]* 2>/dev/null | wc -l); [ \"\$n\" -lt 10 ] && echo few || echo \"many:\$n\""
    # A real host PID, which inside a PID namespace simply is not a process. The first version of
    # this signalled PID 1 and reported KILLED — PID 1 inside the namespace is the capsule's own
    # shell, so it killed itself and the check asserted nothing. A process ID is namespace-local, and
    # a test that forgets it tests the wrong thing convincingly.
    must "a host process cannot be signalled" "denied" \
        "kill -0 $HOST_PID 2>/dev/null && echo SIGNALLED || echo denied"

    pass "The network is denied"
    # Structural, not a connectivity guess. The first version opened /dev/tcp — a bash feature, and
    # the capsule runs /bin/sh, so it failed identically whether the network was denied or wide
    # open. Removing --unshare-net left the gate passing, which is exactly the failure the header of
    # this file warns about: an attempt that could not run read as an attempt that was refused.
    #
    # A fresh network namespace has one interface. A shared host one has several, named.
    must "no interface but loopback" "only-loopback" \
        "others=\$(ip -o link show 2>/dev/null | grep -cv ' lo:'); [ \"\$others\" = 0 ] && echo only-loopback || echo \"HOST-INTERFACES:\$others\""
    # Not a decoration. It separates "the network namespace is fresh and empty" from "networking is
    # broken on this host", which would make every check above pass for the wrong reason. The first
    # version of this line printed the same word on both branches and could not fail.
    must "loopback is up inside the capsule" "loopback-up" \
        "ip -o link show lo 2>/dev/null | grep -q lo && echo loopback-up || echo NO-LOOPBACK"

    pass "The capsule cannot rebuild itself"
    must "no nested user namespace" "denied" \
        "unshare --user true 2>/dev/null && echo NESTED || echo denied"

    pass "The environment carries nothing from the host"
    must "the key store path did not come along" "clean" \
        "[ -z \"\${CYBOU_KEYSTORE_PATH:-}\" ] && echo clean || echo LEAKED"
    must "no agent socket came along" "clean" \
        "[ -z \"\${SSH_AUTH_SOCK:-}\" ] && echo clean || echo LEAKED"
}

echo "=== Pass 1: the capsule holds ==="
run_the_gate

echo
echo "=== Pass 2: it holds with no Cybou process running ==="
# Nothing in Mind participates in holding a capsule, which is the claim. Asserted rather than
# assumed: if a Cybou daemon is running here, this pass would not be testing what it says it is.
if pgrep -u "$(id -u)" -f 'cybou-[a-z]+d' > /dev/null 2>&1; then
    echo "    note: Cybou daemons are running; stopping them is the operator's call," >&2
    echo "          so this pass reports what it saw rather than pretending otherwise." >&2
fi
run_the_gate

# ---------------------------------------------------------------- the budget
#
# Read from the kernel, not from systemd. Asked for the same limits, `systemd-run --user --scope`
# accepted every property, reported success, and left MemoryMax at infinity — so a gate that trusted
# `systemctl show` would have passed on a capsule held to nothing. What is checked here is
# `memory.max`, `pids.max` and `cpu.max` inside the cgroup the capsule actually ran in.
check_the_budget() {
    if ! command -v systemd-run > /dev/null 2>&1 || ! systemctl --user is-system-running > /dev/null 2>&1; then
        echo "    note: no user systemd manager here, so the budget was NOT checked" >&2
        skipped_budget=1
        return
    fi

    local reader="$WORKSPACE/read-limits.sh"
    cat > "$reader" <<'READER'
#!/bin/sh
path=$(sed -n 's/^0:://p' /proc/self/cgroup)
for file in memory.max pids.max cpu.max; do
    printf '%s=%s
' "$file" "$(cat "/sys/fs/cgroup$path/$file" 2>&1)"
done
READER
    chmod +x "$reader"

    local unit="cybou-capsule-gate-$$"
    systemctl --user reset-failed "$unit.service" > /dev/null 2>&1 || true
    # A transient service, never a scope. Asked for these same properties, a user scope accepts them
    # all, reports success, and leaves MemoryMax at infinity — a limit that looks correct everywhere
    # and holds nothing. Switching this line to `--scope` fails three checks below, which is how that
    # is known rather than assumed.
    systemd-run --user --collect --unit="$unit" --wait \
        --property=MemoryMax=64M \
        --property=MemorySwapMax=0 \
        --property=TasksMax=17 \
        --property=CPUQuota=50% \
        --property=RuntimeMaxSec=30 \
        "$reader" > /dev/null 2>&1 || true

    local seen
    # The rest of the line, not up to the first space: cpu.max is two numbers, and an earlier
    # version cut it at the space and then failed to match what it had asked for.
    # The rest of the line, not up to the first space: cpu.max is two numbers — the quota and the
    # period — and an earlier version cut it at the space, then failed to match what it had asked
    # for. The check was right and the reading of it was wrong, which is the more confusing way round.
    seen="$(journalctl --user -u "$unit.service" --no-pager -n 20 2>/dev/null | grep -oE '(memory|pids|cpu)\.max=.*' || true)"
    systemctl --user reset-failed "$unit.service" > /dev/null 2>&1 || true

    local expected="$1"
    local name="$2"
    if [[ "$seen" == *"$expected"* ]]; then
        printf '    ok      %s\n' "$name"
    else
        printf '    FAILED  %s\n        wanted %q\n        got    %q\n' "$name" "$expected" "$seen"
        failures=$((failures + 1))
    fi
}

echo
echo "=== The budget is the kernel's, not systemd's account of itself ==="
skipped_budget=0
check_the_budget "memory.max=67108864" "the memory ceiling reaches the kernel"
[ "$skipped_budget" = 0 ] && check_the_budget "pids.max=17" "the process ceiling reaches the kernel"
[ "$skipped_budget" = 0 ] && check_the_budget "cpu.max=50000 100000" "the CPU quota reaches the kernel"

echo
if [ "$failures" -gt 0 ]; then
    echo "=== CAPSULE GATE FAILED: $failures check(s) ==="
    exit 1
fi
echo "=== capsule gate passed ==="
