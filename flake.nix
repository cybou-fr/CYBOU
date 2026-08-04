# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
{
  description = "Cybou - a calm, reproducible KDE Plasma desktop on NixOS";

  inputs = {
    # Frozen base: NixOS 26.05 stable (AGENTS.md). Do not move to unstable without
    # an ADR recording the blocker.
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
  };

  outputs =
    { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      src = nixpkgs.lib.cleanSource ./.;
    in
    {
      formatter = forAllSystems (pkgs: pkgs.nixfmt-rfc-style);

      # Phase 0 ships empty derivations on purpose: the build interface exists and is
      # checked before any visual work starts (docs/07-implementation-plan.md).
      packages = forAllSystems (pkgs: {
        cybou-theme = pkgs.runCommand "cybou-theme" { } "mkdir -p $out";
        cybou-branding = pkgs.runCommand "cybou-branding" { } "mkdir -p $out";
        default = self.packages.${pkgs.system}.cybou-theme;
      });

      checks = forAllSystems (pkgs: {
        formatting =
          pkgs.runCommand "check-formatting" { nativeBuildInputs = [ pkgs.nixfmt-rfc-style ]; }
            ''
              nixfmt --check ${src}
              touch $out
            '';

        # Static KDE package validation. Catches the Gate B failures - wrong metadata
        # file name, ID/directory mismatch, wrong layout script name, symlinks,
        # malformed SVG, TBD licences - without needing a Plasma session.
        package-metadata =
          pkgs.runCommand "check-package-metadata" { nativeBuildInputs = [ pkgs.python3 ]; }
            ''
              if [ -d ${src}/packages ]; then
                python3 ${src}/scripts/validate-packages.py ${src}/packages
              else
                echo "no packages/ yet - Phase 0"
              fi
              touch $out
            '';
      });
    };
}
