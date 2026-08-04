# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# The build expression is code (MIT); the wallpapers it produces are design assets
# (CC-BY-SA-4.0), which is what meta.license records.
{
  lib,
  runCommand,
  python3,
}:
runCommand "cybou-horizon-wallpaper"
  {
    nativeBuildInputs = [ python3 ];
    meta = {
      description = "Cybou Horizon Field wallpapers";
      license = lib.licenses.cc-by-sa-40;
    };
  }
  ''
    mkdir -p $out/share/wallpapers
    python3 ${../../scripts/generate-wallpaper.py} ${../../spec/design-tokens.json} \
      $out/share/wallpapers
  ''
