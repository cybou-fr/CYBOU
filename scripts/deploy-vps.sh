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
  mkdir -p target/living-canvas
  cd crates/living-canvas
  trunk build --release
  cd ../..

  sudo install -d -m 0755 /usr/libexec/cybou /usr/share/cybou/web /usr/lib/systemd/user /etc/systemd/system

  # Install every Mind daemon and the web gateway to /usr/libexec/cybou
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
    cybou-meaningd
    cybou-telemetryd
    cybou-model-brokerd
    cybou-authd
    cybou-workspaced
    cybou-lifecycled
    cybou-selfd
    cybou-presenced
    cybou-shelld
  )
  for daemon in \"\${DAEMONS[@]}\"; do
    sudo install -m 0755 '$CYBOU_VPS_TARGET'/release/\"\$daemon\" \"/usr/libexec/cybou/\$daemon\"
  done

  # Install Living Canvas web assets
  sudo cp -a target/living-canvas/. /usr/share/cybou/web/
  sudo chown -R root:root /usr/libexec/cybou /usr/share/cybou

  # Install systemd user units and target
  sudo install -m 0644 systemd/user/*.service systemd/user/*.target /usr/lib/systemd/user/

  # The desktop session launcher is installed and left disabled. This host has no seat and no
  # display; a target that started a compositor here would fail at something it does not need, and
  # a unit that is present but not enabled is the honest way to ship a session for machines that do
  # have one.
  sudo install -m 0755 scripts/cybou-desktop-session.sh /usr/libexec/cybou/cybou-desktop-session.sh

  sudo getent group cybou >/dev/null || sudo groupadd --system cybou
  sudo id cybou >/dev/null 2>&1 || sudo useradd --system --gid cybou \
    --home-dir /var/lib/cybou --create-home --shell /usr/sbin/nologin cybou

  # Create shell jail root
  sudo install -d -m 0755 -o cybou -g cybou /var/lib/cybou/shell-jail
  # Something to look at. An empty sandbox and a broken one are indistinguishable from the outside,
  # which is how a misconfigured jail went unnoticed.
  sudo install -d -m 0755 -o cybou -g cybou /var/lib/cybou/shell-jail/notes
  printf '%s
'     'This is the bounded filesystem the Shell and the File Manager can read.'     'It is not the host filesystem: nothing outside this directory is reachable,'     'and nothing here can be written from the desktop.'     | sudo tee /var/lib/cybou/shell-jail/README.txt >/dev/null
  sudo chown cybou:cybou /var/lib/cybou/shell-jail/README.txt

  # The gateway used to be a system service, which is why it could only serve fixtures: a system
  # service has no session bus and therefore no way to reach Presence1. It is a user unit now,
  # started by the same user manager as the organs it presents.
  sudo systemctl disable --now cybou-web-gateway.service 2>/dev/null || true
  sudo rm -f /etc/systemd/system/cybou-web-gateway.service

  sudo install -m 0644 debian/Caddyfile /etc/caddy/Caddyfile
  sudo rm -f /etc/systemd/system/caddy.service.d/cybou.conf
  sudo rm -f /etc/cybou/web.env /etc/cybou/web-password

  # Who may sign in. Membership in this group is the grant: being a valid Linux account is not the
  # same as being someone this system answers to, and without the group every service account on
  # the host would be a way in.
  sudo getent group cybou-access >/dev/null || sudo groupadd --system cybou-access

  # Provision demo user for live desktop authentication
  if ! id demo >/dev/null 2>&1; then
    sudo useradd -m -s /bin/bash -G cybou-access demo
    echo \"demo:cybou2026\" | sudo chpasswd
  else
    sudo usermod -a -G cybou-access demo
    echo \"demo:cybou2026\" | sudo chpasswd
  fi
  sudo -u demo mkdir -p /home/demo/documents /home/demo/downloads /home/demo/projects
  sudo -u demo sh -c 'echo -e \"# Welcome to CYBOU Sovereign Desktop\n\nThis is your bounded read-only home directory.\nYou can inspect files, navigate directories, and explore your sovereign environment.\" > /home/demo/welcome.md'

  # The PAM stack the helper opens. Ordinary Unix password checking and nothing else; the account
  # module is what makes locking an account actually revoke access rather than only look like it.
  sudo install -m 0644 debian/pam-cybou /etc/pam.d/cybou

  # The helper is a system service because it is the one thing here that needs root. Its socket is
  # group-owned by cybou, so only the gateway can attempt a password.
  sudo install -m 0644 systemd/system/cybou-authd.service /etc/systemd/system/
  sudo systemctl daemon-reload
  sudo systemctl enable --now cybou-authd.service
  sudo systemctl restart cybou-authd.service

  # The shared secret this replaced is removed rather than left lying about. A second way in that
  # nobody maintains is how a temporary arrangement outlives the reason for it.
  sudo rm -f /var/lib/cybou/access-credential

  sudo systemctl daemon-reload
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

  # enable --now starts what is stopped and leaves what is running alone, so on every deploy after
  # the first the organs would keep executing the binaries they were started with. Each unit is
  # PartOf=cybou-mind.target, which is the directive that propagates a restart of the target down
  # to them.
  sudo systemctl --user --machine=cybou@.host restart cybou-mind.target
  sudo systemctl --user --machine=cybou@.host --no-pager --full status cybou-mind.target || true

  sudo systemctl --no-pager --full status caddy.service
"

echo "==> Living Canvas and Mind daemons deployed at https://vps-d0669a91.vps.ovh.net"
