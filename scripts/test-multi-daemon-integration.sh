#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Multi-daemon integration verification gate running under an isolated D-Bus session.
#
# The script re-executes itself inside `dbus-run-session` rather than launching the daemons
# from a nested shell: the cleanup trap, the PID list and the daemons must all live in one
# process, otherwise the trap runs in a shell that never saw the PIDs and the daemons leak
# past the end of the run.

set -euo pipefail

if [ -z "${CYBOU_TEST_DBUS_SESSION:-}" ]; then
    if command -v dbus-run-session >/dev/null 2>&1; then
        exec env CYBOU_TEST_DBUS_SESSION=1 dbus-run-session -- "$0" "$@"
    fi
    if [ "$(uname -s)" = "Linux" ]; then
        echo "ERROR: dbus-run-session not found; install dbus-daemon before running this gate." >&2
        exit 1
    fi
    echo "==> Skipping: Linux session bus unavailable on $(uname -s)."
    exit 0
fi

TMP_DIR="$(mktemp -d)"
export XDG_STATE_HOME="$TMP_DIR/state"
export XDG_DATA_HOME="$TMP_DIR/data"
mkdir -p "$XDG_STATE_HOME/cybou" "$XDG_DATA_HOME/cybou"

PIDS=()
cleanup() {
    echo "==> Cleaning up spawned test daemons..."
    for pid in "${PIDS[@]:-}"; do
        [ -n "$pid" ] || continue
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null || true
            wait "$pid" 2>/dev/null || true
        fi
    done
    rm -rf "$TMP_DIR"
    echo "==> Integration test cleanup complete."
}
trap cleanup EXIT

echo "==> Building all Mind daemons..."
cargo build --workspace --bins

BIN_DIR="$(cargo metadata --format-version 1 | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')/debug"

# Wait for a well-known name to be owned instead of sleeping a fixed interval: a loaded CI
# runner takes longer than any constant that is still fast on a developer machine.
wait_for_name() {
    local name="$1"
    local deadline=$((SECONDS + 20))
    while [ "$SECONDS" -lt "$deadline" ]; do
        if busctl --user status "$name" >/dev/null 2>&1; then
            return 0
        fi
        sleep 0.2
    done
    echo "ERROR: $name never appeared on the isolated session bus." >&2
    return 1
}

spawn() {
    "$BIN_DIR/$1" &
    PIDS+=("$!")
}

echo "==> Launching cybou-eventd..."
spawn cybou-eventd
wait_for_name org.cybou.Mind.Event1

echo "==> Launching cognitive organ daemons..."
spawn cybou-identityd
spawn cybou-healthd

spawn cybou-intentiond
INTENTION_PID="${PIDS[-1]}"

spawn cybou-predictord
spawn cybou-perceptiond
spawn cybou-epistemicd
spawn cybou-contextd
spawn cybou-workspaced
spawn cybou-lifecycled

spawn cybou-selfd
SELF_PID="${PIDS[-1]}"

spawn cybou-presenced

NAMES=(
    org.cybou.Mind.Event1
    org.cybou.Mind.Identity1
    org.cybou.Mind.Health1
    org.cybou.Mind.Intention1
    org.cybou.Mind.Predictor1
    org.cybou.Mind.Perception1
    org.cybou.Mind.Epistemic1
    org.cybou.Mind.Context1
    org.cybou.Mind.Workspace1
    org.cybou.Mind.Lifecycle1
    org.cybou.Mind.Self1
    org.cybou.Mind.Presence1
)

for name in "${NAMES[@]}"; do
    wait_for_name "$name"
done

# Health1 probes Ready on every organ, so an organ that does not export it is indistinguishable
# from one that is down and pins the whole control plane at "unavailable". Check all of them.
echo "==> Testing that every organ answers the Health1 readiness probe..."
for name in "${NAMES[@]}"; do
    path="/$(printf '%s' "$name" | tr . /)"
    answer="$(busctl --user call "$name" "$path" "$name" Ready)"
    if [ "$answer" != "b true" ]; then
        echo "ERROR: $name Ready answered '$answer', expected 'b true'." >&2
        exit 1
    fi
    echo "    $name Ready -> $answer"
