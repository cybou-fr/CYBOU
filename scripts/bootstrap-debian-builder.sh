#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# One-time bootstrap for the active Debian 13 builder. Run locally; all mutations are remote.
set -euo pipefail

# shellcheck source=scripts/vps-env.sh
. "$(dirname "$0")/vps-env.sh"

cybou_ssh '
  set -eu
  . /etc/os-release
  [ "$ID" = debian ] && [ "$VERSION_ID" = 13 ] || {
    echo "refusing: Debian 13 is required" >&2
    exit 2
  }

  sudo apt-get update
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    build-essential ca-certificates clang curl dbus-user-session git lld \
    libdbus-1-dev libsodium-dev libsqlite3-dev pkg-config sqlite3

  if ! command -v rustup >/dev/null 2>&1; then
    curl --proto =https --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --profile minimal --default-toolchain 1.95.0
  fi
  . "$HOME/.cargo/env"
  rustup toolchain install 1.95.0 --profile minimal --component clippy,rustfmt \
    --target wasm32-unknown-unknown
  rustup default 1.95.0
  cargo install trunk --version 0.21.14 --locked

  rustc --version
  cargo --version
  trunk --version
'

echo "==> Debian 13 builder ready: $CYBOU_VPS_HOST"
