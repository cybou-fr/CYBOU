# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Cybou Horizon SDDM theme, derived from Breeze.
#
# SDDM is not the Plasma lock screen and does not follow the Global Theme; it has its own theme
# directory, which is why the login background stayed Breeze while the desktop was already
# Cybou while preserving upstream package boundaries (docs/ARCHITECTURE.md).
#
# Deriving from Breeze rather than writing QML from scratch is deliberate: a broken greeter QML
# is a blocking release defect (docs/08), and Breeze's greeter is already correct. Cybou changes
# the background and the identity, nothing structural. This is a KDE-derived modification and
# keeps a licence compatible with its source, per docs/adr/ADR-0007-reuse-3.x-compliance.md.
{
  lib,
  runCommand,
  kdePackages,
  horizon-wallpaper,
}:
runCommand "cybou-horizon-sddm"
  {
    meta = {
      description = "Cybou Horizon SDDM theme";
      license = lib.licenses.gpl2Plus; # inherited from the Breeze theme it derives from
    };
  }
  ''
    src=${kdePackages.plasma-desktop}/share/sddm/themes/breeze
    if [ ! -d "$src" ]; then
      echo "Breeze SDDM theme not found at $src" >&2
      echo "Available under plasma-workspace/share:" >&2
      ls ${kdePackages.plasma-desktop}/share >&2
      exit 1
    fi

    dir=$out/share/sddm/themes/cybou-horizon
    mkdir -p "$(dirname "$dir")"
    cp -rL "$src" "$dir"
    chmod -R u+w "$out"

    # Horizon Field replaces the Breeze background. The greeter renders SVG, so the same
    # generated wallpaper serves both the desktop and the login screen - one source, no drift.
    install -Dm444 \
      ${horizon-wallpaper}/share/wallpapers/CybouHorizonDark/contents/images/3840x2160.svg \
      "$dir/components/artwork/background.svg"

    if [ -f "$dir/theme.conf" ]; then
      sed -i 's|^background=.*|background=components/artwork/background.svg|' "$dir/theme.conf"
      grep -q '^background=' "$dir/theme.conf" || \
        echo 'background=components/artwork/background.svg' >> "$dir/theme.conf"
    fi

    if [ -f "$dir/metadata.desktop" ]; then
      sed -i 's|^Name=.*|Name=Cybou Horizon|; s|^Description=.*|Description=Cybou Horizon login screen|' \
        "$dir/metadata.desktop"
    fi
  ''