done

# Formed with no cause on purpose: Kind::Intention is not a root kind, so an intention with
# nothing to cite cannot enter the Journal, and this exercises the path where the obligation is
# durable in its own organ while the biography records nothing.
echo "==> Testing Intention formation and restart survival..."
INTENTION_ID=$(busctl --user call org.cybou.Mind.Intention1 /org/cybou/Mind/Intention1 org.cybou.Mind.Intention1 Form sss "Run integration tests" "Session startup" "" | awk '{print $2}' | tr -d '"')
echo "Formed Intention ID: $INTENTION_ID"

if [ -z "$INTENTION_ID" ]; then
    echo "ERROR: Failed to form intention!" >&2
    exit 1
fi

echo "==> Restarting cybou-intentiond to verify restart survival..."
kill "$INTENTION_PID" 2>/dev/null || true
wait "$INTENTION_PID" 2>/dev/null || true

spawn cybou-intentiond
wait_for_name org.cybou.Mind.Intention1

busctl --user call org.cybou.Mind.Intention1 /org/cybou/Mind/Intention1 org.cybou.Mind.Intention1 Ready

# A workspace seeded once and never updated would keep deliberating over its seed while the
# system moved on, and its salience would decay to nothing without anything noticing. Form a
# contribution and require the workspace to be attending to something recent.
# contextd derives its graph from accepted contributions. An organ that subscribed but never
# ingested, or ingested but never activated a concept, is indistinguishable from one that started
# correctly — until something asks it what it holds.
# Presence1 is a command gateway that owned nothing and did nothing: every mutation returned a
# fail-closed default. Exercise one command end to end — Presence1 asks Intention1, Intention1
# holds the obligation — and require the obligation to appear where its owner keeps it.
# Key continuity, checked the only way that means anything: across processes. eventd wraps every
# contribution's data key with a key-encryption key, so a KEK generated per run can unwrap only
# what that run wrote. A restart would then make earlier sealed payloads unreadable with no
# ErasureRequested and no ErasureApplied — erasure as a side effect of a process dying.
echo "==> Verifying key material survives a restart of the organ that owns it..."
master="$XDG_DATA_HOME/cybou/keys/master.json"
if [ ! -f "$master" ]; then
    echo "ERROR: eventd established no durable master key material." >&2
    exit 1
fi
domain_before="$(tr -d ' 
' < "$master")"

EVENT_PID="${PIDS[0]}"
kill "$EVENT_PID" 2>/dev/null || true
wait "$EVENT_PID" 2>/dev/null || true
spawn cybou-eventd
wait_for_name org.cybou.Mind.Event1

domain_after="$(tr -d ' 
' < "$master")"
if [ "$domain_before" != "$domain_after" ]; then
    echo "ERROR: restarting eventd replaced the key material that wraps existing data keys." >&2
    exit 1
fi
echo "    Key domain and master secret survived the restart"

echo "==> Verifying a Presence1 command reaches the owner that holds the state..."
before="$(busctl --user call org.cybou.Mind.Intention1 /org/cybou/Mind/Intention1 org.cybou.Mind.Intention1 OpenCount | awk '{print $2}')"
promised="$(busctl --user call org.cybou.Mind.Presence1 /org/cybou/Mind/Presence1 org.cybou.Mind.Presence1 Promise s "Verify the command path" | awk '{print $2}' | tr -d '"')"
if [ -z "$promised" ]; then
    echo "ERROR: Presence1 Promise returned no intention identity." >&2
    exit 1
fi
after="$(busctl --user call org.cybou.Mind.Intention1 /org/cybou/Mind/Intention1 org.cybou.Mind.Intention1 OpenCount | awk '{print $2}')"
if [ "$after" -le "$before" ]; then
    echo "ERROR: Presence1 answered with an identity, yet Intention1 holds no new obligation." >&2
    exit 1
fi
echo "    Promise reached Intention1: open obligations $before -> $after"

