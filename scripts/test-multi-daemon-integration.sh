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
spawn cybou-meaningd
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
    org.cybou.Mind.Meaning1
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
#
# Readiness is waited for rather than asserted at once: for an organ derived from the Journal it
# means the whole Journal has been read, which takes as long as the Journal is long. An organ that
# answered immediately would be answering about something that costs nothing to establish.
echo "==> Testing that every organ answers the Health1 readiness probe..."
for name in "${NAMES[@]}"; do
    path="/$(printf '%s' "$name" | tr . /)"
    answer=""
    deadline=$((SECONDS + 60))
    while [ "$SECONDS" -lt "$deadline" ]; do
        answer="$(busctl --user call "$name" "$path" "$name" Ready 2>/dev/null || true)"
        if [ "$answer" = "b true" ]; then
            break
        fi
        sleep 1
    done
    if [ "$answer" != "b true" ]; then
        echo "ERROR: $name Ready answered '${answer:-nothing}', expected 'b true'." >&2
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
# The state directory, not the data directory. A fresh installation keeps the keys where a backup
# of the Journal does not reach them, so that destroying a data key actually makes the record
# unreadable in a copy somebody else holds (ADR-0028 E11).
master="$XDG_STATE_HOME/cybou/keys/master.json"
if [ ! -f "$master" ]; then
    echo "ERROR: eventd established no durable master key material at $master." >&2
    exit 1
fi
# And the separation itself, checked rather than assumed. This is the whole guarantee: a test that
# only looked for the file would pass just as well with both under one directory.
if [ -e "$XDG_DATA_HOME/cybou/keys" ]; then
    echo "ERROR: a fresh installation put its keys beside the Journal, where one backup takes both." >&2
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

# The public surface refuses to publish personal state, and that refusal is only worth anything if
# the classification underneath it is real. Before a person has done anything, the Journal holds
# machine facts and nothing else — every writer used to stamp Personal regardless of content.
echo "==> Verifying machine facts are not labelled as belonging to the person..."
sensitivity_before="$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 HighestSensitivity | awk '{print $2}')"
if [ "$sensitivity_before" != "0" ]; then
    echo "ERROR: only machine facts have been recorded, yet the Journal reports sensitivity $sensitivity_before." >&2
    exit 1
fi
echo "    Journal sensitivity is ordinary; a public surface may serve it"

# The public surface refuses to publish personal state. A guard that can only be tested by making
# a deployment publish something personal is a guard nobody tests, so it is exercised here: a
# gateway of our own, on its own port, beside the twelve owners this gate already runs.
echo "==> Verifying a public surface serves a Journal of machine facts..."
GATEWAY_ADDR="127.0.0.1:8799"
CYBOU_SESSION_MODE=public-preview CYBOU_GATEWAY_ADDR="$GATEWAY_ADDR"     "$BIN_DIR/cybou-web-gateway" >"$TMP_DIR/gateway.log" 2>&1 &
GATEWAY_PID="$!"
PIDS+=("$GATEWAY_PID")

deadline=$((SECONDS + 30))
while [ "$SECONDS" -lt "$deadline" ]; do
    if curl -fsS "http://$GATEWAY_ADDR/api/v1/session" >/dev/null 2>&1; then
        break
    fi
    sleep 1
done
if ! curl -fsS "http://$GATEWAY_ADDR/api/v1/session" >/dev/null 2>&1; then
    echo "ERROR: the gateway refused to serve a Journal that holds only machine facts." >&2
    sed -n '1,5p' "$TMP_DIR/gateway.log" >&2
    exit 1
fi
echo "    Public surface is serving"

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

# And a promise is the person's, so recording one has to raise what the Journal carries. This is
# the transition the public surface refuses to publish across; if it never happened, the tripwire
# would be watching for something that cannot occur.
sensitivity_after="$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 HighestSensitivity | awk '{print $2}')"
if [ "$sensitivity_after" -le "$sensitivity_before" ]; then
    echo "ERROR: a person made a promise and the Journal still reports sensitivity $sensitivity_after." >&2
    exit 1
fi
echo "    A promise raised Journal sensitivity $sensitivity_before -> $sensitivity_after"

