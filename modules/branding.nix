# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Installs the Cybou theme and makes it the system default.
{ cybouPackages, ... }:
{
  environment.systemPackages = [
    cybouPackages.cybou-theme
    cybouPackages.cybou-branding
    cybouPackages.horizon-sddm
    cybouPackages.cybou-tools

    # The Mind and its panel. The QML module has to be in the system environment because
    # plasmashell resolves org.cybou.presence through QML2_IMPORT_PATH, which NixOS builds
    # from systemPackages - a user-level install would leave the applet blank.
    cybouPackages.cybou-mind
    cybouPackages.cybou-presence-applet
  ];

  # SDDM has its own theme directory and does not follow the Global Theme, which is why the
  # login screen stayed Breeze while the desktop was already Cybou.
  services.displayManager.sddm.theme = "cybou-horizon";

  # System-wide defaults only. KDE reads /etc/xdg/kdeglobals as a fallback beneath the user's
  # own ~/.config/kdeglobals, so a fresh account starts on Cybou Horizon while anyone who has
  # already chosen something keeps it. This is the "provide system defaults where KDE supports
  # them" step from docs/02-architecture.md - not the first-login initializer, which is
  # CYB-023 and may only run once, against a clean profile.
  # First login, once per profile (CYB-023). Deliberately not ordered Before= anything in the
  # session: docs/04 requires that a failed initializer must not block login, so it runs
  # alongside the session and exits 0 in every path.
  systemd.user.services.cybou-first-login = {
    description = "Apply the Cybou desktop on a clean profile";
    wantedBy = [ "graphical-session.target" ];
    partOf = [ "graphical-session.target" ];
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${cybouPackages.cybou-tools}/bin/cybou-first-login";
      # A failure here is logged and ignored; the session is worth more than the theme.
      SuccessExitStatus = "0 1";
    };
  };

  # Four virtual desktops (docs/04). Not settable from the layout script: KWin owns this,
  # and the layout script owns the panel.
  # Switching shortcuts for those desktops (CYB-021). Meta+1..4 and nothing else invented:
  # docs/04 forbids a large custom scheme in v0.1, so every other binding stays at the KDE
  # default. These are system defaults, so a user who rebinds them keeps their choice.
  environment.etc."xdg/kglobalshortcutsrc".text = ''
    [kwin]
    Switch to Desktop 1=Meta+1,Ctrl+F1,Switch to Desktop 1
    Switch to Desktop 2=Meta+2,Ctrl+F2,Switch to Desktop 2
    Switch to Desktop 3=Meta+3,Ctrl+F3,Switch to Desktop 3
    Switch to Desktop 4=Meta+4,Ctrl+F4,Switch to Desktop 4
    Overview=Meta+W,Meta+W,Toggle Overview

    [org.kde.spectacle.desktop]
    _launch=Print,Print,Launch Screenshot Tool
  '';

  environment.etc."xdg/kwinrc".text = ''
    [Desktops]
    Number=4
    Rows=1
  '';

  environment.etc."xdg/kdeglobals".text = ''
    [KDE]
    LookAndFeelPackage=org.cybou.horizon.desktop
    widgetStyle=Breeze

    [General]
    ColorScheme=CybouHorizonDark
  '';
}
