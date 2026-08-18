# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Development VM. Not a release artefact - it exists so Gate A can be checked quickly.
{ lib, ... }:
{
  imports = [ ./common.nix ];

  # W2 preview: SDDM offers this as a separate session; Plasma remains the default/fallback until
  # renderer, compositor, input, and recovery gates pass.
  cybou.desktopWeb.enable = true;

  users.users.cybou = {
    isNormalUser = true;
    description = "Cybou test user";
    extraGroups = [
      "wheel"
      "networkmanager"
    ];
    # Development VM only. The ISO must not embed credentials
    # Development-only credential; release images follow docs/RELEASE.md and embed none.
    initialPassword = "cybou";
  };

  # Software rendering, for this VM only.
  #
  # QEMU's emulated bochs-drm exposes /dev/dri/card0 but no render node, so Mesa cannot pick a
  # driver ("failed to get driver name for fd -1", then "ZINK: failed to choose pdev") and KWin
  # never gets an EGL context. The processes stay alive and every unit reports success while
  # nothing renders - the greeter showed only its wallpaper and the session had no panel.
  #
  # KWIN_COMPOSE=Q switches KWin to the QPainter compositor, which draws into dumb buffers and
  # needs no GL; LIBGL_ALWAYS_SOFTWARE stops Mesa looking for hardware.
  #
  # It must be sessionVariables: SDDM starts the greeter as its own PAM session, so neither
  # environment.variables nor the display-manager unit environment reaches it. All three were
  # tested by reading /proc/<kwin pid>/environ; only this one arrives.
  #
  # Deliberately NOT in modules/: real hardware and Hyper-V have a render node, and the product
  # configuration must not carry an emulator workaround.
  # QT_QUICK_BACKEND matters as much as the other two and was missed at first: KWIN_COMPOSE
  # fixes the compositor, so the wallpaper and cursor appear, but SDDM's greeter is a separate
  # Qt Quick application that needs its own software scene graph. Without it the screen shows a
  # background and nothing else - which reads as "the desktop is dead".
  environment.sessionVariables = {
    KWIN_COMPOSE = "Q";
    LIBGL_ALWAYS_SOFTWARE = "1";
    QT_QUICK_BACKEND = "software";
  };

  virtualisation.vmVariant.virtualisation = {
    memorySize = lib.mkDefault 4096;
    cores = lib.mkDefault 4;
    diskSize = lib.mkDefault 16384;

    # Deliberately no `-vga none -device virtio-gpu-pci`. That combination was tried and
    # produced a black window for the whole boot: without a GL-capable host display,
    # virtio-gpu scans out nothing until the guest DRM driver comes up, so there is not even
    # firmware output to look at. The NixOS default (std VGA / bochs-drm) draws from firmware
    # onward and KWin falls back to software rendering, which is slow but visible.
    # Revisit only with a host that can pass GL through.
  };

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = false;

  fileSystems."/" = {
    device = "/dev/disk/by-label/nixos";
    fsType = "ext4";
  };
}
