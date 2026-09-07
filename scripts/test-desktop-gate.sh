#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# CYBOU Desktop Reliability & Architectural Grounding Gate.
#
# Proves:
# 1. Desktop layout normalization, migration and corrupted state recovery.
# 2. Invariant-safe Deck composition and dissolution.
# 3. Web Gateway security & session boundaries.
# 4. WASM compilation of the Living Canvas frontend.
#
# This ran `cargo test -p cybou-shelld` until the sandboxed Safe Shell was removed, which left a
# gate that could not pass and that nothing invoked. TESTING.md had already been corrected; the
# script had not, and a check whose failure nobody ever sees is the defect this repository keeps
# finding in itself. What replaced that stage is a real PTY owned by the authenticated account,
# proven by scripts/test-terminal-gate.sh on a deployed host — a terminal that runs programs as a
# person is not something a unit test can stand in for.
#
# Every stage below is also a step of scripts/gate.sh, which is what CI runs. This script is the
# standalone form, for working on the desktop without paying for the whole gate; it is deliberately
# not wired into gate.sh, because running it there would be running the same four things twice.
#
# What this does NOT prove: that the browser half is lint-clean. `crates/living-canvas/src/components`
# compiles only for wasm32, so the workspace Clippy step never sees it, and stage 4 is a compile
# rather than a lint. See "What is not covered" in docs/TESTING.md.

set -euo pipefail

echo "==> [Gate 1/4] Running Desktop & Living Canvas unit tests..."
cargo test -p living-canvas --locked

echo "==> [Gate 2/4] Running Web Gateway security & session tests..."
cargo test -p cybou-web-gateway --locked

echo "==> [Gate 3/4] Verifying WASM32 target compilation..."
cargo check -p living-canvas --target wasm32-unknown-unknown --locked

echo "==> [Gate 4/4] Verifying Clippy warnings on entire workspace..."
cargo clippy --workspace --all-targets --locked -- -D warnings

echo "==> ALL 4 CYBOU Desktop Reliability Gates PASSED successfully!"
