#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

# ADR-0043 B5. Exercise the real HTTP adapter against a fake proxy: the master key may reach only
# key administration, every completion gets a short-lived model/budget/concurrency-scoped key, and
# proxy cost plus deployment/call attribution must survive the provider-neutral worker boundary.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

cargo test -p cybou-provider-litellm --locked

if cargo tree -p cybou-model-gateway --locked --prefix none | grep -q '^cybou-provider-litellm '; then
    echo "cybou-model-gateway depends on its replaceable LiteLLM implementation" >&2
    exit 1
fi
