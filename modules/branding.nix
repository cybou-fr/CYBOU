# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Installs the Cybou theme and makes it the system default.
{ cybouPackages, ... }:
{
  environment.systemPackages = [
    cybouPackages.cybou-theme
    cybouPackages.cybou-branding
  ];

  # System-wide defaults only. KDE reads /etc/xdg/kdeglobals as a fallback beneath the user's
  # own ~/.config/kdeglobals, so a fresh account starts on Cybou Horizon while anyone who has
  # already chosen something keeps it. This is the "provide system defaults where KDE supports
  # them" step from docs/02-architecture.md - not the first-login initializer, which is
  # CYB-023 and may only run once, against a clean profile.
  environment.etc."xdg/kdeglobals".text = ''
    [KDE]
    LookAndFeelPackage=org.cybou.horizon.desktop
    widgetStyle=Breeze

    [General]
    ColorScheme=CybouHorizonDark
  '';
}
