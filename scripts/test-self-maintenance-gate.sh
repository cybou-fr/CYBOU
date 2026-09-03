#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# A host that repairs itself, with nobody driving it.
#
# The existing action gate proves the vertical works when something runs it, and the thing that runs
# it is `action-roundtrip` — an example written for that gate. So the loop was demonstrated and never
# autonomous, and every summary of this repository that said otherwise, including several of mine,
# was describing the example.
#
# Here nothing drives it. Four daemons are started, a harmless unit is stopped, and then the gate
# waits. What it waits for is the host noticing on its own, proposing on its own, being permitted by
# a standing policy an operator set, carrying it out, looking again, and recording what it saw.
#
# Exit 3 means this host has no root systemd to run that on, which is a check that did not run rather
# than one that passed.
set -euo pipefail

cd "$(dirname "$0")/.."
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

not_run() {
    echo "==> self-maintenance gate NOT RUN: $1" >&2
    exit 3
}

[ "$(id -u)" -eq 0 ] || not_run "a root systemd host is required"
command -v systemctl >/dev/null 2>&1 || not_run "systemctl is not available"
systemctl show --property=Version --value >/dev/null 2>&1 || not_run "systemd is not running here"
if [ -z "${CYBOU_SELF_MAINTENANCE_SESSION:-}" ]; then
    command -v dbus-run-session >/dev/null 2>&1 ||
        not_run "dbus-run-session is needed for the telemetry organ's bus"
    exec env CYBOU_SELF_MAINTENANCE_SESSION=1 dbus-run-session -- bash "$0" "$@"
fi

UNIT_NAME=cybou-self-maintenance-gate.service
UNIT="/run/systemd/system/$UNIT_NAME"
DBUS_POLICY="/etc/dbus-1/system.d/cybou-self-maintenance-$$.conf"
WORK="$(mktemp -d)"
export XDG_STATE_HOME="$WORK/state"
export XDG_DATA_HOME="$WORK/data"
mkdir -p "$XDG_STATE_HOME/cybou" "$XDG_DATA_HOME/cybou"
pids=()

