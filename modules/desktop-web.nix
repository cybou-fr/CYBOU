# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

{
  config,
  cybouPackages,
  lib,
  ...
}:
let
  cfg = config.cybou.desktopWeb;
in
{
  options.cybou.desktopWeb = {
    enable = lib.mkEnableOption "the opt-in single-surface Cybou web desktop session";

    package = lib.mkOption {
      type = lib.types.package;
      default = cybouPackages.cybou-desktop-shell;
      description = "Desktop session package containing the compositor and Chromium launcher.";
    };

    frontendPackage = lib.mkOption {
      type = lib.types.package;
      default = cybouPackages.cybou-web-ui;
      description = "Immutable Living Canvas package served by the local gateway.";
    };
  };

  config = lib.mkIf cfg.enable {
    cybou.webGateway = {
      enable = true;
      frontendPackage = cfg.frontendPackage;
    };

    services.displayManager.sessionPackages = [ cfg.package ];
    environment.systemPackages = [ cfg.package ];
  };
}
