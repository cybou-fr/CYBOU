#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# B2: the client speaks stable ACP to a process, and refuses incompatible negotiation.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

cargo build --quiet -p cybou-acp --bin cybou-acp --example fake-agent --example probe-agent

CLIENT="$CARGO_TARGET_DIR/debug/examples/probe-agent"
AGENT="$CARGO_TARGET_DIR/debug/examples/fake-agent"

echo "=== ACP v1 initialization ==="
handshake="$("$CLIENT" "$AGENT")"
grep -q '"protocolVersion": 1' <<<"$handshake"
grep -q '"name": "cybou-fake-agent"' <<<"$handshake"
grep -q '"id": "fake-login"' <<<"$handshake"
grep -q '"loadSession": true' <<<"$handshake"
echo "    ok      identity, auth methods and capabilities crossed ACP stdio"

if "$CLIENT" "$AGENT" --wrong-version >/dev/null 2>&1; then
    echo "ERROR: client accepted an unsupported ACP wire version" >&2
    exit 1
fi
echo "    ok      unsupported wire negotiation is refused"

echo "=== ACP gate passed ==="
