#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

# ADR-0042 B3. Exercise the public profile-selection boundary rather than an internal fixture: the
# selected profile must mint one standing lease, stay silent inside it, and fail closed before a
# malformed selection becomes authority.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/cybou-target}"

cargo test -p cybou-capsule --test standing_lease --locked
