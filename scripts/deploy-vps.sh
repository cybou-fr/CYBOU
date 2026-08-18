#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# RETIRED: OVH is no longer a Cybou deployment or evaluation target.
#
# This file remains only to fail old commands clearly.
#
# The build runs on the target rather than locally: the development workstation is Windows, and
# a NixOS closure cannot be built there. `test` activates without touching the bootloader,
# `boot` does the opposite, `switch` does both. Rolling back is a NixOS generation switch, not
# a re-deploy - see docs/DEPLOYMENT.md.
set -euo pipefail

echo "deploy-vps: retired; OVH is no longer a Cybou Nix evaluation target" >&2
exit 2

action="${1:-switch}"
case "$action" in
  switch | boot | test | build | dry-activate) ;;
  *)
    echo "usage: $0 [switch|boot|test|build|dry-activate]" >&2
    exit 2
    ;;
esac

# shellcheck source=scripts/vps-env.sh
. "$(dirname "$0")/vps-env.sh"

cybou_push_source

echo "==> nixos-rebuild $action --flake $CYBOU_VPS_SRC#$CYBOU_VPS_FLAKE_ATTR"
cybou_ssh "
  set -eu
  sudo nixos-rebuild $action --flake '$CYBOU_VPS_SRC#$CYBOU_VPS_FLAKE_ATTR' --print-build-logs
"

echo "==> active generation"
cybou_ssh "readlink -f /run/current-system; systemctl --failed --no-legend || true"
