# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Cybou Horizon Plasma Style (KPackage, Plasma/Theme).
#
# Ships a `colors` file and nothing else on purpose. Plasma tints its own widget SVGs from
# that file, so the panel, menus and applets follow the tokens without Cybou overriding a
# single SVG - and docs/05-plasma-packaging.md says to override only what is intentional and
# to let Plasma fall back for the rest. Overridden SVGs come with CYB-013's later passes,
# once there is a reason for each one.
#
# Naming note: the directory is `CybouHorizon` and KPlugin.Id matches it, because Plasma
# references a style by directory name in plasmarc. spec/theme-manifest.yaml records the id as
# `org.cybou.horizon.plasma`; that reverse-domain form is right for the Global Theme but does
# not work here. The spec needs reconciling - see the note added to docs/05.
{
  lib,
  runCommand,
  python3,
}:
runCommand "cybou-horizon-plasma-style"
  {
    nativeBuildInputs = [ python3 ];
    meta = {
      description = "Cybou Horizon Plasma Style";
      license = lib.licenses.cc-by-sa-40;
    };
  }
  ''
    dir=$out/share/plasma/desktoptheme/CybouHorizon
    install -Dm444 ${./metadata.json} $dir/metadata.json
    python3 ${../../scripts/generate-colors.py} ${../../spec/design-tokens.json} \
      "$(mktemp -d)" "$dir"
  ''
