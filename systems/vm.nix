# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Development VM. Not a release artefact - it exists so Gate A can be checked quickly.
{ lib, ... }:
{
  imports = [ ./common.nix ];

  users.users.cybou = {
    isNormalUser = true;
    description = "Cybou test user";
    extraGroups = [
      "wheel"
      "networkmanager"
    ];
    # Development VM only. The ISO must not embed credentials
    # (spec/acceptance.yaml: no_embedded_credentials).
    initialPassword = "cybou";
  };

  virtualisation.vmVariant.virtualisation = {
    memorySize = lib.mkDefault 4096;
    cores = lib.mkDefault 4;
    diskSize = lib.mkDefault 16384;
    # Plasma 6 on Wayland needs a GPU the guest can actually use.
    qemu.options = [
      "-vga none"
      "-device virtio-gpu-pci"
    ];
  };

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = false;

  fileSystems."/" = {
    device = "/dev/disk/by-label/nixos";
    fsType = "ext4";
  };
}