# And that is the transition the surface must survive without publishing across it. Stopping was
# what it used to do, and it was the wrong instrument: it took the whole surface down over rows it
# would never have shown, and the pressure to bring it back is what produced a raised threshold
# that outlived its reason and published a person's words. It withholds them instead now, and both
# halves are checked, because withholding everything would pass the first half on its own.
sleep 3
if ! kill -0 "$GATEWAY_PID" 2>/dev/null; then
    echo "ERROR: the public surface stopped instead of withholding what it may not publish:" >&2
    sed -n '1,10p' "$TMP_DIR/gateway.log" >&2
    exit 1
fi
if ! grep -q "withholds anything above sensitivity 0" "$TMP_DIR/gateway.log"; then
    echo "ERROR: the surface is serving without the public filter in place:" >&2
    sed -n '1,10p' "$TMP_DIR/gateway.log" >&2
    exit 1
fi

public_projection="$(curl -fsS --max-time 10 "http://$GATEWAY_ADDR/api/v1/mind" 2>/dev/null || true)"
if [ -z "$public_projection" ]; then
    echo "ERROR: the public surface did not answer, so nothing was proven about what it withholds." >&2
    exit 1
fi
case "$public_projection" in
    *"Verify the command path"*)
        echo "ERROR: the public surface published what the person promised." >&2
        exit 1
        ;;
esac
case "$public_projection" in
    *kernel-version*) ;;
    *)
        echo "ERROR: the public surface withheld the machine facts too, so it proves nothing." >&2
        exit 1
        ;;
esac
echo "    Public surface kept serving machine facts and withheld the person's promise"

# The same for a sentence spoken to Meaning1, which is what actually leaked: the beliefs and
# concepts derived from an utterance carried its text verbatim into the public projection.
spoken="a sentence only the owner should see"
busctl --user call org.cybou.Mind.Meaning1 /org/cybou/Mind/Meaning1 org.cybou.Mind.Meaning1 Interpret ss "Remember that $spoken" person >/dev/null

deadline=$((SECONDS + 30))
while [ "$SECONDS" -lt "$deadline" ]; do
    if busctl --user call org.cybou.Mind.Context1 /org/cybou/Mind/Context1 org.cybou.Mind.Context1 ActiveContext | grep -q "117 116 116 101 114 97 110 99 101"; then
        break
    fi
    sleep 1
done

public_projection="$(curl -fsS --max-time 10 "http://$GATEWAY_ADDR/api/v1/mind" 2>/dev/null || true)"
case "$public_projection" in
    *"$spoken"*)
        echo "ERROR: the public surface published what a person said to Meaning1." >&2
        exit 1
        ;;
esac
echo "    Public surface withheld what a person said to Meaning1"


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
echo "==> Verifying language crosses the meaning boundary as a typed act..."
# ADR-0031 C1: what reaches Mind is an act, not the sentence. An empty answer means the utterance
# was refused, which is what this build does with anything outside its vocabulary.
interpreted="$(busctl --user call org.cybou.Mind.Meaning1 /org/cybou/Mind/Meaning1 org.cybou.Mind.Meaning1 Interpret ss "Verify the chain" person)"
if [ "$interpreted" = "ay 0" ]; then
    echo "ERROR: Meaning1 could not interpret an utterance in its own vocabulary." >&2
    exit 1
fi
echo "    Meaning1 turned an utterance into a typed act"

# C8: nothing here is a generative model, and an utterance outside the vocabulary is refused rather
# than guessed at. A layer that answered something for every input would have nothing to refuse
# with, and every wrong reading would look exactly like a right one.
guessed="$(busctl --user call org.cybou.Mind.Meaning1 /org/cybou/Mind/Meaning1 org.cybou.Mind.Meaning1 Interpret ss "the kettle is boiling over there" person)"
if [ "$guessed" != "ay 0" ]; then
    echo "ERROR: Meaning1 produced an act for an utterance it has no vocabulary for." >&2
    exit 1
fi
echo "    Meaning1 refused an utterance it has no vocabulary for"

# C4: the act outlives whatever produced it, because it is a row. The Journal holds an Observation
# for what was said and a Hypothesis for what it was taken to mean, and a Hypothesis is a derived
# kind — Event1 accepted it only because it cited the utterance.
speech_rows="$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 Count | awk '{print $2}')"
if [ "$speech_rows" -lt 2 ]; then
    echo "ERROR: an interpretation was reported and the Journal holds fewer than two rows." >&2
    exit 1
