#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Convert the remote evaluation host from its stock Debian image to NixOS running this flake.
#
# Usage: scripts/prepare-vps-nixos.sh <preflight|nix|build|convert|reboot|verify>
#
# The procedure is the one documented by NixOS itself for installing from a running Linux
# distribution: install Nix, build the target system closure, set it as the system profile, and
# hand the root filesystem to NIXOS_LUSTRATE on the next boot. No third-party infect script is
# downloaded and run as root; the only external artifact is the version-pinned Nix installer,
# whose checksum is verified below.
#
# Stages exist because they differ in reversibility:
#
#   preflight  reads the machine and refuses if it does not match systems/vps.nix
#   nix        installs the Nix package manager beside Debian            (reversible)
#   build      builds the NixOS closure on the target                    (reversible)
#   convert    switches the system profile and bootloader                (POINT OF NO RETURN)
#   reboot     boots into NixOS and lustrates the old root
#   verify     confirms the host came back as NixOS with the expected generation
#
# `convert` requires CYBOU_VPS_CONFIRM=I-UNDERSTAND in the environment. After it runs, the
# machine boots NixOS; recovering the Debian image means reinstalling it from the OVH panel.
set -euo pipefail

# shellcheck source=scripts/vps-env.sh
. "$(dirname "$0")/vps-env.sh"

# Pinned upstream installer for Nix 2.31.2. The versioned `install` script carries the checksums
# of the binary tarball it fetches; this checksum covers the script itself, so neither the script
# nor the tarball is taken on trust from the network alone.
NIX_INSTALLER_URL="https://releases.nixos.org/nix/nix-2.31.2/install"
NIX_INSTALLER_SHA256="078e2ffeddf6a9c1f22adf41458ccc46a58bb26911a9e01579645314f9982994"

# Facts read from the running Debian image before the first conversion. systems/vps.nix encodes
# the same values; if the machine stops matching them, the configuration is wrong for it and the
# deployment must not proceed on assumption.
EXPECTED_ROOT_UUID="9b085fa0-4a45-4ee4-8528-4ed80730760d"
EXPECTED_DISK="/dev/sda"
EXPECTED_IFACE="ens3"

