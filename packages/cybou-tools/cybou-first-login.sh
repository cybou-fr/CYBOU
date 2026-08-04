#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# cybou-first-login - apply the Cybou desktop once, on a clean profile.
#
# docs/04-desktop-layout.md sets the rules this obeys:
#   - runs once, guarded by a versioned marker
#   - never reapplies on later logins
#   - never deletes a configured user's panel
#   - never prevents the user from switching themes afterwards
#   - failure must not block login
#
# It exits 0 in every path on purpose. A desktop that will not start because its decoration
# script failed is a far worse outcome than a desktop that came up as stock Breeze.
set -uo pipefail

VERSION=1
STATE="${XDG_STATE_HOME:-$HOME/.local/state}/cybou"
MARKER="$STATE/desktop-layout-version"
CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}"
LAYOUT="$CONFIG/plasma-org.kde.plasma.desktop-appletsrc"

log() { echo "cybou-first-login: $*"; }

# Already initialised at this version: do nothing. This is the branch that runs on every
# login after the first, so it must be cheap and silent about success.
if [ -f "$MARKER" ] && [ "$(cat "$MARKER" 2>/dev/null)" = "$VERSION" ]; then
  exit 0
fi

# A profile that already has a panel layout belongs to someone who has used this desktop.
# Applying a look-and-feel here would replace their panel, which docs/04 forbids outright.
# The marker is still written, so this decision is made once and not re-examined every login.
if [ -f "$LAYOUT" ] && [ ! -f "$MARKER" ]; then
  log "existing Plasma layout found; leaving it alone"
  mkdir -p "$STATE" && echo "$VERSION" > "$MARKER"
  exit 0
fi

log "clean profile; applying Cybou Horizon"

if command -v plasma-apply-lookandfeel >/dev/null 2>&1; then
  if plasma-apply-lookandfeel --apply org.cybou.horizon.desktop; then
    log "look-and-feel applied"
  else
    log "could not apply the look-and-feel; leaving the session as it is"
  fi
else
  log "plasma-apply-lookandfeel not found; nothing applied"
fi

# The marker is written whether or not the theme applied. Retrying on every login would turn
# one bad session into a permanent loop, and docs/06 requires no permanent loop.
if mkdir -p "$STATE" && echo "$VERSION" > "$MARKER"; then
  log "marker written: $MARKER ($VERSION)"
else
  log "could not write the marker; this will be attempted again next login"
fi

exit 0