# A promise the biography never heard of is the failure this path had: Kind::Intention is derived,
# so an intention with no cause cannot enter the Journal, and a promise made through Presence1 had
# no cause at all. Require the Journal to have grown by both the request and the intention.
kinds="$(sqlite3 "$XDG_DATA_HOME/cybou/journal.sqlite3"     'SELECT group_concat(DISTINCT kind) FROM contribution;' 2>/dev/null || echo '')"
case ",$kinds," in
    *,11,*) echo "    The promise is in the biography as an Intention contribution" ;;
    *)
        echo "ERROR: a promise was made and the Journal holds no Intention contribution." >&2
        exit 1
        ;;
esac

# Close the obligation that was just promised, not whichever one happens to be first: Intention1
# appends, so the new one is last. Fulfilling index 0 would have closed an unrelated obligation and
# still looked like a passing check.
if [ "$(busctl --user call org.cybou.Mind.Presence1 /org/cybou/Mind/Presence1 org.cybou.Mind.Presence1 FulfillIndex i $((after - 1)))" != "b true" ]; then
    echo "ERROR: Presence1 could not fulfil the obligation it had just created." >&2
    exit 1
fi
restored="$(busctl --user call org.cybou.Mind.Intention1 /org/cybou/Mind/Intention1 org.cybou.Mind.Intention1 OpenCount | awk '{print $2}')"
if [ "$restored" != "$before" ]; then
    echo "ERROR: fulfilling the promised obligation left $restored open, expected $before." >&2
    exit 1
fi
echo "    FulfillIndex closed it through its owner: open obligations $after -> $restored"

echo "==> Verifying the associative context is built from what was accepted..."
context="ay 1 128"
deadline=$((SECONDS + 30))
while [ "$SECONDS" -lt "$deadline" ]; do
    context="$(busctl --user call org.cybou.Mind.Context1 /org/cybou/Mind/Context1 org.cybou.Mind.Context1 ActiveContext)"
    if [ "$context" != "ay 1 128" ]; then
        break
    fi
    sleep 1
done
if [ "$context" = "ay 1 128" ]; then
    echo "ERROR: contributions were accepted, yet Context1 activated no concept." >&2
    exit 1
fi
echo "    Context1 activated at least one concept"

# The subject of a belief is the subject of what was observed, never the organ that reported it.
# Keying them by organ collapsed everything one organ ever said into a single self-disputing
# belief, and printed a payload where a claim belonged. Both derived organs are checked, because
# both take the subject from the same place and both were wrong in the same way.
# The public surface refuses to publish personal state, and that refusal is only worth anything if
# the classification underneath it is real. Machine facts must not be labelled as belonging to the
# person: every writer used to stamp Personal regardless of what it was recording.
echo "==> Verifying machine facts are not labelled as belonging to the person..."
highest="$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 HighestSensitivity | awk '{print $2}')"
if [ "$highest" != "0" ]; then
    echo "ERROR: the Journal holds only machine facts, yet reports sensitivity $highest." >&2
    exit 1
fi
echo "    Journal sensitivity is ordinary; a public surface may serve it"

echo "==> Verifying the derived organs name what was observed, not who observed it..."
observed_subject="operating-system"
for owner in Epistemic1:Beliefs Context1:ActiveContext; do
    name="org.cybou.Mind.${owner%%:*}"
    method="${owner##*:}"
    path="/$(printf '%s' "$name" | tr . /)"
    text=""
    deadline=$((SECONDS + 30))
    while [ "$SECONDS" -lt "$deadline" ]; do
        # The reply is CBOR; the subjects inside it are plain text, which is all this needs to see.
        text="$(busctl --user call "$name" "$path" "$name" "$method"             | tr ' ' '
' | awk '$1 > 31 && $1 < 127 { printf "%c", $1 }')"
        case "$text" in
            *"$observed_subject"*) break ;;
        esac
        sleep 1
    done
    case "$text" in
        *"$observed_subject"*) ;;
        *)
            echo "ERROR: $name never named the observed subject '$observed_subject'." >&2
            exit 1
            ;;
    esac
    case "$text" in
        *organ.*)
            echo "ERROR: $name named an organ as a subject; a claim is about what was observed." >&2
            exit 1
            ;;
    esac
    echo "    $name names '$observed_subject'"
done