stage_preflight() {
  cybou_ssh "
    set -eu
    fail() { echo \"preflight: \$1\" >&2; exit 1; }

    sudo -n true || fail 'passwordless sudo is required'
    [ \"\$(uname -m)\" = x86_64 ] || fail 'not x86_64'
    [ -b $EXPECTED_DISK ] || fail 'missing $EXPECTED_DISK'
    [ -d /sys/firmware/efi ] && fail 'host booted EFI; systems/vps.nix installs GRUB for BIOS'

    root_uuid=\$(findmnt -no UUID /)
    [ \"\$root_uuid\" = $EXPECTED_ROOT_UUID ] || fail \"root UUID \$root_uuid != $EXPECTED_ROOT_UUID\"

    ip link show $EXPECTED_IFACE >/dev/null 2>&1 || fail 'missing interface $EXPECTED_IFACE'

    free_gib=\$(df -BG --output=avail / | tail -1 | tr -dc '0-9')
    [ \"\$free_gib\" -ge 20 ] || fail \"only \${free_gib}G free on /; need 20G for the closure\"

    echo 'preflight: host matches systems/vps.nix'
    echo \"  distribution: \$(. /etc/os-release; echo \"\$PRETTY_NAME\")\"
    echo \"  kernel:       \$(uname -r)\"
    echo \"  free on /:    \${free_gib}G\"
    echo \"  nix present:  \$(command -v nix >/dev/null && echo yes || echo no)\"
  "
}

stage_nix() {
  cybou_ssh "
    set -eu
    if command -v nix >/dev/null || [ -d /nix ]; then
      echo 'nix: already installed'
      exit 0
    fi

    tmp=\$(mktemp -d)
    curl -sSL '$NIX_INSTALLER_URL' -o \"\$tmp/install\"
    echo '$NIX_INSTALLER_SHA256  '\"\$tmp/install\" | sha256sum -c -

    # Multi-user: the daemon owns the store, so a later deploy over SSH does not depend on
    # which account happens to run it.
    sh \"\$tmp/install\" --daemon --yes
    rm -rf \"\$tmp\"
  "

  cybou_ssh "
    set -eu
    sudo mkdir -p /etc/nix
    printf 'experimental-features = nix-command flakes\ntrusted-users = root debian\n' \
      | sudo tee /etc/nix/nix.conf.d-cybou >/dev/null
    grep -q 'experimental-features' /etc/nix/nix.conf 2>/dev/null \
      || cat /etc/nix/nix.conf.d-cybou | sudo tee -a /etc/nix/nix.conf >/dev/null
    sudo systemctl restart nix-daemon.service
    . /etc/profile.d/nix.sh 2>/dev/null || true
    nix --version
  "
}

stage_build() {
  cybou_push_source
  cybou_ssh "
    set -eu
    . /etc/profile.d/nix.sh 2>/dev/null || true
    cd '$CYBOU_VPS_SRC'
    nix build --print-build-logs \
      '.#nixosConfigurations.$CYBOU_VPS_FLAKE_ATTR.config.system.build.toplevel' \
      --out-link /home/debian/cybou-system
    readlink -f /home/debian/cybou-system
  "
}

stage_convert() {
  if [ "${CYBOU_VPS_CONFIRM:-}" != "I-UNDERSTAND" ]; then
    echo "convert: refusing without CYBOU_VPS_CONFIRM=I-UNDERSTAND" >&2
    echo "convert: this replaces the boot chain; the Debian image is not recoverable in place" >&2
    exit 3
  fi

  cybou_ssh "
    set -eu
    . /etc/profile.d/nix.sh 2>/dev/null || true
    system=\$(readlink -f /home/debian/cybou-system)
    [ -x \"\$system/bin/switch-to-configuration\" ] || { echo 'convert: run the build stage first' >&2; exit 1; }

    sudo nix-env -p /nix/var/nix/profiles/system --set \"\$system\"

    # NIXOS_LUSTRATE tells NixOS stage 2 to move the inherited filesystem into /old-root on the
    # next boot, keeping only /nix, /boot, and the paths listed here. The SSH host keys are kept
    # so the known_hosts entry for this machine stays valid across the conversion; treating a
    # changed host key as normal is exactly the habit a man-in-the-middle relies on.
    sudo touch /etc/NIXOS
    printf 'etc/ssh/ssh_host_ed25519_key\netc/ssh/ssh_host_ed25519_key.pub\netc/ssh/ssh_host_rsa_key\netc/ssh/ssh_host_rsa_key.pub\n' \
      | sudo tee /etc/NIXOS_LUSTRATE >/dev/null

    # Debian's /boot must not stay in the way of the NixOS one, and it is worth keeping until
    # the new system has proved it boots.
    [ -d /boot.bak ] || sudo mv -v /boot /boot.bak
    sudo mkdir -p /boot

    sudo NIXOS_INSTALL_BOOTLOADER=1 \"\$system/bin/switch-to-configuration\" boot
    echo 'convert: bootloader installed; the next boot is NixOS'
  "
}

stage_reboot() {
  echo "==> rebooting $CYBOU_VPS_HOST into NixOS"
  cybou_ssh "sudo systemctl reboot" || true

  for _ in $(seq 1 60); do
    sleep 10
    if cybou_ssh "true" 2>/dev/null; then
      echo "==> host is back"
      return 0
    fi
    echo "    waiting for the host to come back ..."
  done

  echo "reboot: host did not return within 10 minutes; use the OVH serial console" >&2
  return 1
}

stage_verify() {
  cybou_ssh "
    set -eu
    . /etc/os-release
    echo \"distribution:  \$PRETTY_NAME\"
    echo \"generation:    \$(readlink -f /run/current-system)\"
    echo \"hostname:      \$(hostname)\"
    echo \"nixos version: \$(nixos-version 2>/dev/null || echo 'not NixOS')\"
    echo '--- failed units ---'
    systemctl --failed --no-legend || true
    echo '--- listeners ---'
    ss -ltnp 2>/dev/null | head -10 || true
  "
}

case "${1:-}" in
  preflight) stage_preflight ;;
  nix) stage_nix ;;
  build) stage_build ;;
  convert) stage_convert ;;
  reboot) stage_reboot ;;
  verify) stage_verify ;;
  *)
    echo "usage: $0 <preflight|nix|build|convert|reboot|verify>" >&2
    exit 2
    ;;
esac
