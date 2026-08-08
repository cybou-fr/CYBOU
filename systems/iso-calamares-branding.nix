# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Calamares branding for the live ISO (CYB-040).
#
# ADR-0005 caps this at product name, Aperture logo and token colours.
#
# What NOT to do here, learned by doing it: a blanket "replace nixos with cybou" across
# branding.desc rewrites `componentName`, which Calamares uses to select the branding at all,
# and turns every upstream URL into an invented one - docs/security/README.md forbids
# inventing a website. A blanket "copy our SVG over every image" writes XML into .png files,
# which render as nothing.
#
# The product name needs no patching whatsoever: branding.desc reads `productName: "${NAME}"`,
# substituted at runtime from os-release, which `system.nixos.distroName` already sets to Cybou.
{ cybouPackages, ... }:
{
  nixpkgs.overlays = [
    (final: prev: {
      calamares-nixos-extensions = prev.calamares-nixos-extensions.overrideAttrs (old: {
        postInstall = (old.postInstall or "") + ''
          branding=$out/share/calamares/branding/nixos
          desc="$branding/branding.desc"
          if [ ! -f "$desc" ]; then
            echo "calamares branding.desc not found at $desc" >&2
            ls -R "$out/share/calamares" >&2
            exit 1
          fi

          # The icon and the welcome image are SVG upstream, so an SVG replacement keeps the
          # format. white.png and the gfx-landing-*.png files are left alone: they are raster,
          # and writing SVG into them produces a file that renders as nothing.
          cp -f ${cybouPackages.cybou-branding}/share/cybou/branding/cybou-aperture.svg \
            "$branding/nix-snowflake.svg"

          # productLogo is white.png, a raster file shown in the sidebar. Writing SVG into a
          # .png renders as nothing, so the mark is rasterised properly instead. 256 px keeps
          # it sharp at the sizes Calamares uses.
          ${final.resvg}/bin/resvg --width 256 --height 256 \
            ${cybouPackages.cybou-branding}/share/cybou/branding/cybou-aperture.svg \
            "$branding/white.png"

          # The welcome text uses versionedName, not productName - which is why the installer
          # still said "Welcome to the NixOS installer" while the window title was already
          # Cybou. Only the product-name strings are touched; componentName and every URL are
          # left exactly as upstream wrote them.
          substituteInPlace "$desc" \
            --replace-quiet 'versionedName:       NixOS' \
                            'versionedName:       Cybou' \
            --replace-quiet 'shortVersionedName:  NixOS' \
                            'shortVersionedName:  Cybou' \
            --replace-quiet 'shortProductName:    NixOS' \
                            'shortProductName:    Cybou' \
            --replace-quiet 'bootloaderEntryName: NixOS' \
                            'bootloaderEntryName: Cybou'

          # Sidebar colours from spec/design-tokens.json: surface, text, canvas, accent.
          substituteInPlace "$desc" \
            --replace-quiet '"#5277C3"' '"#171D27"' \
            --replace-quiet '"#7EBAE4"' '"#70E1C8"' \
            --replace-quiet '"#292F34"' '"#0A0D12"' \
            --replace-quiet '"#FFFFFF"' '"#F2F5F8"'

          # componentName selects the branding; renaming it detaches Calamares from this
          # directory. Untouched on purpose. So are the upstream URLs: no real Cybou site
          # exists yet, and an invented one is worse than an upstream one.
          grep -q '^componentName: *nixos' "$desc" || {
            echo "componentName is no longer nixos - branding selection would break" >&2
            exit 1
          }
        '';
      });
    })
  ];
}
