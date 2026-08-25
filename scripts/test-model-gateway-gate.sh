#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

# ADR-0043 B4. The compatibility surface is tested without a real provider: one fake registered
# worker must serve both request shapes, while authentication, lease lifetime and budgets remain
# real. Provider breadth belongs to B5 and cannot be smuggled into this gate.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

cargo test -p cybou-model-brokerd -p cybou-model-gateway --locked
