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

  sudo install -d -m 0755 /usr/libexec/cybou /usr/share/cybou/web /usr/lib/systemd/user \
    /usr/share/dbus-1/system.d /etc/systemd/system /etc/cybou

  # Install every Mind daemon, the typed Body executor, and the web gateway.
  DAEMONS=(
    cybou-web-gateway
    cybou-eventd
    cybou-identityd
    cybou-healthd
    cybou-host-filesd
    cybou-intentiond
    cybou-predictord
    cybou-perceptiond
    cybou-epistemicd
    cybou-contextd
    cybou-meaningd
    cybou-telemetryd
    cybou-model-brokerd
    cybou-agent-gateway
    cybou-authd
    cybou-workspaced
    cybou-lifecycled
    cybou-selfd
    cybou-presenced
    cybou-shelld
    cybou-actiond
    cybou-remediationd
    cybou-executord
    cybou-agentd
    cybou-egressd
  )
  for daemon in \"\${DAEMONS[@]}\"; do
    sudo install -m 0755 '$CYBOU_VPS_TARGET'/release/\"\$daemon\" \"/usr/libexec/cybou/\$daemon\"
  done
  # The installed ACP command is discovery-only. Process probing remains a development example so
  # a registry entry cannot accidentally be launched outside an Agent Capsule.
  sudo install -m 0755 '$CYBOU_VPS_TARGET'/release/cybou-acp /usr/bin/cybou-acp
  # Capsule transports are infrastructure, not tools the agent grants itself. The OpenCode pack is
  # fetched from its immutable upstream release and digest-checked before it enters the read-only
  # /usr view shared by capsules. No provider credential is installed here.
  sudo install -m 0755 '$CYBOU_VPS_TARGET'/release/cybou-capsule-enter \
    /usr/libexec/cybou/cybou-capsule-enter
  sudo install -m 0755 '$CYBOU_VPS_TARGET'/release/cybou-egress-bridge \
    /usr/libexec/cybou/cybou-egress-bridge
  sudo install -m 0755 '$CYBOU_VPS_TARGET'/release/cybou-model-bridge \
    /usr/libexec/cybou/cybou-model-bridge
  sudo bash scripts/install-opencode-pack.sh

  # Install Living Canvas web assets by replacing the directory, not by merging into it.
  #
  # Copying into the live directory only ever adds. Every build produces content-hashed bundles under
  # new names, so nothing was ever overwritten and nothing was ever removed: by 2026-08-24 the web
  # root held 64 WebAssembly bundles and 94 MB, accumulated since the first deployment, on a machine
  # whose whole purpose is to notice a disk filling up. It would have diagnosed itself eventually,
  # which is the least dignified way for that feature to get its first real finding.
  #
  # Stage and swap rather than emptying in place; delete-then-copy leaves the surface
  # serving nothing for as long as the copy takes, and a failure in between leaves it serving nothing
  # at all. A rename is one step, and the old directory is only removed once the new one is live.
  sudo rm -rf /usr/share/cybou/web.new /usr/share/cybou/web.old
  sudo install -d -m 0755 /usr/share/cybou/web.new
  sudo cp -a target/living-canvas/. /usr/share/cybou/web.new/
  sudo chown -R root:root /usr/share/cybou/web.new
  if [ -d /usr/share/cybou/web ]; then
    sudo mv /usr/share/cybou/web /usr/share/cybou/web.old
  fi
  sudo mv /usr/share/cybou/web.new /usr/share/cybou/web
  sudo rm -rf /usr/share/cybou/web.old
  sudo chown -R root:root /usr/libexec/cybou

  # Install systemd user units and target
  sudo install -m 0644 systemd/user/*.service systemd/user/*.target /usr/lib/systemd/user/

  # The desktop session launcher is installed and left disabled. This host has no seat and no
  # display; a target that started a compositor here would fail at something it does not need, and
  # a unit that is present but not enabled is the honest way to ship a session for machines that do
  # have one.
  sudo install -m 0755 scripts/cybou-desktop-session.sh /usr/libexec/cybou/cybou-desktop-session.sh
  sudo install -m 0755 scripts/cybou-action-policy.sh /usr/sbin/cybou-action-policy

  sudo getent group cybou >/dev/null || sudo groupadd --system cybou
  sudo id cybou >/dev/null 2>&1 || sudo useradd --system --gid cybou \
    --home-dir /var/lib/cybou --create-home --shell /usr/sbin/nologin cybou

  # Standing authorization is an operator-owned file and grants nothing on first install. Never
  # overwrite it on deployment: replacing a person's policy with the repository default would be
  # an authorization change disguised as a software update.
  if [ ! -e /etc/cybou/action-policy.env ]; then
    printf '%s\n' 'CYBOU_PREAUTHORIZED_ACTIONS=' | sudo tee /etc/cybou/action-policy.env >/dev/null
  fi
  sudo chown root:root /etc/cybou/action-policy.env
  sudo chmod 0644 /etc/cybou/action-policy.env

  # Action1 and the executor share the system transport so D-Bus can authenticate their distinct
  # UIDs. Policy still belongs to the unprivileged Action1 process; only the fixed Body adapter is
  # root. Remove the obsolete cross-UID session-bus address from pre-release deployments.
  sudo rm -f /etc/cybou/executor.env

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
  sudo install -m 0644 systemd/system/cybou-authd.service \
    systemd/system/cybou-host-filesd@.service \
    systemd/system/cybou-executord.service systemd/system/cybou-agent-gateway@.service \
    /etc/systemd/system/
  # The session owner writes the lease and the launch file here; systemd reads them back as root
  # through LoadCredential and EnvironmentFile. Owned by cybou, because the owner writing its own
  # session files is not a privilege boundary — the boundary is the provider credential, which stays
  # root-only and never appears here.
  sudo install -d -m 0700 -o cybou -g cybou /run/cybou-agent-leases
  # What an operator has approved for agents to run under. An empty catalogue offers nothing, which
  # is the fail-closed state by construction rather than by a flag: a caller can only name a profile
  # that is in here, and until somebody writes one there is nothing to name. Root-owned and readable
  # by cybou, because the session owner must read it and must never be able to add to it.
  if [ ! -e /etc/cybou/agent-profiles.json ]; then
    printf '%s\n' '[]' | sudo tee /etc/cybou/agent-profiles.json >/dev/null
  fi
  sudo chown root:cybou /etc/cybou/agent-profiles.json
  sudo chmod 0640 /etc/cybou/agent-profiles.json

  # Admission is a promise across every live session, not a per-process preflight. Keep the initial
  # host closed until an operator chooses real totals for this machine; absence means the historical
  # unbounded mode, and Agent1 deliberately will not launch through a reachable surface in that mode.
  # Preserve every valid operator policy. Repair only a missing or malformed file to the explicit
  # zero-capacity policy; malformed input must not silently restore the historical unbounded mode.
  if ! sudo python3 -c 'import json, sys; d = json.load(open(sys.argv[1])); required = {\"maxSessions\", \"memoryMiB\", \"cpus\", \"tasksMax\", \"spendUnits\"}; assert set(d) == required; assert all(type(d[k]) is int and d[k] >= 0 for k in required)' \
      /etc/cybou/agent-capacity.json 2>/dev/null; then
    printf '%s\n' \
      '{\"maxSessions\":0,\"memoryMiB\":0,\"cpus\":0,\"tasksMax\":0,\"spendUnits\":0}' \
      | sudo tee /etc/cybou/agent-capacity.json >/dev/null
  fi
  sudo chown root:cybou /etc/cybou/agent-capacity.json
  sudo chmod 0640 /etc/cybou/agent-capacity.json

  # The one privileged step of a launch. Start and stop, that unit template, that user, nothing else.
  sudo install -d -m 0755 /etc/polkit-1/rules.d
  sudo install -m 0644 debian/cybou-agent-gateway.rules /etc/polkit-1/rules.d/50-cybou.rules
  # Provider routing is operator policy and the master key is a systemd credential. Deployment
  # creates fail-closed placeholders once, never replaces configured values, and never starts a
  # per-capsule gateway by itself.
  if [ ! -e /etc/cybou/provider.env ]; then
    printf '%s\n' \
      'CYBOU_LITELLM_BASE_URL=' \
      'CYBOU_LITELLM_PROVIDER=' \
      'CYBOU_LITELLM_MODEL_GROUP=' \
      'CYBOU_LITELLM_DEPLOYMENT_SHA256=' \
      'CYBOU_LITELLM_ZERO_COST=' \
      'CYBOU_LITELLM_TIMEOUT_MS=30000' \
      'CYBOU_MODEL_MICROUSD_PER_UNIT=1' \
      | sudo tee /etc/cybou/provider.env >/dev/null
  fi
  if [ ! -e /etc/cybou/litellm-master-key ]; then
    sudo install -m 0600 -o root -g root /dev/null /etc/cybou/litellm-master-key
  fi
  sudo chown root:root /etc/cybou/provider.env /etc/cybou/litellm-master-key
  sudo chmod 0644 /etc/cybou/provider.env
  sudo chmod 0600 /etc/cybou/litellm-master-key
  sudo install -m 0644 debian/org.cybou.Body.Executor1.conf /usr/share/dbus-1/system.d/
  sudo install -m 0644 debian/cybou-agent.conf /usr/lib/tmpfiles.d/cybou-agent.conf
  sudo systemd-tmpfiles --create /usr/lib/tmpfiles.d/cybou-agent.conf
  sudo systemctl daemon-reload
  sudo systemctl reload dbus.service
  sudo systemctl enable --now cybou-authd.service
  sudo systemctl restart cybou-authd.service
  # HostUserPath is an explicit per-account capability. The demo account is the only account this
  # deployment provisions, so it is the only owner instance enabled here. Additional admitted
  # accounts require an equally explicit enable by the operator.
  sudo systemctl enable --now cybou-host-filesd@demo.service
  sudo systemctl restart cybou-host-filesd@demo.service

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
  # Agent1 is deliberately outside Mind's target, but it is still a deployed runtime and must be
  # started explicitly. Enabling it independently preserves that ownership boundary across boots.
  sudo systemctl --user --machine=cybou@.host enable --now cybou-agentd.service
  sudo systemctl --user --machine=cybou@.host --no-pager --full status cybou-mind.target || true

  # Start only after the lingering user manager has brought up Action1. If its system-bus name is
  # not ready yet, Restart=on-failure keeps the executor fail-closed until it can claim permits.
  sudo systemctl enable --now cybou-executord.service
  # enable --now leaves an already-running process on its old binary after an upgrade.
  sudo systemctl restart cybou-executord.service
  sudo systemctl --no-pager --full status cybou-executord.service

  sudo systemctl --no-pager --full status caddy.service
"

echo "==> Living Canvas and Mind daemons deployed at https://vps-d0669a91.vps.ovh.net"
