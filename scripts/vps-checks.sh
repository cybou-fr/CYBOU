#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Run repository gates on the remote evaluation host (docs/DEPLOYMENT.md).
#
# Usage: scripts/vps-checks.sh [fast|full|<flake check name> ...]
#
# `fast` is the set CI runs on every push. `full` adds the NixOS VM tests, which need /dev/kvm;
# the host has it, but a VM gate still costs minutes and a lot of store space, so it is not the
# default. The checks run on the target because the workstation is Windows and cannot evaluate
# a NixOS test.
set -euo pipefail

fast_checks=(formatting reuse package-metadata cognitive-docs mind-access qml-api ui-polish rust-foundation)
full_checks=("${fast_checks[@]}" vm-smoke p4-plasma-lifecycle lifecycle-continuity m6-recovery-boundary)

case "${1:-fast}" in
  fast) checks=("${fast_checks[@]}") ;;
  full) checks=("${full_checks[@]}") ;;
  *) checks=("$@") ;;
esac

# shellcheck source=scripts/vps-env.sh
. "$(dirname "$0")/vps-env.sh"

cybou_push_source

targets=""
for check in "${checks[@]}"; do
  targets="$targets '$CYBOU_VPS_SRC#checks.x86_64-linux.$check'"
done

echo "==> nix build ${checks[*]}"
# shellcheck disable=SC2029 # $targets must expand here, not on the remote shell
cybou_ssh "
  set -eu
  cd '$CYBOU_VPS_SRC'
  nix build --no-link --print-build-logs $targets
"
echo "==> checks passed: ${checks[*]}"
