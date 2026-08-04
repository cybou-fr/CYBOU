# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Cybou live ISO.
#
# ADR-0005: Calamares from the upstream NixOS graphical installation-CD profile. Cybou inherits
# the installer rather than forking it, and branding is capped at product name, Aperture logo and
# token colours - deep installer theming is optional and must not block the release.
#
# The upstream profile already brings Plasma 6, SDDM and Calamares, so this file adds the Cybou
# layer on top instead of importing systems/common.nix, which would set the same options a
# second time.
{
  config,
  lib,
  modulesPath,
  ...
}:
{
  imports = [
    "${modulesPath}/installer/cd-dvd/installation-cd-graphical-calamares-plasma6.nix"
    ../modules/branding.nix
  ];

  networking.hostName = "cybou";

  # The installer profile pulls in ZFS; 26.11 makes false the default and warns until then.
  boot.zfs.forceImportRoot = false;

  # Product identity in the places a live system shows it.
  system.nixos.distroName = "Cybou";
  system.nixos.distroId = "cybou";

  isoImage = {
    # isoBaseName, not image.fileName: the installation-CD profile composes the final name
    # from this base, and setting the composed name instead is silently overridden - the
    # first build came out as nixos-plasma6-*.iso despite image.fileName evaluating to cybou-*.
    isoBaseName = lib.mkForce "cybou";
    volumeID = "CYBOU";
    # Must boot without a network after it is built (docs/06).
    squashfsCompression = "zstd -Xcompression-level 6";
  };

  # No embedded credentials: spec/acceptance.yaml treats that as blocking. The live user comes
  # from the upstream profile with no password; nothing is added here.

  # Same software-rendering workaround as the VM and Hyper-V images, and for the same reason:
  # a live ISO is usually started in a virtual machine or on unaccelerated hardware, and KWin
  # with no render node draws a background and nothing else. Documented in systems/vm.nix.
  environment.sessionVariables = {
    KWIN_COMPOSE = "Q";
    LIBGL_ALWAYS_SOFTWARE = "1";
    QT_QUICK_BACKEND = "software";
  };
}
