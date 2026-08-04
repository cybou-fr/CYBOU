# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Hyper-V development image. Produces a VHDX that boots straight to the desktop, so the
# desktop can be looked at and clicked without an installer.
#
# Not a release artefact: the ISO (Phase 6) remains the thing users install.
{ modulesPath, ... }:
{
  imports = [
    ./common.nix
    "${modulesPath}/virtualisation/hyperv-image.nix"
  ];

  virtualisation.hypervGuest.enable = true;
  virtualisation.diskSize = 32768; # MiB; the Plasma closure is large

  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = false;

  users.users.cybou = {
    isNormalUser = true;
    description = "Cybou test user";
    extraGroups = [
      "wheel"
      "networkmanager"
    ];
    # Development image only. The ISO must not embed credentials
    # (spec/acceptance.yaml: no_embedded_credentials).
    initialPassword = "cybou";
  };

  # Software rendering, same reason as systems/vm.nix.
  #
  # Hyper-V Gen 2 gives a Linux guest no GPU: video arrives over hyperv_drm, which like QEMU's
  # bochs-drm exposes a card node but no render node. KWin therefore cannot create an EGL
  # context and renders nothing while every process and unit reports success. Moving from QEMU
  # to Hyper-V does not avoid this; it only changes which emulated GPU is missing the node.
  #
  # sessionVariables specifically: SDDM runs its greeter as a separate PAM session, so neither
  # environment.variables nor the display-manager unit environment reaches it. Verified by
  # reading /proc/<kwin pid>/environ under all three mechanisms.
  # QT_QUICK_BACKEND is the one that was missing at first. KWIN_COMPOSE fixes the compositor,
  # which is why the wallpaper and the mouse cursor appeared - but SDDM's greeter is a separate
  # Qt Quick application, and it needs its own software scene graph. Without it the compositor
  # draws a background and nothing else, which looks exactly like a dead desktop.
  environment.sessionVariables = {
    KWIN_COMPOSE = "Q";
    LIBGL_ALWAYS_SOFTWARE = "1";
    QT_QUICK_BACKEND = "software";
  };

  # So this image can be inspected without rebuilding and re-copying 11 GB for every
  # hypothesis. Development image only - the ISO ships no password authentication.
  services.openssh = {
    enable = true;
    settings.PasswordAuthentication = true;
    settings.PermitRootLogin = "no";
  };
}
