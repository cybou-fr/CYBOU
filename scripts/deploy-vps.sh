#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Deploy this working tree to the remote evaluation host (docs/DEPLOYMENT.md).
#
# Usage: scripts/deploy-vps.sh [switch|boot|test|build|dry-activate]
#
# The build runs on the target rather than locally: the development workstation is Windows, and
# a NixOS closure cannot be built there. `test` activates without touching the bootloader,
# `boot` does the opposite, `switch` does both. Rolling back is a NixOS generation switch, not
# a re-deploy - see docs/DEPLOYMENT.md.
set -euo pipefail

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
