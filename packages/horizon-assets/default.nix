# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# The build expression is code (MIT); the mark it installs is a design asset
# (CC-BY-SA-4.0), recorded in meta.license.
#
# Installation paths follow docs/05-plasma-packaging.md "Shared assets".
{
  lib,
  runCommand,
}:
runCommand "cybou-horizon-assets"
  {
    meta = {
      description = "Cybou Aperture mark and shared branding assets";
      license = lib.licenses.cc-by-sa-40;
    };
  }
  ''
    install -Dm444 ${./cybou-aperture.svg} $out/share/cybou/branding/cybou-aperture.svg
    install -Dm444 ${./cybou-aperture.svg} $out/share/icons/hicolor/scalable/apps/cybou.svg
    install -Dm444 ${./cybou-aperture.svg} $out/share/pixmaps/cybou.svg
    install -Dm444 ${./cybou-aperture-light.svg} $out/share/cybou/branding/cybou-aperture-light.svg
  ''