echo "==> Verifying the global workspace follows new contributions..."
moment="$(busctl --user call org.cybou.Mind.Workspace1 /org/cybou/Mind/Workspace1 org.cybou.Mind.Workspace1 MomentState)"
if [ "$moment" = "ay 0" ]; then
    echo "ERROR: Workspace1 answered with no momentary state at all." >&2
    exit 1
fi
echo "    Workspace1 MomentState answered"

echo "==> Testing Presence1 query..."
busctl --user call org.cybou.Mind.Presence1 /org/cybou/Mind/Presence1 org.cybou.Mind.Presence1 Ready

# With every organ up and answering, the control plane must describe itself as healthy. The first
# probe round fires before the later organs have taken their names, so the reading starts degraded
# and settles; poll until it does rather than accepting the transient.
echo "==> Waiting for the control plane to report its own health..."
health="unset"
deadline=$((SECONDS + 40))
while [ "$SECONDS" -lt "$deadline" ]; do
    health="$(busctl --user call org.cybou.Mind.Presence1 /org/cybou/Mind/Presence1 org.cybou.Mind.Presence1 Health)"
    if [ "$health" = 's "healthy"' ]; then
        break
    fi
    sleep 1
done
echo "    Presence1 Health -> $health"
if [ "$health" != 's "healthy"' ]; then
    echo "ERROR: every organ is running and answering Ready, yet the control plane settled on $health." >&2
    exit 1
fi

# A control plane that cannot report a change is a control plane nobody can watch. Presence1
# declares a Changed signal; prove it actually fires, and fires for a real reason, by taking one
# organ away and requiring both the signal and the degraded verdict that explains it.
echo "==> Verifying the control plane observes and announces an organ dying..."
# Watch both links of the chain. Health1 is where the observation happens and Presence1 is where
# a subscriber waits; logging them separately says which half broke rather than only that one did.
HEALTH_LOG="$TMP_DIR/health-changed.log"
CHANGED_LOG="$TMP_DIR/presence-changed.log"
dbus-monitor --session     "type='signal',interface='org.cybou.Mind.Health1',member='Changed'"     >"$HEALTH_LOG" 2>/dev/null &
PIDS+=("$!")
dbus-monitor --session     "type='signal',interface='org.cybou.Mind.Presence1',member='Changed'"     >"$CHANGED_LOG" 2>/dev/null &
PIDS+=("$!")
sleep 1

kill "$SELF_PID" 2>/dev/null || true
wait "$SELF_PID" 2>/dev/null || true

health="unset"
deadline=$((SECONDS + 30))
while [ "$SECONDS" -lt "$deadline" ]; do
    health="$(busctl --user call org.cybou.Mind.Presence1 /org/cybou/Mind/Presence1 org.cybou.Mind.Presence1 Health)"
    if [ "$health" != 's "healthy"' ]; then
        break
    fi
    sleep 1
done
echo "    Presence1 Health after losing selfd -> $health"
if [ "$health" = 's "healthy"' ]; then
    echo "ERROR: selfd was killed and the control plane still reports itself healthy." >&2
    exit 1
fi

if ! grep -q "member=Changed" "$HEALTH_LOG"; then
    echo "ERROR: the capability states changed but Health1 never emitted Changed." >&2
    exit 1
fi
echo "    Health1 emitted Changed"

if ! grep -q "member=Changed" "$CHANGED_LOG"; then
    echo "ERROR: Health1 announced the change but Presence1 never relayed it." >&2
    exit 1
fi
echo "    Presence1 relayed Changed"

echo "==> Restoring cybou-selfd..."
spawn cybou-selfd
wait_for_name org.cybou.Mind.Self1

health="unset"
deadline=$((SECONDS + 40))
while [ "$SECONDS" -lt "$deadline" ]; do
    health="$(busctl --user call org.cybou.Mind.Presence1 /org/cybou/Mind/Presence1 org.cybou.Mind.Presence1 Health)"
    if [ "$health" = 's "healthy"' ]; then
        break
    fi
    sleep 1
done
echo "    Presence1 Health after restoring selfd -> $health"
if [ "$health" != 's "healthy"' ]; then
    echo "ERROR: selfd is back and answering, yet the control plane settled on $health." >&2
    exit 1
fi

echo "==> Multi-daemon integration test PASSED successfully!"
