#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

# ADR-0043 B6 / N6-N7. Provider facts arrive only as external, expiring, source-backed data. The
# compiled default knows none; a stale claim cannot route; a fallback must be named in operator
# order and remains distinguishable from the preferred provider.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

cargo test -p cybou-provider-catalogue --locked

if cargo tree -p cybou-provider-catalogue --locked --prefix none \
    | grep -Eq '^cybou-(model-brokerd|model-gateway|provider-litellm) ';
then
    echo "provider catalogue depends on a routing or provider implementation" >&2
    exit 1
fi