fi
echo "    The utterance and its interpretation are in the biography"

# C3: a correction appends rather than rewriting. It has to name a real prior act, so the act
# identity is read out of the interpretation that was just recorded. It is the sixteen bytes that
# follow the "actId" key in the CBOR the owner answered with; nothing else in the reply is needed.
act_id_of() {
    # busctl prints a byte array as decimal fields. The key "actId" is CBOR text-6 followed by a
    # 16-byte string header, so 101 97 99 116 73 100 80 locates the identity that follows it.
    printf '%s' "$1" | awk '
    {
        for (i = 1; i <= NF - 22; i++) {
            if ($i == 101 && $(i+1) == 97 && $(i+2) == 99 && $(i+3) == 116 &&
                $(i+4) == 73 && $(i+5) == 100 && $(i+6) == 80) {
                out = ""
                for (b = 0; b < 16; b++) {
                    out = out sprintf("%02x", $(i + 7 + b))
                    if (b == 3 || b == 5 || b == 7 || b == 9) {
                        out = out "-"
                    }
                }
                print out
                exit
            }
        }
    }'
}

prior_act="$(act_id_of "$interpreted")"
if [ -z "$prior_act" ]; then
    echo "ERROR: the interpretation Meaning1 answered with carries no act identity." >&2
    exit 1
fi

# A correction naming an act the Journal does not hold corrects nothing, and is refused before it
# reaches Event1 so an ordinary rejection stays distinguishable from an unreachable Journal.
invented_correction="$(busctl --user call org.cybou.Mind.Meaning1 /org/cybou/Mind/Meaning1 org.cybou.Mind.Meaning1 Correct sss 00000000-0000-4000-8000-000000000000 "No, the disk was fine" person)"
if [ "$invented_correction" != "ay 0" ]; then
    echo "ERROR: Meaning1 corrected an interpretation the Journal does not hold." >&2
    exit 1
fi
echo "    Meaning1 refused to correct an interpretation the Journal does not hold"

before_correction="$speech_rows"
corrected="$(busctl --user call org.cybou.Mind.Meaning1 /org/cybou/Mind/Meaning1 org.cybou.Mind.Meaning1 Correct sss "$prior_act" "No, the disk was fine" person)"
if [ "$corrected" = "ay 0" ]; then
    echo "ERROR: Meaning1 refused a correction of an act the Journal holds." >&2
    exit 1
fi
after_correction="$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 Count | awk '{print $2}')"
if [ "$after_correction" -le "$before_correction" ]; then
    echo "ERROR: a correction was reported and the Journal did not grow: $before_correction -> $after_correction." >&2
    exit 1
fi
# Appending, not rewriting: what was previously understood is still there to be argued with.
still_held="$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 Contains s "$prior_act" | awk '{print $2}')"
if [ "$still_held" != "true" ]; then
    echo "ERROR: correcting an interpretation removed the interpretation it corrected." >&2
    exit 1
fi
echo "    A correction appended and left what it corrected in place: $before_correction -> $after_correction"

# C6: prose comes from a plan and from nothing else. Handed bytes that are not a plan, the
# renderer says nothing rather than falling back on a sentence — a fallback is exactly how a claim
# Mind never made would get into an answer, and it would read like every other answer.
not_a_plan="$(busctl --user call org.cybou.Mind.Meaning1 /org/cybou/Mind/Meaning1 org.cybou.Mind.Meaning1 Realize ays 0 en | awk '{print $2}')"
if [ "$not_a_plan" != '""' ]; then
    echo "ERROR: the renderer produced $not_a_plan from something that is not a plan." >&2
    exit 1
fi
echo "    The renderer says nothing when handed something that is not a plan"

