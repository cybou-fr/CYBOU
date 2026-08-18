#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Build on Debian 13, then install the current Rust gateway and shared WASM frontend.
set -euo pipefail

# shellcheck source=scripts/vps-env.sh
. "$(dirname "$0")/vps-env.sh"

cybou_push_source

cybou_ssh "
  set -eu
  . \"\$HOME/.cargo/env\"
  cd '$CYBOU_VPS_SRC'
  cargo test --workspace --locked
  cargo build --workspace --release --locked
  cd crates/living-canvas
  trunk build --release
  cd ../..

  sudo install -d -m 0755 /usr/libexec/cybou /usr/share/cybou/web
  sudo install -m 0755 target/release/cybou-web-gateway /usr/libexec/cybou/cybou-web-gateway
  sudo cp -a target/living-canvas/. /usr/share/cybou/web/
  sudo chown -R root:root /usr/libexec/cybou /usr/share/cybou
"

echo "==> Debian artifacts installed on $CYBOU_VPS_HOST"
