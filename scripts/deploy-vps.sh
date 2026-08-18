#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Build on Debian 13, install the Rust gateway/shared WASM frontend, and activate HTTPS.
set -euo pipefail

# shellcheck source=scripts/vps-env.sh
. "$(dirname "$0")/vps-env.sh"

cybou_push_source

cybou_ssh "
  set -eu
  . \"\$HOME/.cargo/env\"
  cd '$CYBOU_VPS_SRC'
  export CARGO_TARGET_DIR='$CYBOU_VPS_TARGET'
  cargo test --workspace --locked
  cargo build --workspace --release --locked
  cd crates/living-canvas
  trunk build --release
  cd ../..

  sudo install -d -m 0755 /usr/libexec/cybou /usr/share/cybou/web
  sudo install -m 0755 '$CYBOU_VPS_TARGET/release/cybou-web-gateway' /usr/libexec/cybou/cybou-web-gateway
  sudo cp -a target/living-canvas/. /usr/share/cybou/web/
  sudo chown -R root:root /usr/libexec/cybou /usr/share/cybou

  sudo getent group cybou >/dev/null || sudo groupadd --system cybou
  sudo id cybou >/dev/null 2>&1 || sudo useradd --system --gid cybou \
    --home-dir /var/lib/cybou --create-home --shell /usr/sbin/nologin cybou

  sudo install -d -m 0755 /etc/systemd/system
  sudo install -m 0644 systemd/system/cybou-web-gateway.service \
    /etc/systemd/system/cybou-web-gateway.service
  sudo install -m 0644 debian/Caddyfile /etc/caddy/Caddyfile
  sudo rm -f /etc/systemd/system/caddy.service.d/cybou.conf
  sudo rm -f /etc/cybou/web.env /etc/cybou/web-password

  sudo systemctl daemon-reload
  sudo systemctl enable cybou-web-gateway.service
  sudo systemctl restart cybou-web-gateway.service
  sudo systemctl restart caddy.service
  sudo systemctl --no-pager --full status cybou-web-gateway.service caddy.service
"

echo "==> Living Canvas deployed at https://vps-d0669a91.vps.ovh.net"
