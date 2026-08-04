#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# cybou-reset-desktop - return the desktop to stock Breeze.
#
# docs/04-desktop-layout.md prescribes the order: explain first, back up second, only then
# change anything, leave personal files alone, and print where the backup went.
#
# Runs as the user, never as root. Touches only KDE configuration.
set -uo pipefail

CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}"
STATE="${XDG_STATE_HOME:-$HOME/.local/state}/cybou"
STAMP="$(date +%Y%m%d-%H%M%S)"
BACKUP="$STATE/reset-$STAMP"

FILES=(
  plasma-org.kde.plasma.desktop-appletsrc
  plasmarc
  plasmashellrc
  kdeglobals
  kwinrc
  kscreenlockerrc
  ksplashrc
  kcminputrc
)

cat <<EOF
cybou-reset-desktop

This returns the desktop to the stock Breeze appearance and a standard panel.

It will:
  - back up your KDE configuration to
    $BACKUP
  - reset the global theme, colours, window decoration, splash and panel layout

It will NOT touch your documents, your applications or any file outside KDE's
configuration. You can reapply Cybou Horizon afterwards from System Settings.

EOF

if [ "${1:-}" != "--yes" ]; then
  read -r -p "Continue? [y/N] " reply
  case "$reply" in
    [yY] | [yY][eE][sS]) ;;
    *)
      echo "Nothing was changed."
      exit 0
      ;;
  esac
fi

mkdir -p "$BACKUP" || {
  echo "Could not create $BACKUP - nothing was changed." >&2
  exit 1
}

saved=0
for f in "${FILES[@]}"; do
  if [ -f "$CONFIG/$f" ]; then
    cp -a "$CONFIG/$f" "$BACKUP/" && saved=$((saved + 1))
  fi
done
echo "Backed up $saved file(s) to $BACKUP"

# The layout is what makes a stale panel survive a theme change, so it goes first.
rm -f "$CONFIG/plasma-org.kde.plasma.desktop-appletsrc"

# Ask Plasma to apply Breeze rather than editing its files by hand: the tools know which
# services to notify, and a hand-edited config needs a logout to take effect anyway.
if command -v plasma-apply-lookandfeel >/dev/null 2>&1; then
  plasma-apply-lookandfeel --apply org.kde.breezedark.desktop || true
fi
if command -v plasma-apply-desktoptheme >/dev/null 2>&1; then
  plasma-apply-desktoptheme breeze-dark || true
fi
if command -v plasma-apply-colorscheme >/dev/null 2>&1; then
  plasma-apply-colorscheme BreezeDark || true
fi

# The first-login marker is versioned; clearing it lets the initializer run once more on a
# genuinely clean profile without duplicating anything (docs/04, CYB-023).
rm -f "$STATE/desktop-layout-version"

cat <<EOF

Done. Log out and back in to complete the reset.

Backup: $BACKUP
Restore a single file with:
  cp "$BACKUP/<file>" "$CONFIG/<file>"
EOF