cleanup() {
    status=$?
    if [ "$status" -ne 0 ]; then
        for log in "$WORK"/*.log; do
            [ -f "$log" ] && { echo "--- $log"; tail -30 "$log"; } >&2
        done
    fi
    for pid in "${pids[@]:-}"; do
        [ -n "$pid" ] && kill "$pid" >/dev/null 2>&1 || true
    done
    wait >/dev/null 2>&1 || true
    systemctl stop "$UNIT_NAME" >/dev/null 2>&1 || true
    rm -f "$UNIT"
    systemctl daemon-reload >/dev/null 2>&1 || true
    rm -f "$DBUS_POLICY"
    systemctl reload dbus.service >/dev/null 2>&1 || true
    rm -rf "$WORK"
    return "$status"
}
trap cleanup EXIT

cargo build --quiet --locked -p cybou-telemetryd -p cybou-actiond -p cybou-executord \
    -p cybou-remediationd
BIN="$CARGO_TARGET_DIR/debug"

# A unit that does nothing and can be stopped and started without consequence. The finding this gate
# waits for is about this and nothing else on the host.
install -m 0644 /dev/stdin "$UNIT" <<'EOF'
[Unit]
Description=Harmless Cybou self-maintenance gate unit

[Service]
Type=oneshot
ExecStart=/usr/bin/true
RemainAfterExit=yes
EOF
systemctl daemon-reload
systemctl start "$UNIT_NAME"
systemctl is-active --quiet "$UNIT_NAME"

install -m 0644 /dev/stdin "$DBUS_POLICY" <<'EOF'
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <policy user="root">
    <allow own="org.cybou.Mind.Action1"/>
    <allow send_destination="org.cybou.Mind.Action1"/>
    <allow own="org.cybou.Body.Executor1"/>
    <allow send_destination="org.cybou.Body.Executor1"/>
  </policy>
</busconfig>
EOF
systemctl reload dbus.service

spawn() {
    "$@" >>"$WORK/$(basename "$1").log" 2>&1 &
    pids+=("$!")
}

wait_for_system_name() {
    for _ in $(seq 1 100); do
        busctl --system list --no-legend 2>/dev/null | grep -q "^$1 " && return 0
        sleep 0.1
    done
    echo "$1 never appeared on the system bus" >&2
    return 1
}

# The telemetry organ watches this unit because an operator declared it. Nothing beyond the universal
# is watched by default: an undeclared thing is simply not this host's business, and a declared one
# that cannot be read is announced as unreadable rather than passed over.
export XDG_CONFIG_HOME="$WORK/config"
mkdir -p "$XDG_CONFIG_HOME/cybou"
printf 'service %s\n' "$UNIT_NAME" >"$XDG_CONFIG_HOME/cybou/telemetry.watch"
# The Journal, first. This gate runs on a session bus of its own, so whatever a host keeps is not
# here, and an execution that cannot be made durable is correctly refused by Action1 — which would
# make this gate fail for a reason that has nothing to do with what it is proving.
spawn "$BIN/cybou-eventd"
for _ in $(seq 1 100); do
    busctl --user list --no-legend 2>/dev/null | grep -q '^org.cybou.Mind.Event1 ' && break
    sleep 0.1
done
busctl --user list --no-legend 2>/dev/null | grep -q '^org.cybou.Mind.Event1 ' || {
    echo "the Journal never came up, so nothing here could be made durable" >&2
    exit 1
}

spawn "$BIN/cybou-telemetryd"
for _ in $(seq 1 100); do
    busctl --user list --no-legend 2>/dev/null | grep -q '^org.cybou.Mind.Telemetry1 ' && break
    sleep 0.1
done

export CYBOU_ACTION_SYSTEM_BUS=1
export CYBOU_EXECUTOR_SYSTEM_BUS=1
# The one operation this operator has pre-authorized. Everything else the host might want to do about
# anything reaches Action1 and is refused, which is the default this gate does not change.
export CYBOU_PREAUTHORIZED_ACTIONS=service.restart
spawn "$BIN/cybou-actiond"
spawn "$BIN/cybou-executord"
wait_for_system_name org.cybou.Mind.Action1
wait_for_system_name org.cybou.Body.Executor1

# And the thing under test. From here nothing in this script touches Action1 or the executor.
spawn "$BIN/cybou-remediationd"

# Now break it, and stop helping.
systemctl stop "$UNIT_NAME"
systemctl is-active --quiet "$UNIT_NAME" && {
    echo "the gate unit refused to stop, so there is nothing to notice" >&2
    exit 1
}

echo "==> waiting for this host to notice and repair itself"
repaired=0
for _ in $(seq 1 120); do
    if systemctl is-active --quiet "$UNIT_NAME"; then
        repaired=1
        break
    fi
    sleep 1
done

[ "$repaired" -eq 1 ] || {
    echo "the host did not repair itself within two minutes" >&2
    exit 1
}
echo "    ok      it noticed, proposed, was permitted, and carried it out"

# It was this host's own doing, and the log says which part did what. Without this the gate would
# pass on a unit that came back for any reason at all.
grep -q 'Carrying out service.restart' "$WORK/cybou-remediationd.log" || {
    echo "the unit came back but nothing here says this host restarted it" >&2
    exit 1
}

# And it looks again afterwards rather than believing the executor. The wait is the point: an answer
# taken before the remedy has taken effect is not an answer, and acting on one is worse than not
# looking.
echo "==> waiting for it to look again and conclude what happened"
for _ in $(seq 1 150); do
    grep -q 'service.restart for ' "$WORK/cybou-remediationd.log" && break
    sleep 1
done
grep -q 'service.restart for ' "$WORK/cybou-remediationd.log" || {
    echo "it acted and never looked again" >&2
    tail -20 "$WORK/cybou-remediationd.log" >&2
    exit 1
}
echo "    ok      it re-observed and concluded on its own"

# It repaired the host. The harder question is whether it *concluded* anything, and the gate that
# only checked the repair passed while the successful case was the one being lost: a remedy that works
# makes its finding disappear, and an episode concluded only from findings still present concludes
# every failure and never a success.
for _ in $(seq 1 40); do
    grep -q 'Relieved' "$WORK/cybou-remediationd.log" && break
    sleep 3
done
grep -q 'Relieved' "$WORK/cybou-remediationd.log" || {
    echo "the host repaired the service and never concluded that it had:" >&2
    cat "$WORK/cybou-remediationd.log" >&2
    exit 1
}

# And it stopped. Exactly one restart, however long it is left running: a second would mean the host
# could not tell "not yet" from "not this way".
restarts="$(grep -c 'Carrying out service.restart' "$WORK/cybou-remediationd.log")"
test "$restarts" = "1" || {
    echo "the host carried out $restarts restarts where one was right" >&2
    exit 1
}

# Now the part that has to survive a crash. Until this ran, "this host acts once on a finding" meant
# once per uninterrupted process, and Restart=on-failure would then have let a crash cause a second
# restart of a service. The crash has to land mid-episode, between acting and concluding, because
# that is the only window in which anything is owed.
echo "==> breaking it again and killing the driver between the act and the conclusion"
systemctl stop "$UNIT_NAME"
driver_pid="${pids[-1]}"
killed=0
for _ in $(seq 1 240); do
    if [ "$(grep -c 'Carrying out service.restart' "$WORK/cybou-remediationd.log")" -ge 2 ]; then
        kill -9 "$driver_pid" 2>/dev/null || true
        killed=1
        break
    fi
    sleep 0.25
done
wait "$driver_pid" 2>/dev/null || true
[ "$killed" -eq 1 ] || {
    echo "the host never acted a second time, so there was no episode to crash inside of" >&2
    cat "$WORK/cybou-remediationd.log" >&2
    exit 1
}

# Whatever it had concluded before dying is in the log, not in the process. The new one starts with
# nothing, and the whole question is whether it can find out what it owes.
mv "$WORK/cybou-remediationd.log" "$WORK/cybou-remediationd-first.log"
spawn "$BIN/cybou-remediationd"

for _ in $(seq 1 60); do
    grep -q 'Taking over' "$WORK/cybou-remediationd.log" 2>/dev/null && break
    sleep 0.5
done
grep -q 'Taking over' "$WORK/cybou-remediationd.log" || {
    echo "the restarted driver did not take over the episode it had left open:" >&2
    cat "$WORK/cybou-remediationd.log" >&2
    exit 1
}
echo "    ok      it asked the owner what it had left unfinished"

# And it finishes it rather than leaving it open forever. The finding that caused it is gone with the
# process that held it, so what it concludes is whatever can honestly be said with the record alone.
for _ in $(seq 1 60); do
    grep -q 'service.restart for ' "$WORK/cybou-remediationd.log" && break
    sleep 2
done
grep -q 'service.restart for ' "$WORK/cybou-remediationd.log" || {
    echo "the adopted episode was never concluded:" >&2
    cat "$WORK/cybou-remediationd.log" >&2
    exit 1
}
echo "    ok      it finished the episode it had inherited"

# And it did not redo it. The unit was already back before the crash; a driver that read its own
# amnesia as "nothing was tried" would restart it a third time.
inherited_restarts="$(grep -c 'Carrying out service.restart' "$WORK/cybou-remediationd.log" || true)"
test "$inherited_restarts" = "0" || {
    echo "the restarted driver acted $inherited_restarts more times on work already done" >&2
    cat "$WORK/cybou-remediationd.log" >&2
    exit 1
}
echo "    ok      it did not repeat what it had already carried out"

# The other side of the restart boundary is a concluded remedy that did not work. It is absent from
# UnfinishedEpisodes by definition, but it is still the decisive reason not to execute again. Make
# this same harmless unit permanently fail, let the driver reach StillPresent, then restart only the
# driver while the finding remains.
echo "==> leaving a remedy ineffective and restarting the driver after its terminal outcome"
FAILING_UNIT="$WORK/failing.service"
tee "$FAILING_UNIT" >/dev/null <<'EOF'
[Unit]
Description=Harmless failing Cybou self-maintenance gate unit

[Service]
Type=oneshot
ExecStart=/usr/bin/false
RemainAfterExit=yes
EOF
install -m 0644 "$FAILING_UNIT" "$UNIT"
systemctl daemon-reload

# Start this scenario with a fresh process and a fresh log. Renaming a live process's log would not
# redirect its open file descriptor; it would keep writing to the renamed inode and make the absence
# of the new path look like absence of an attempt.
driver_pid="${pids[-1]}"
kill "$driver_pid" 2>/dev/null || true
wait "$driver_pid" 2>/dev/null || true
mv "$WORK/cybou-remediationd.log" "$WORK/cybou-remediationd-unfinished-recovery.log"
spawn "$BIN/cybou-remediationd"
for _ in $(seq 1 40); do
    grep -q 'Watching what this host concludes' "$WORK/cybou-remediationd.log" 2>/dev/null && break
    sleep 0.25
done
systemctl stop "$UNIT_NAME"

for _ in $(seq 1 120); do
    grep -q 'Carrying out service.restart' "$WORK/cybou-remediationd.log" 2>/dev/null && break
    sleep 1
done
grep -q 'Carrying out service.restart' "$WORK/cybou-remediationd.log" || {
    echo "the host never attempted the remedy that must remain ineffective:" >&2
    cat "$WORK/cybou-remediationd.log" >&2
    exit 1
}

for _ in $(seq 1 60); do
    grep -q 'StillPresent' "$WORK/cybou-remediationd.log" && break
    sleep 2
done
grep -q 'StillPresent' "$WORK/cybou-remediationd.log" || {
    echo "the ineffective remedy never reached a terminal StillPresent outcome:" >&2
    cat "$WORK/cybou-remediationd.log" >&2
    exit 1
}
echo "    ok      the failed remedy was independently concluded StillPresent"

driver_pid="${pids[-1]}"
kill "$driver_pid" 2>/dev/null || true
wait "$driver_pid" 2>/dev/null || true
mv "$WORK/cybou-remediationd.log" "$WORK/cybou-remediationd-concluded-first.log"
spawn "$BIN/cybou-remediationd"

# Seeing this line proves the per-cause owner lookup worked. Merely observing zero executions would
# also pass if Action1 were unreachable and the driver conservatively declined to act.
for _ in $(seq 1 60); do
    grep -q 'Remembering the episode already carried out' "$WORK/cybou-remediationd.log" 2>/dev/null && break
    sleep 0.5
done
grep -q 'Remembering the episode already carried out' "$WORK/cybou-remediationd.log" || {
    echo "the restarted driver did not recover the concluded episode by cause:" >&2
    cat "$WORK/cybou-remediationd.log" >&2
    exit 1
}

# Leave it through another full consideration interval. The finding remains present, so any loss of
# the completed episode would produce another executor attempt here.
sleep 20
concluded_restarts="$(grep -c 'Carrying out service.restart' "$WORK/cybou-remediationd.log" || true)"
test "$concluded_restarts" = "0" || {
    echo "the restarted driver repeated a remedy already concluded ineffective" >&2
    cat "$WORK/cybou-remediationd.log" >&2
    exit 1
}
echo "    ok      a restart did not repeat the concluded ineffective remedy"

echo "=== Self-maintenance gate passed: nobody drove this, and restarts did not erase its memory ==="
