#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Build on Debian 13, install the Rust gateway, Mind daemons, shared WASM frontend, and activate HTTPS.
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

  sudo install -d -m 0755 /usr/libexec/cybou /usr/share/cybou/web /usr/lib/systemd/user /etc/systemd/system

  # Install all 12 Mind daemons and web gateway to /usr/libexec/cybou
  DAEMONS=(
    cybou-web-gateway
    cybou-eventd
    cybou-identityd
    cybou-healthd
    cybou-intentiond
    cybou-predictord
    cybou-perceptiond
    cybou-epistemicd
    cybou-contextd
    cybou-workspaced
    cybou-lifecycled
    cybou-selfd
    cybou-presenced
  )
  for daemon in \"\${DAEMONS[@]}\"; do
    sudo install -m 0755 \"\$CYBOU_VPS_TARGET/release/\$daemon\" \"/usr/libexec/cybou/\$daemon\"
  done

  # Install Living Canvas web assets
  sudo cp -a target/living-canvas/. /usr/share/cybou/web/
  sudo chown -R root:root /usr/libexec/cybou /usr/share/cybou

  # Install systemd user units and target
  sudo install -m 0644 systemd/user/*.service systemd/user/*.target /usr/lib/systemd/user/

  sudo getent group cybou >/dev/null || sudo groupadd --system cybou
  sudo id cybou >/dev/null 2>&1 || sudo useradd --system --gid cybou \
    --home-dir /var/lib/cybou --create-home --shell /usr/sbin/nologin cybou

  # Install systemd system unit for web gateway and Caddy
  sudo install -m 0644 systemd/system/cybou-web-gateway.service \
    /etc/systemd/system/cybou-web-gateway.service
  sudo install -m 0644 debian/Caddyfile /etc/caddy/Caddyfile
  sudo rm -f /etc/systemd/system/caddy.service.d/cybou.conf
  sudo rm -f /etc/cybou/web.env /etc/cybou/web-password

  sudo systemctl daemon-reload
  sudo systemctl enable cybou-web-gateway.service
  sudo systemctl restart cybou-web-gateway.service
  sudo systemctl restart caddy.service

  # The Mind daemons are systemd *user* units owned by the cybou user. A system user with
  # nologin never gets a login session, so without lingering its user manager is never
  # started and every unit installed above would stay inert.
  sudo loginctl enable-linger cybou
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    sudo systemctl --user --machine=cybou@.host daemon-reload 2>/dev/null && break
    sleep 1
  done
  sudo systemctl --user --machine=cybou@.host enable --now cybou-mind.target
  sudo systemctl --user --machine=cybou@.host --no-pager --full status cybou-mind.target || true

  sudo systemctl --no-pager --full status cybou-web-gateway.service caddy.service
"

echo "==> Living Canvas and Mind daemons deployed at https://vps-d0669a91.vps.ovh.net"
