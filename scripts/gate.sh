#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Every check that has to pass before a commit is deployed, in one place that cannot report a pass
# it did not get.
#
# This exists because the ad-hoc form of it lied twice in one afternoon. `cmd | grep x | head -1`
# takes its exit status from `head`, which succeeds whether or not grep matched, so a build failure
# printed nothing and the `&&` chain continued to the line that says everything passed. Separately,
# a `;` where an `&&` belonged printed "clippy ok" immediately after clippy had failed.
#
# Both are the same defect as the ones this repository keeps finding in itself: a check whose
# failure is invisible is not a check. So:
#
#   - `set -o pipefail`, so a failure anywhere in a pipe is a failure.
#   - no filtering of the output of anything whose exit status matters.
#   - the summary is printed from a trap, so it cannot be reached by skipping the failure.
#
# The wasm target matters more here than anywhere. `crates/living-canvas/src/components` is compiled
# only for `wasm32`, so `cargo check --workspace` says nothing about it, and a broken card compiles
# clean on a native run.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"
export CHROMEDRIVER="${CHROMEDRIVER:-/usr/bin/chromedriver}"

failed=""
skipped=""

announce() {
    printf '\n==> %s\n' "$1"
}

report() {
    if [ -n "$failed" ]; then
        printf '\n=== GATE FAILED: %s ===\n' "$failed"
        return
    fi
    if [ -n "$skipped" ]; then
        # Not "every gate passed". A check that did not run is not a check that passed, and a
        # summary that says otherwise is the exact failure this script exists to remove — one step
        # further along than the pipeline that reported success after a hidden error.
        printf '\n=== every gate that ran passed; NOT RUN:%s ===\n' "$skipped"
        return
    fi
    printf '\n=== every gate passed ===\n'
}
trap report EXIT

step() {
    local name="$1"
    shift
    announce "$name"
    # The name is recorded before the command runs, so an interrupted or crashed step is reported as
    # the step it was rather than as a pass.
    failed="$name"
    "$@"
    failed=""
}

step "formatting"            cargo fmt --all -- --check
step "clippy"                cargo clippy --workspace --all-targets --locked -- -D warnings
step "native tests"          cargo test --workspace --locked
step "browser tests"         cargo test -p living-canvas --target wasm32-unknown-unknown --locked
step "cognitive documents"   python3 scripts/validate-cognitive-docs.py .
step "card messages"      python3 scripts/validate-card-signals.py
step "desktop styles"        python3 scripts/validate-desktop-styles.py
step "organ layering"        python3 scripts/validate-organ-layering.py
step "document links"        python3 scripts/validate-doc-links.py
step "site wording"          python3 scripts/sync-site-i18n.py --check
step "multi-daemon organs"   bash scripts/test-multi-daemon-integration.sh
step "ACP client boundary"   bash scripts/test-acp-gate.sh
step "standing lease mint"   bash scripts/test-standing-lease-gate.sh
step "agent model gateway"   bash scripts/test-model-gateway-gate.sh
step "LiteLLM worker"        bash scripts/test-litellm-worker-gate.sh
step "provider catalogue"    bash scripts/test-provider-catalogue-gate.sh
step "OpenCode agent pack"   bash scripts/test-opencode-pack-gate.sh
step "agent session owner"   bash scripts/test-agent-session-gate.sh

# A whole prompt turn against a stand-in agent. Exit 3 means there was no python3 here to run one
# with, which is a check that did not run rather than one that passed.
announce "ACP prompt turn"
failed="ACP prompt turn"
acp_status=0
bash scripts/test-acp-session-gate.sh || acp_status=$?
case "$acp_status" in
    0) failed="" ;;
    3)
        announce "ACP prompt turn not run: no python3 to run a stand-in agent with"
        skipped="$skipped acp-prompt-turn"
        failed=""
        ;;
    *) exit "$acp_status" ;;
esac

# What a browser is told about running agents, and by whom. Exit 3 means this host cannot run an
# owner and a gateway together, which is a check that did not run.
announce "agent card"
failed="agent card"
card_status=0
bash scripts/test-agent-card-gate.sh || card_status=$?
case "$card_status" in
    0) failed="" ;;
    3)
        announce "agent card not run: no session to run an owner and a gateway in"
        skipped="$skipped agent-card"
        failed=""
        ;;
    *) exit "$card_status" ;;
esac

