# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Shared settings for the remote evaluation host (docs/DEPLOYMENT.md). Sourced, not executed.
#
# Every value can be overridden from the environment so the same scripts can address a second
# host without editing them - the default is the only host currently in use.

CYBOU_VPS_HOST="${CYBOU_VPS_HOST:-debian@vps-d0669a91.vps.ovh.net}"
CYBOU_VPS_FLAKE_ATTR="${CYBOU_VPS_FLAKE_ATTR:-cybou-vps}"
CYBOU_VPS_SRC="${CYBOU_VPS_SRC:-/home/debian/cybou-src}"

# BatchMode keeps a missing key an immediate error instead of an interactive prompt that a
# CI shell would hang on. ControlMaster reuses one connection for the sync plus the build.
CYBOU_SSH_OPTS="${CYBOU_SSH_OPTS:--o BatchMode=yes -o ServerAliveInterval=30 -o ServerAliveCountMax=10}"

cybou_ssh() {
  # shellcheck disable=SC2086 # word splitting of the option list is intended
  ssh $CYBOU_SSH_OPTS "$CYBOU_VPS_HOST" "$@"
}

# The working tree is pushed, not the last commit: a deployment that only ever matched HEAD
# could not test an unfinished change, which is the main reason this host exists. Build outputs
# and dependency caches are excluded because they are large, host-specific, and rebuilt anyway.
cybou_push_source() {
  local repo_root
  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

  echo "==> pushing $repo_root to $CYBOU_VPS_HOST:$CYBOU_VPS_SRC"
  tar -C "$repo_root" -czf - \
    --exclude=.git \
    --exclude=node_modules \
    --exclude=target \
    --exclude=dist \
    --exclude=build \
    --exclude='result' \
    --exclude='result-*' \
    . | cybou_ssh "
      set -eu
      rm -rf '$CYBOU_VPS_SRC.new'
      mkdir -p '$CYBOU_VPS_SRC.new'
      tar -xzf - -C '$CYBOU_VPS_SRC.new'
      rm -rf '$CYBOU_VPS_SRC'
      mv '$CYBOU_VPS_SRC.new' '$CYBOU_VPS_SRC'
    "
}
