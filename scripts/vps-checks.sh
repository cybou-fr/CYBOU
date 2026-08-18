#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Push the unfinished working tree to the active Debian 13 builder and run Rust/Web gates there.
set -euo pipefail

# shellcheck source=scripts/vps-env.sh
. "$(dirname "$0")/vps-env.sh"

profile="${1:-fast}"
case "$profile" in
  fast | release) ;;
  *) echo "usage: $0 [fast|release]" >&2; exit 2 ;;
esac

cybou_push_source

cybou_ssh "
  set -eu
  . \"\$HOME/.cargo/env\"
  cd '$CYBOU_VPS_SRC'
  export CARGO_TARGET_DIR='$CYBOU_VPS_TARGET'
  cargo fmt --all -- --check
  cargo test --workspace --locked
  cargo clippy --workspace --all-targets --locked -- -D warnings
  cargo check -p living-canvas --target wasm32-unknown-unknown --locked
  if [ '$profile' = release ]; then
    cargo build --workspace --release --locked
    cd crates/living-canvas
    trunk build --release
  fi
"

echo "==> Debian 13 $profile checks passed on $CYBOU_VPS_HOST"