# The profile door, against a catalogue on disk. Exit 3 means the catalogue cannot be placed where
# this build reads it, which is a check that did not run.
announce "agent profiles"
failed="agent profiles"
profile_status=0
bash scripts/test-agent-profile-gate.sh || profile_status=$?
case "$profile_status" in
    0) failed="" ;;
    3)
        announce "agent profiles not run: the approved catalogue cannot be placed here"
        skipped="$skipped agent-profiles"
        failed=""
        ;;
    *) exit "$profile_status" ;;
esac

# A host that repairs itself with nobody driving it. Exit 3 means there is no root systemd here to
# run four daemons against, which is a check that did not run rather than one that passed.
announce "self-maintenance"
failed="self-maintenance"
self_status=0
bash scripts/test-self-maintenance-gate.sh || self_status=$?
case "$self_status" in
    0) failed="" ;;
    3)
        announce "self-maintenance not run: no root systemd to run the organs against"
        skipped="$skipped self-maintenance"
        failed=""
        ;;
    *) exit "$self_status" ;;
esac

# Action1 writing its lifecycle to a real Event1 and reading it back after a restart. Exit 3 means
# there is no session bus to run two daemons on, which is a check that did not run.
announce "action durability"
failed="action durability"
durability_status=0
bash scripts/test-action-durability-gate.sh || durability_status=$?
case "$durability_status" in
    0) failed="" ;;
    3)
        announce "action durability not run: no session bus to run Event1 and Action1 on"
        skipped="$skipped action-durability"
        failed=""
        ;;
    *) exit "$durability_status" ;;
esac

# A whole launch with no model in it, which needs no provider and no gateway. Exit 3 means the
# capsule's host programs are not installed here, which is a check that did not run.
announce "capsule launch"
failed="capsule launch"
capsule_launch_status=0
bash scripts/test-capsule-launch-gate.sh || capsule_launch_status=$?
case "$capsule_launch_status" in
    0) failed="" ;;
    3)
        announce "capsule launch not run: the capsule's host programs are not installed here"
        skipped="$skipped capsule-launch"
        failed=""
        ;;
    *) exit "$capsule_launch_status" ;;
esac

# What a root service manager will hand an unprivileged service. Exit 3 means there is no system
# manager here to ask, which is a check that did not run rather than one that passed.
announce "credential boundary"
failed="credential boundary"
credential_status=0
bash scripts/test-credential-boundary-gate.sh || credential_status=$?
case "$credential_status" in
    0) failed="" ;;
    3)
        announce "credential boundary not run: no root system manager to ask for a credential"
        skipped="$skipped credential-boundary"
        failed=""
        ;;
    *) exit "$credential_status" ;;
esac

# A real daemon, a real bus name, a real service manager. Exit 3 means this host has no user session
# to run one against, which is a check that did not run rather than one that passed.
announce "agent runtime ownership"
failed="agent runtime ownership"
runtime_status=0
bash scripts/test-agent-runtime-gate.sh || runtime_status=$?
case "$runtime_status" in
    0) failed="" ;;
    3)
        announce "agent runtime ownership not run: no user session to hold a bus name in"
        skipped="$skipped agent-runtime-ownership"
        failed=""
        ;;
    *) exit "$runtime_status" ;;
esac

# Freeze, Resume, Quarantine and Stop against a real cgroup and both outbound runtime units.
# Exit 3 means the deployed gateway/provider/polkit boundary is absent, never that the proof passed.
announce "agent physical controls"
failed="agent physical controls"
control_status=0
bash scripts/test-agent-control-gate.sh || control_status=$?
case "$control_status" in
    0) failed="" ;;
    3)
        announce "agent physical controls not run: this host has no complete deployed session boundary"
        skipped="$skipped agent-physical-controls"
        failed=""
        ;;
    *) exit "$control_status" ;;
esac

# A live Agent1 record must become durable Operation1 state, while the gateway stays a disposable
# projection. Exit 3 means this machine has no isolated session boundary on which to run it.
announce "operation continuity"
failed="operation continuity"
operation_status=0
bash scripts/test-operation-continuity-gate.sh || operation_status=$?
case "$operation_status" in
    0) failed="" ;;
    3)
        announce "operation continuity not run: this host has no free deployed session boundary"
        skipped="$skipped operation-continuity"
        failed=""
        ;;
    *) exit "$operation_status" ;;
esac

