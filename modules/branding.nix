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
