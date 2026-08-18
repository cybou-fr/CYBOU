#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Build repository checks inside the NixOS WSL2 distribution.
# Invoke from Windows with: wsl -d NixOS -- bash /mnt/c/.../scripts/wsl-checks.sh [fast|full|check ...]
set -euo pipefail

fast_checks=(formatting reuse package-metadata cognitive-docs mind-access qml-api ui-polish rust-foundation web-ui desktop-shell)
full_checks=("${fast_checks[@]}" vm-smoke p4-plasma-lifecycle lifecycle-continuity m6-recovery-boundary)

case "${1:-fast}" in
  fast) checks=("${fast_checks[@]}") ;;
  full) checks=("${full_checks[@]}") ;;
  *) checks=("$@") ;;
esac

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work_root="$(mktemp -d --tmpdir cybou-wsl-checks.XXXXXX)"
cleanup() {
  rm -rf -- "$work_root"
}
trap cleanup EXIT INT TERM

tar -C "$repo_root" -cf - \
  --exclude=.git \
  --exclude=node_modules \
  --exclude=target \
  --exclude=dist \
  --exclude=build \
  --exclude='result' \
  --exclude='result-*' \
  . | tar -C "$work_root" -xf -

targets=()
for check in "${checks[@]}"; do
  targets+=("$work_root#checks.x86_64-linux.$check")
done

echo "==> NixOS WSL2: nix build ${checks[*]}"
nix build --no-link --print-build-logs "${targets[@]}"
echo "==> WSL checks passed: ${checks[*]}"
