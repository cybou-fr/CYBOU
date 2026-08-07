# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

{ stdenv, lib }:

stdenv.mkDerivation {
  pname = "cybou-layout-templates";
  version = "0.1.0";

  src = ./.;

  phases = "installPhase";

  installPhase = ''
    mkdir -p $out/share/plasma/layout-templates
    cp -r "$src/org.cybou.plasma.minddock" "$out/share/plasma/layout-templates/"
  '';

  meta = {
    description = "Cybou layout templates for Plasma";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}
