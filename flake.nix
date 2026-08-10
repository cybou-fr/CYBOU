# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
{
  description = "Cybou - a calm, reproducible KDE Plasma desktop on NixOS";

  inputs = {
    # Frozen base: NixOS 26.05 stable (ADR-0006). Do not move to unstable without
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

      # Package boundaries remain explicit so each visual/runtime output can be validated
      # independently (docs/ARCHITECTURE.md and docs/TESTING.md).
      packages = forAllSystems (pkgs: rec {
        horizon-colors = pkgs.callPackage ./packages/horizon-colors { };
        horizon-wallpaper = pkgs.callPackage ./packages/horizon-wallpaper { };
        horizon-assets = pkgs.callPackage ./packages/horizon-assets { };

        horizon-global-theme = pkgs.callPackage ./packages/horizon-global-theme { inherit horizon-assets; };
        horizon-plasma-style = pkgs.callPackage ./packages/horizon-plasma-style { };
        horizon-sddm = pkgs.callPackage ./packages/horizon-sddm { inherit horizon-wallpaper; };
        horizon-aurorae = pkgs.callPackage ./packages/horizon-aurorae { };
        cybou-tools = pkgs.callPackage ./packages/cybou-tools { };

        # The Mind and the panel that shows it. Separate derivations: ADR-0008 keeps the
        # cognitive code isolated, and the applet is data that must not force a C++ rebuild.
        cybou-mind = pkgs.callPackage ./packages/cybou-mind { };
        cybou-presence-applet = pkgs.callPackage ./packages/cybou-presence-applet { };
        cybou-layout-templates = pkgs.callPackage ./packages/cybou-layout-templates { };

        # Copies rather than symlinkJoin, and not as a matter of taste: Plasma 6 KPackage
        # rejects symlinks inside a theme package, so a symlink farm produces a Global Theme
        # that silently fails to load. checks.package-metadata caught exactly that.
        cybou-theme = pkgs.runCommand "cybou-theme" { } ''
          mkdir -p $out
          for p in ${horizon-colors} ${horizon-wallpaper} ${horizon-global-theme} \
                   ${horizon-plasma-style} ${horizon-aurorae}; do
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
          specialArgs.cybouPackages = self.packages.x86_64-linux;
          modules = [ ./systems/vm.nix ];
        };

        # Live ISO; build system.build.isoImage (ADR-0005).
        cybou-iso = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          specialArgs.cybouPackages = self.packages.x86_64-linux;
          modules = [ ./systems/iso.nix ];
        };

        # Development image for Hyper-V; build system.build.hypervImage.
        cybou-hyperv = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          specialArgs.cybouPackages = self.packages.x86_64-linux;
          modules = [ ./systems/hyperv.nix ];
        };
      };

      # `nix develop` gives the tools the checks use, so a failing check can be reproduced
      # by hand instead of only inside the build sandbox.
      devShells = forAllSystems (pkgs: {
        # Everything the project is built with, pinned by flake.lock. Nothing is installed by
        # hand - not Qt, not a compiler, not on Windows and not in WSL (ADR-0008).
        default = pkgs.mkShell {
          packages = [
            # pyyaml: the spec package carries YAML that CI parses.
            (pkgs.python3.withPackages (ps: [ ps.pyyaml ]))
            pkgs.nixfmt
            pkgs.reuse

            # C++ toolchain for mind/ (ADR-0008, docs/BUILDING.md).
            pkgs.clang-tools
            pkgs.cmake
            pkgs.ninja
            pkgs.gdb
            pkgs.pkg-config
            pkgs.dbus

            # Qt 6. qtdeclarative brings QML; qttools brings Designer and the profiler.
            pkgs.qt6.qtbase
            pkgs.qt6.qtdeclarative
            pkgs.qt6.qtsvg
            pkgs.qt6.qttools
            pkgs.qt6.qtwayland

            # KDE frameworks the Presence surface needs.
            pkgs.kdePackages.kirigami
            pkgs.kdePackages.kcoreaddons
            pkgs.kdePackages.libplasma

            # Optional, heavy, and worth having: Qt Creator opens through WSLg for QML work.
            pkgs.qtcreator
          ];

          shellHook = ''
            echo "cybou dev shell ready: $(cmake --version | head -1)"
          '';
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
        vm-smoke = import ./tests/vm-smoke.nix {
          inherit pkgs;
          cybouPackages = self.packages.${pkgs.stdenv.hostPlatform.system};
        };

        # Focused P4 gate: one Plasma node proves shell recreation cannot mutate an
        # active lifecycle run or append a duplicate Event1 contribution.
        p4-plasma-lifecycle = import ./tests/p4-plasma-lifecycle.nix {
          inherit pkgs;
          cybouPackages = self.packages.${pkgs.stdenv.hostPlatform.system};
        };

        # Focused P2/P3 gate: one headless node proves identity/run continuity and
        # split-commit idempotency across real reboots without the Plasma smoke-test cost.
        lifecycle-continuity = import ./tests/lifecycle-continuity.nix {
          inherit pkgs;
          cybouPackages = self.packages.${pkgs.stdenv.hostPlatform.system};
        };

        # Focused M6 gate: Plasma remains present while D-Bus activation, timeout recovery,
        # and required Event1 loss are exercised through installed user services.
        m6-recovery-boundary = import ./tests/m6-recovery-boundary.nix {
          inherit pkgs;
          cybouPackages = self.packages.${pkgs.stdenv.hostPlatform.system};
        };

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

        cognitive-docs = pkgs.runCommand "check-cognitive-docs" { nativeBuildInputs = [ pkgs.python3 ]; } ''
          python3 ${src}/scripts/validate-cognitive-docs.py ${src}
          touch $out
        '';

        mind-access = pkgs.runCommand "check-mind-access" { nativeBuildInputs = [ pkgs.python3 ]; } ''
          python3 ${src}/scripts/validate-mind-access.py \
            ${src}/packages/cybou-presence-applet/org.cybou.presence \
            ${src}/packages/cybou-presence-applet/org.cybou.mindhandle \
            ${src}/packages/cybou-layout-templates/org.cybou.plasma.minddock/contents/layout.js
          touch $out
        '';

        qml-api = pkgs.runCommand "check-qml-api" { nativeBuildInputs = [ pkgs.python3 ]; } ''
          python3 ${src}/scripts/validate-qml-api.py \
            ${src}/packages/cybou-presence-applet/org.cybou.presence
          touch $out
        '';

        ui-polish = pkgs.runCommand "check-ui-polish" { nativeBuildInputs = [ pkgs.python3 ]; } ''
          python3 ${src}/scripts/validate-ui-polish.py \
            ${src}/packages/cybou-presence-applet/org.cybou.presence \
            ${src}/packages/cybou-presence-applet/org.cybou.mindhandle
          touch $out
        '';
      });
    };
}