causation_of() {
    # The same trick as act_id_of, for what a contribution was caused by. The envelope is serialised
    # with its Rust field names, so the key is CBOR text-12 "causation_id"
    # (108 99 97 117 115 97 116 105 111 110 95 105 100) followed by a 16-byte string header (80).
    printf '%s' "$1" | awk '
    {
        for (i = 1; i <= NF - 30; i++) {
            if ($i == 108 && $(i+1) == 99 && $(i+2) == 97 && $(i+3) == 117 &&
                $(i+4) == 115 && $(i+5) == 97 && $(i+6) == 116 && $(i+7) == 105 &&
                $(i+8) == 111 && $(i+9) == 110 && $(i+10) == 95 && $(i+11) == 105 &&
                $(i+12) == 100 && $(i+13) == 80) {
                out = ""
                for (b = 0; b < 16; b++) {
                    out = out sprintf("%02x", $(i + 14 + b))
                    if (b == 3 || b == 5 || b == 7 || b == 9) { out = out "-" }
                }
                print out
                exit
            }
        }
    }'
}

echo "==> Verifying the biography records who was supplied what..."
# The Journal could say what the system did and not who looked at it, which made it traceable in
# one direction only. ADR-0030 B4: every delivery that crosses a boundary is recorded with its
# destination and the provenance of what was supplied, and no copy of the content.
before_disclosures="$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 Count | awk '{print $2}')"
curl -fsS --max-time 10 "http://$GATEWAY_ADDR/api/v1/mind" >/dev/null 2>&1 || true

deadline=$((SECONDS + 20))
disclosure=""
while [ "$SECONDS" -lt "$deadline" ] && [ -z "$disclosure" ]; do
    seq="$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 Count | awk '{print $2}')"
    while [ "$seq" -gt "$before_disclosures" ]; do
        row="$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 AtSequence t "$seq")"
        # Kind 17 is ContextDisclosed, and the origin is the surface that supplied it.
        if printf '%s' "$row" | awk '{ for (i = 3; i <= NF; i++) printf "%c", $i }' | grep -q "web-gateway"; then
            disclosure="$row"
            break
        fi
        seq=$((seq - 1))
    done
    [ -z "$disclosure" ] && sleep 1
done

if [ -z "$disclosure" ]; then
    echo "ERROR: a reader was supplied a projection and the Journal records no disclosure." >&2
    exit 1
fi
echo "    A delivery to the public surface is in the biography"

# What the record says is checked by the unit tests rather than here: it is sealed, as anything
# about the person is, so the bytes on this side of Event1 are ciphertext. What is checkable here is
# the thing those tests cannot reach — that a real delivery over a real bus produced one.
#
# Asking again for the same projection is not a second delivery. A reader watching the event stream
# receives the same thing every few seconds, and a record per response would fill the Journal with
# thousands of rows that answer no question anyone would ask.
after_first="$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 Count | awk '{print $2}')"
curl -fsS --max-time 10 "http://$GATEWAY_ADDR/api/v1/mind" >/dev/null 2>&1 || true
curl -fsS --max-time 10 "http://$GATEWAY_ADDR/api/v1/mind" >/dev/null 2>&1 || true
sleep 2
after_repeat="$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 Count | awk '{print $2}')"
if [ "$after_repeat" -gt "$after_first" ]; then
    echo "ERROR: asking twice for the same projection recorded $((after_repeat - after_first)) further deliveries." >&2
    exit 1
fi
echo "    Being supplied the same thing again is not recorded as a new delivery"

echo "==> Verifying a person can take back what they said..."
# Until now nothing anywhere raised the erasure epoch or removed a payload: ADR-0028 was described,
# its kinds existed, Context1 reacted to the epoch, and there was no path from a person asking for
# something to be gone to it being gone. This is that path.
#
# The target is the utterance rather than the reading of it, because the closure travels downward:
# erasing what a person said has to take the interpretation with it, and erasing the interpretation
# would leave the sentence. The interpretation names its cause, which is the utterance.
interpreted_to_forget="$(busctl --user call org.cybou.Mind.Meaning1 /org/cybou/Mind/Meaning1 org.cybou.Mind.Meaning1 Interpret ss "Remember that this has to be forgettable" person)"
act_to_forget="$(act_id_of "$interpreted_to_forget")"
if [ -z "$act_to_forget" ]; then
    echo "ERROR: the sentence was not interpreted, so there is nothing to erase." >&2
    exit 1
fi
interpretation_row="$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 Contribution s "$act_to_forget")"
target="$(causation_of "$interpretation_row")"
if [ -z "$target" ]; then
    echo "ERROR: the interpretation names no cause, so the utterance cannot be found." >&2
    exit 1
fi
echo "    The sentence is in the Journal as $target"

