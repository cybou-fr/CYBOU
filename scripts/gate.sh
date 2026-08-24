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
step "desktop styles"        python3 scripts/validate-desktop-styles.py
step "organ layering"        python3 scripts/validate-organ-layering.py
step "document links"        python3 scripts/validate-doc-links.py
step "multi-daemon organs"   bash scripts/test-multi-daemon-integration.sh

# Licensing headers, because CI runs this and a gate that claims to be every check and is not is the
# same defect as a check whose failure is invisible. Skipped with a said reason rather than silently
# when the tool is absent: an absent check must not look like a passed one.
if command -v reuse > /dev/null 2>&1; then
    step "licence headers"      reuse lint
else
    announce "licence headers not run: reuse is not installed here, and CI runs it"
    skipped="$skipped licence-headers"
fi
