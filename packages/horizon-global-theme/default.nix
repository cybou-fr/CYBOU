# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Cybou Horizon Global Theme (KPackage, Plasma/LookAndFeel).
#
# Format checked against plasma-workspace/lookandfeel/org.kde.breezedark, not against a blog:
#   * the metadata file is metadata.json, never manifest.json
#   * the directory name must equal KPlugin.Id
#   * the desktop layout script, when it arrives in Phase 4, must be named
#     contents/layouts/org.kde.plasma.desktop-layout.js - Plasma ignores any other name
#     and fails silently, with no panel and no error
#
# What `defaults` still points at Breeze, and why: the Plasma style (CYB-013), the window
# decoration (CYB-015) and the splash (CYB-018) are Phase 3 work that has not landed. Naming a
# package that does not exist yet would leave a user with a half-applied theme, which is worse
# than an honest Breeze fallback. The colour scheme and wallpaper are already ours.
{
  lib,
  runCommand,
}:
let
  id = "org.cybou.horizon.desktop";
in
runCommand "cybou-horizon-global-theme"
  {
    meta = {
      description = "Cybou Horizon Global Theme for Plasma 6";
      license = lib.licenses.cc-by-sa-40;
    };
  }
  ''
    dir=$out/share/plasma/look-and-feel/${id}
    install -Dm444 ${./metadata.json} $dir/metadata.json
    install -Dm444 ${./defaults} $dir/contents/defaults
  ''
