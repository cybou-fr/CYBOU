# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Presence + discoverability handle applets (KPackage, Plasma/Applet).
#
# Copies rather than symlinks: Plasma 6 KPackage rejects symlinks inside a package. Directory
# names must equal KPlugin.Id or kpackagetool6 will not find them.
{
  lib,
  runCommand,
  python3,
}:
runCommand "cybou-presence-applet"
  {
    nativeBuildInputs = [ python3 ];
    meta = {
      description = "Cybou Presence and Mind access applets for Plasma 6";
      license = lib.licenses.mit;
    };
  }
  ''
    root=$out/share/plasma/plasmoids
    presence=$root/org.cybou.presence
    handle=$root/org.cybou.mindhandle

    mkdir -p "$presence" "$handle"

    cp -rL ${./org.cybou.presence}/. "$presence"/
    cp -rL ${./org.cybou.mindhandle}/. "$handle"/
    chmod -R u+w "$root"

    python3 ${../../scripts/validate-packages.py} "$root"
    python3 ${../../scripts/validate-qml-api.py} "$presence"

    python3 ${../../scripts/validate-mind-access.py} \
      "$presence" \
      "$handle" \
      ${../cybou-layout-templates/org.cybou.plasma.minddock/contents/layout.js}
  ''
