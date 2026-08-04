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

  # image.baseName and image.fileName are the current names; isoImage.isoBaseName and
  # isoImage.isoName are renamed aliases that warn.
  #
  # The file inside the output is composed from baseName alone and comes out as cybou.iso;
  # fileName is metadata and does not rename it. Three attempts to force a versioned filename
  # failed, so the version lives where it is reliable: in the store path, and in the versioned
  # copy the release step publishes next to the SHA256. Do not spend a fourth attempt here.
  image.baseName = lib.mkForce "cybou";
  image.fileName = lib.mkForce "cybou-${config.system.nixos.label}-x86_64-linux.iso";

  isoImage = {
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