# Each Linux account's personal records answer from that account's own owner, and from no other.
# Exit 3 means this host cannot run the owner unprivileged.
announce "personal owner isolation"
failed="personal owner isolation"
personal_status=0
bash scripts/test-personal-owner-gate.sh || personal_status=$?
case "$personal_status" in
    0) failed="" ;;
    3)
        announce "personal owner isolation not run: this host cannot run an unprivileged owner"
        skipped="$skipped personal-owner-isolation"
        failed=""
        ;;
    *) exit "$personal_status" ;;
esac

# One launch, carried out on a real host, leaving nothing behind. Exit 3 means this host has no
# deployed gateway template, provider or user service manager to launch against — a check that did
# not run rather than one that passed.
announce "agent launch teardown"
failed="agent launch teardown"
launch_status=0
bash scripts/test-agent-launch-gate.sh || launch_status=$?
case "$launch_status" in
    0) failed="" ;;
    3)
        announce "agent launch teardown not run: this host has no deployed agent session to launch"
        skipped="$skipped agent-launch-teardown"
        failed=""
        ;;
    *) exit "$launch_status" ;;
esac

# ADR-0042 G1. Exit 3 means bubblewrap is absent, which is a check that did not run rather than one
# that passed — the distinction this whole script exists to keep.
announce "capsule escape attempts"
failed="capsule escape attempts"
capsule_status=0
bash scripts/test-capsule-gate.sh || capsule_status=$?
case "$capsule_status" in
    0) failed="" ;;
    3)
        announce "capsule escape attempts not run: bubblewrap is not installed here"
        skipped="$skipped capsule-escape-attempts"
        failed=""
        ;;
    *) exit "$capsule_status" ;;
esac

# ADR-0042 step ten. Exit 3 means there was nothing here to speak to a Unix socket with, which is a
# check that did not run rather than one that passed.
announce "egress broker refusals"
failed="egress broker refusals"
egress_status=0
bash scripts/test-egress-gate.sh || egress_status=$?
case "$egress_status" in
    0) failed="" ;;
    3)
        announce "egress broker refusals not run: see the note above"
        skipped="$skipped egress-broker-refusals"
        failed=""
        ;;
    *) exit "$egress_status" ;;
esac

# ADR-0022 A1. The gate uses a disposable systemd service, never a real workload. Exit 3 keeps an
# environment without a root system manager visibly distinct from one in which the boundary held.
announce "authorized action boundary"
failed="authorized action boundary"
action_status=0
bash scripts/test-action-gate.sh || action_status=$?
case "$action_status" in
    0) failed="" ;;
    3)
        announce "authorized action boundary not run: a root systemd host is unavailable"
        skipped="$skipped authorized-action-boundary"
        failed=""
        ;;
    *) exit "$action_status" ;;
esac

# ADR-0022, the other half. The gate above proves the path a standing policy opens; this one runs
# the host in the state every installation is in until somebody changes it, where a proposal stops
# at a question and a person's answer is what carries it the rest of the way.
announce "confirmed action boundary"
failed="confirmed action boundary"
confirmation_status=0
bash scripts/test-confirmation-gate.sh || confirmation_status=$?
case "$confirmation_status" in
    0) failed="" ;;
    3)
        announce "confirmed action boundary not run: a root systemd host, dbus-run-session and sqlite3 are required"
        skipped="$skipped confirmed-action-boundary"
        failed=""
        ;;
    *) exit "$confirmation_status" ;;
esac

# ADR-0048, the other entrance. The two gates above start from something this host concluded about
# itself; this one starts from a person, and there is no question to answer because the asking is
# the confirmation.
announce "requested action boundary"
failed="requested action boundary"
request_status=0
bash scripts/test-request-gate.sh || request_status=$?
case "$request_status" in
    0) failed="" ;;
    3)
        announce "requested action boundary not run: a root systemd host, dbus-run-session and sqlite3 are required"
        skipped="$skipped requested-action-boundary"
        failed=""
        ;;
    *) exit "$request_status" ;;
esac

# Licensing headers, because CI runs this and a gate that claims to be every check and is not is the
# same defect as a check whose failure is invisible. Skipped with a said reason rather than silently
# when the tool is absent: an absent check must not look like a passed one.
if command -v reuse > /dev/null 2>&1; then
    step "licence headers"      reuse lint
else
    announce "licence headers not run: reuse is not installed here, and CI runs it"
    skipped="$skipped licence-headers"
fi
