#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# CYBOU Desktop Reliability & Architectural Grounding Gate.
#
# Proves:
# 1. Desktop layout normalization & corrupted state recovery.
# 2. Invariant-safe Deck composition and dissolution.
# 3. WASM compilation of Living Canvas frontend.
# 4. DemoReadOnly Shell capability confinement (pwd, cd, ls, cat, help, clear).
# 5. Public Preview session boundaries and lock gating.

set -euo pipefail

echo "==> [Gate 1/5] Running Desktop & Living Canvas unit tests..."
cargo test -p living-canvas --locked

echo "==> [Gate 2/5] Running CYBOU Shelld confinement tests..."
cargo test -p cybou-shelld --locked

echo "==> [Gate 3/5] Running Web Gateway security & session tests..."
cargo test -p cybou-web-gateway --locked

echo "==> [Gate 4/5] Verifying WASM32 target compilation..."
cargo check -p living-canvas --target wasm32-unknown-unknown --locked

echo "==> [Gate 5/5] Verifying Clippy warnings on entire workspace..."
cargo clippy --workspace --all-targets --locked -- -D warnings

echo "==> ALL 5 CYBOU Desktop Reliability Gates PASSED successfully!"
