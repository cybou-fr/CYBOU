#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
set -uo pipefail

# Version 2 migrates Presence from the top panel to the dedicated right-side Mind dock.
VERSION=2
STATE="${XDG_STATE_HOME:-$HOME/.local/state}/cybou"
MARKER="$STATE/desktop-layout-version"
CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}"
LAYOUT="$CONFIG/plasma-org.kde.plasma.desktop-appletsrc"

log() { echo "cybou-first-login: $*"; }

if [ -f "$MARKER" ] && [ "$(cat "$MARKER" 2>/dev/null)" = "$VERSION" ]; then
  exit 0
fi

# Preserve a pre-existing non-Cybou layout on the user's first initialization.
if [ -f "$LAYOUT" ] && [ ! -f "$MARKER" ]; then
  log "existing Plasma layout found; leaving it alone"
  mkdir -p "$STATE" && echo "$VERSION" > "$MARKER"
  exit 0
fi

log "applying Cybou Horizon layout version $VERSION"
if command -v plasma-apply-lookandfeel >/dev/null 2>&1; then
  if plasma-apply-lookandfeel --apply org.cybou.horizon.desktop; then
        log "look-and-feel applied"
    else
        log "could not apply the look-and-feel; leaving the session as it is"
    fi
else
  log "plasma-apply-lookandfeel not found; nothing applied"
fi

if mkdir -p "$STATE" && echo "$VERSION" > "$MARKER"; then
  log "marker written: $MARKER ($VERSION)"
else
  log "could not write the marker; this will be attempted again next login"
fi
exit 0
