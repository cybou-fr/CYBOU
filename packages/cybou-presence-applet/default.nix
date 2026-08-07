# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# The Presence applet (KPackage, Plasma/Applet).
#
# Copies rather than symlinks: Plasma 6 KPackage rejects symlinks inside a package, the same
# trap that silently broke the Global Theme early on. The directory name must equal KPlugin.Id
# or kpackagetool6 will not find it - validate-packages.py enforces that.
{
  lib,
  runCommand,
  python3,
}:
runCommand "cybou-presence-applet"
  {
    nativeBuildInputs = [ python3 ];
    meta = {
      description = "Cybou Presence applet for Plasma 6";
      license = lib.licenses.mit;
    };
  }
  ''
    dir=$out/share/plasma/plasmoids/org.cybou.presence
    mkdir -p "$dir"
    cp -rL ${./org.cybou.presence}/. "$dir"/
    chmod -R u+w "$dir"
    python3 ${../../scripts/validate-packages.py} "$dir"
    python3 ${../../scripts/validate-qml-api.py} "$dir"
  ''
