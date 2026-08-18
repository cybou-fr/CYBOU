# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

{
  config,
  cybouPackages,
  lib,
  ...
}:
let
  cfg = config.cybou.webGateway;
in
{
  options.cybou.webGateway = {
    enable = lib.mkEnableOption "the opt-in read-only Cybou web gateway";

    package = lib.mkOption {
      type = lib.types.package;
      default = cybouPackages.cybou-web-gateway;
      description = "Package containing the cybou-web-gateway executable.";
    };

    frontendPackage = lib.mkOption {
      type = lib.types.package;
      default = cybouPackages.cybou-web-ui;
      description = "Immutable frontend package served from the gateway origin.";
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.user.services.cybou-web-gateway = {
      description = "Cybou read-only local web gateway";
      after = [ "cybou-presenced.service" ];
      wants = [ "cybou-presenced.service" ];
      wantedBy = [ "graphical-session.target" ];
      partOf = [ "graphical-session.target" ];

      serviceConfig = {
        ExecStart = "${cfg.package}/bin/cybou-web-gateway";
        Environment = "CYBOU_WEB_ROOT=${cfg.frontendPackage}/share/cybou/web";
        Restart = "on-failure";
        RestartSec = "1s";
        NoNewPrivileges = true;
        MemoryMax = "256M";
        TasksMax = 64;
        RestrictAddressFamilies = [
          "AF_INET"
          "AF_UNIX"
        ];
        SystemCallArchitectures = "native";
        SystemCallFilter = [
          "@system-service"
          "~@privileged"
          "~@resources"
        ];
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;
      };
    };
  };
}
