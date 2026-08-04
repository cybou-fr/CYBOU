# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Cybou Horizon window decoration (Aurorae).
#
# NOT VISUALLY VERIFIED. The build proves every file is present and every SVG parses; it cannot
# prove that KWin stretches the nine frame pieces correctly, and a wrong element id produces a
# misdrawn titlebar rather than an error. Treat CYB-015 as open until it has been looked at.
#
# The button glyphs are generated from the tokens so hover colours cannot drift from the
# palette: close turns danger-coloured on hover, everything else brightens (docs/04).
{
  lib,
  runCommand,
  python3,
}:
runCommand "cybou-horizon-aurorae"
  {
    nativeBuildInputs = [ python3 ];
    meta = {
      description = "Cybou Horizon Aurorae window decoration";
      license = lib.licenses.cc-by-sa-40;
    };
  }
  ''
    dir=$out/share/aurorae/themes/CybouHorizon
    install -Dm444 ${./metadata.desktop} $dir/metadata.desktop
    install -Dm444 ${./CybouHorizonrc} $dir/CybouHorizonrc
    install -Dm444 ${./decoration.svg} $dir/decoration.svg
    python3 ${./mkbuttons.py} ${../../spec/design-tokens.json} "$dir"

    # Every button the layout references must exist, or KWin draws a gap where the control
    # should be. Fail the build instead.
    for b in close minimize maximize restore; do
      [ -f "$dir/$b.svg" ] || { echo "missing button: $b.svg" >&2; exit 1; }
    done
  ''
