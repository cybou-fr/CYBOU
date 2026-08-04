# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# KDE Plasma 6 on Wayland with SDDM. Phase 1 is deliberately unbranded: Gate A must pass
# on a plain Plasma session before any Cybou theme is introduced (AGENTS.md).
{ pkgs, ... }:
{
  services.displayManager.sddm = {
    enable = true;
    wayland.enable = true;
  };

  services.desktopManager.plasma6.enable = true;

  # Wayland session only. X11 applications still run through XWayland.
  services.xserver.enable = false;

  xdg.portal.enable = true;

  environment.systemPackages = with pkgs; [
    # Curated set from docs/01-product-scope.md. Kept small on purpose:
    # the ISO is not a software collection.
    kdePackages.dolphin
    kdePackages.konsole
    kdePackages.kate
    kdePackages.ark
    kdePackages.okular
    kdePackages.gwenview
    kdePackages.kcalc
    kdePackages.spectacle
    kdePackages.plasma-systemmonitor
    firefox
  ];

  # Plasma ships a broad default set; drop what the curated list does not need.
  environment.plasma6.excludePackages = with pkgs.kdePackages; [
    elisa
    khelpcenter
  ];
}