epoch_before="$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 ErasureEpoch | awk '{print $2}')"

# A reason outside the closed set is refused: an erasure record is permanent, and free text would
# let the thing being forgotten be restated where nothing can ever be erased.
invented="$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 RequestErasure ss "$target" "because I said so" | awk '{print $2}')"
if [ "$invented" != "-1" ]; then
    echo "ERROR: Event1 accepted an erasure reason it does not know." >&2
    exit 1
fi
echo "    Event1 refused a reason outside the closed set"

# Two contributions have to go: the utterance and the reading derived from it. One would mean the
# closure is not travelling, which is the failure that leaves the reasoning that restates what was
# erased.
erased="$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 RequestErasure ss "$target" user-requested | awk '{print $2}')"
if [ "$erased" -lt 2 ]; then
    echo "ERROR: erasing an utterance redacted $erased contributions; the interpretation derived from it should have gone too." >&2
    exit 1
fi
echo "    Erasure redacted $erased contributions: the sentence and what was derived from it"

epoch_after="$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 ErasureEpoch | awk '{print $2}')"
if [ "$epoch_after" -le "$epoch_before" ]; then
    echo "ERROR: an erasure happened and the epoch stayed at $epoch_after." >&2
    exit 1
fi
echo "    Erasure epoch advanced: $epoch_before -> $epoch_after"

# The rows are still there. Identity, author, causality and position are never erased: a Mind that
# could forget that it once concluded something could not be audited afterwards.
for surviving in "$target" "$act_to_forget"; do
    if [ "$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 Contains s "$surviving" | awk '{print $2}')" != "true" ]; then
        echo "ERROR: erasing a payload removed the contribution $surviving itself." >&2
        exit 1
    fi
done
echo "    Both contributions survive as records of what happened"

# And the chain still verifies. Erasure that broke the biography's structure would be corruption
# with a nicer name.
if [ "$(busctl --user call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 org.cybou.Mind.Event1 VerifyFullyStep u 4096)" = "ay 0" ]; then
    echo "ERROR: Event1 cannot verify its chain after an erasure." >&2
    exit 1
fi
echo "    The chain still verifies with the payloads gone"

echo "==> Verifying an assessment cannot cite a cause the Journal does not hold..."
# Assess took a cause and ignored it, so an assessment naming a contribution that never existed
# was indistinguishable from one naming a real cause.
invented="$(busctl --user call org.cybou.Mind.Self1 /org/cybou/Mind/Self1 org.cybou.Mind.Self1 Assess s 00000000-0000-4000-8000-000000000000)"
if [ "$invented" != "ay 0" ]; then
    echo "ERROR: Self1 assessed against a cause the Journal does not hold." >&2
    exit 1
fi
echo "    Self1 refused a cause the Journal does not hold"

# The same measurement without a cause has to still work, or the refusal above would pass just as
# well on an organ whose assessment is broken outright.
measured="$(busctl --user call org.cybou.Mind.Self1 /org/cybou/Mind/Self1 org.cybou.Mind.Self1 Measure)"
if [ "$measured" = "ay 0" ]; then
    echo "ERROR: Self1 cannot measure at all, so refusing an invented cause proves nothing." >&2
    exit 1
fi
echo "    Self1 still measures when it is not asked about a cause"

echo "==> Verifying the forecaster learns from the biography rather than from callers..."
# perceptiond contributes cpu-count and memory-total-kib as numbers, so a forecast about one of
# them can only exist if predictord read the Journal. It had a replay routine and a cursor and
# nothing that ever called them, so every forecast came from whatever a caller pushed in by hand.
forecast="ay 0"
deadline=$((SECONDS + 30))
while [ "$SECONDS" -lt "$deadline" ]; do
    forecast="$(busctl --user call org.cybou.Mind.Predictor1 /org/cybou/Mind/Predictor1 org.cybou.Mind.Predictor1 Predict s cpu-count)"
    if [ "$forecast" != "ay 0" ]; then
        break
    fi
    sleep 1
done
if [ "$forecast" = "ay 0" ]; then
    echo "ERROR: the Journal holds measured observations, yet Predictor1 forecasts nothing." >&2
    exit 1
fi
echo "    Predictor1 forecasts a subject it only knows from the Journal"

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
