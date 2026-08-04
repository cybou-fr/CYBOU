# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# The build expression is code and therefore MIT; the colour schemes it produces are design
# assets under CC-BY-SA-4.0, which is what `meta.license` below records.
#
# Cybou Horizon colour schemes, generated from spec/design-tokens.json at build time.
# Nothing here is hand-written: the tokens file is authoritative, and a checked-in .colors
# would drift from it without anything noticing.
{
  lib,
  runCommand,
  python3,
}:
runCommand "cybou-horizon-colors"
  {
    nativeBuildInputs = [ python3 ];
    meta = {
      description = "Cybou Horizon colour schemes for KDE Plasma";
      license = lib.licenses.cc-by-sa-40;
    };
  }
  ''
    mkdir -p $out/share/color-schemes
    python3 ${../../scripts/generate-colors.py} ${../../spec/design-tokens.json} $out/share/color-schemes
  ''
