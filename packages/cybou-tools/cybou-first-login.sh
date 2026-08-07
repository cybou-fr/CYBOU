#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT

set -uo pipefail

VERSION=2

STATE="${XDG_STATE_HOME:-$HOME/.local/state}/cybou"
MARKER="$STATE/desktop-layout-version"
CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}"
LAYOUT="$CONFIG/plasma-org.kde.plasma.desktop-appletsrc"

log() {
    echo "cybou-first-login: $*"
}

mkdir -p "$STATE" || {
    log "cannot create state directory"
    exit 0
}

current_version=0

if [ -f "$MARKER" ]; then
    current_version="$(cat "$MARKER" 2>/dev/null || echo 0)"
fi

# Уже актуальная версия.
if [ "$current_version" = "$VERSION" ]; then
    exit 0
fi

#
# Existing layout + no previous Cybou marker:
# this is assumed to be the user's own Plasma configuration.
# Never replace it automatically.
#
if [ -f "$LAYOUT" ] && [ "$current_version" = "0" ]; then
    log "existing non-Cybou Plasma layout found; preserving it"
    echo "$VERSION" > "$MARKER"
    exit 0
fi

if ! command -v plasma-apply-lookandfeel >/dev/null 2>&1; then
    log "plasma-apply-lookandfeel not found"
    exit 0
fi

#
# Cybou migration.
#
# An older Cybou marker proves that this layout was previously
# initialized by Cybou, so replacing it is safe.
#
if [ "$current_version" -gt 0 ] 2>/dev/null; then
    log "migrating Cybou desktop layout $current_version -> $VERSION"

    if plasma-apply-lookandfeel \
        --apply org.cybou.horizon.desktop \
        --resetLayout
    then
        log "Cybou desktop layout migrated"
        echo "$VERSION" > "$MARKER"
    else
        log "layout migration failed; old marker retained"
    fi

    exit 0
fi

#
# Fresh Cybou profile.
#
log "initializing Cybou Horizon"

if plasma-apply-lookandfeel \
    --apply org.cybou.horizon.desktop \
    --resetLayout
then
    log "Cybou Horizon initialized"
    echo "$VERSION" > "$MARKER"
else
    log "could not initialize Cybou Horizon"
fi

exit 0