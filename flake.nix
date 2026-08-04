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
      # `nix fmt` passes no path when invoked bare, and nixfmt then reads stdin and fails.
      # Wrap it so both `nix fmt` and `nix fmt path/` work. In 26.05 `nixfmt-rfc-style`
      # is a deprecated alias for `nixfmt`.
      formatter = forAllSystems (
        pkgs:
        pkgs.writeShellApplication {
          name = "cybou-fmt";
          runtimeInputs = [
            pkgs.nixfmt
            pkgs.findutils
          ];
          text = ''
            if [ "$#" -eq 0 ]; then set -- .; fi
            find "$@" -type f -name '*.nix' -print0 | xargs -0 --no-run-if-empty nixfmt
          '';
        }
      );

      # Phase 0 ships empty derivations on purpose: the build interface exists and is
      # checked before any visual work starts (docs/07-implementation-plan.md).
      packages = forAllSystems (pkgs: rec {
        horizon-colors = pkgs.callPackage ./packages/horizon-colors { };
        horizon-wallpaper = pkgs.callPackage ./packages/horizon-wallpaper { };
        horizon-assets = pkgs.callPackage ./packages/horizon-assets { };

        horizon-global-theme = pkgs.callPackage ./packages/horizon-global-theme { };

        # Copies rather than symlinkJoin, and not as a matter of taste: Plasma 6 KPackage
        # rejects symlinks inside a theme package, so a symlink farm produces a Global Theme
        # that silently fails to load. checks.package-metadata caught exactly that.
        cybou-theme = pkgs.runCommand "cybou-theme" { } ''
          mkdir -p $out
          for p in ${horizon-colors} ${horizon-wallpaper} ${horizon-global-theme}; do
            cp -rL "$p"/. $out/
            # Store files arrive read-only; without this the next package cannot be
            # merged into the directories the previous one created.
            chmod -R u+w $out
          done
        '';

        cybou-branding = pkgs.runCommand "cybou-branding" { } ''
          mkdir -p $out
          cp -rL ${horizon-assets}/. $out/
          chmod -R u+w $out
        '';
        # `rec` rather than `self.packages.${pkgs.system}`: `pkgs.system` is deprecated
        # in favour of `stdenv.hostPlatform.system` and warns during evaluation.
        default = cybou-theme;
      });

      nixosConfigurations = {
        cybou-vm = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [ ./systems/vm.nix ];
        };

        # Development image for Hyper-V; build system.build.hypervImage.
        cybou-hyperv = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [ ./systems/hyperv.nix ];
        };
      };

      # `nix develop` gives the tools the checks use, so a failing check can be reproduced
      # by hand instead of only inside the build sandbox.
      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = [
            # pyyaml included: the spec package carries YAML that CI parses, and without it
            # the check can only be run by pulling a one-off shell.
            (pkgs.python3.withPackages (ps: [ ps.pyyaml ]))
            pkgs.nixfmt
            pkgs.reuse
          ];
        };
      });

      checks = forAllSystems (pkgs: {
        formatting =
          pkgs.runCommand "check-formatting"
            {
              nativeBuildInputs = [
                pkgs.nixfmt
                pkgs.findutils
              ];
            }
            ''
              find ${src} -type f -name '*.nix' -print0 \
                | xargs -0 --no-run-if-empty nixfmt --check
              touch $out
            '';

        # Licence compliance (ADR-0007). Turns the Gate D "licence manifest" from a
        # hand-maintained checkbox into a machine-verified artifact.
        reuse = pkgs.runCommand "check-reuse" { nativeBuildInputs = [ pkgs.reuse ]; } ''
          cd ${src}
          reuse lint
          touch $out
        '';

        # Gate A, service level. Heavy: boots a full Plasma VM under KVM. CI runs the three
        # cheap checks on every push and the whole set on a tag - see .github/workflows.
        vm-smoke = import ./tests/vm-smoke.nix { inherit pkgs; };

        # Static KDE package validation. Catches the Gate B failures - wrong metadata
        # file name, ID/directory mismatch, wrong layout script name, symlinks,
        # malformed SVG, TBD licences - without needing a Plasma session.
        # Runs against the *built* theme, not the source tree: the thing that has to be a
        # valid KDE package is what gets installed, and packages/ holds Nix expressions.
        package-metadata =
          pkgs.runCommand "check-package-metadata" { nativeBuildInputs = [ pkgs.python3 ]; }
            ''
              python3 ${src}/scripts/validate-packages.py \
                ${self.packages.${pkgs.stdenv.hostPlatform.system}.cybou-theme}/share
              touch $out
            '';
      });
    };
}
