# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# D-Bus-activated M4 services. They are not eagerly WantedBy the graphical target: the QML
# Presence proxy performs the one-time legacy state migration before Presence1 activation.
#
# Dependencies use Wants+After rather than Requires. A crashed optional organ must not make
# systemd tear down the rest of Mind; M6 will turn that missing process into an explicit
# capability deficit.
{ cybouPackages, ... }:
let
  mind = cybouPackages.cybou-mind;

  mkService =
    {
      description,
      binary,
      busName,
      after ? [ ],
      wants ? [ ],
    }:
    {
      inherit description after wants;
      partOf = [ "graphical-session.target" ];

      serviceConfig = {
        Type = "dbus";
        BusName = busName;
        ExecStart = "${mind}/bin/${binary}";
        Restart = "on-failure";
        RestartSec = "1s";
      };
    };
in
{
  systemd.user.services = {
    cybou-eventd = mkService {
      description = "Cybou durable cognitive event journal";
      binary = "cybou-eventd";
      busName = "org.cybou.Mind.Event1";
    };

    cybou-identityd = mkService {
      description = "Cybou identity continuity organ";
      binary = "cybou-identityd";
      busName = "org.cybou.Mind.Identity1";
      after = [ "cybou-eventd.service" ];
      wants = [ "cybou-eventd.service" ];
    };

    cybou-intentiond = mkService {
      description = "Cybou intentions organ";
      binary = "cybou-intentiond";
      busName = "org.cybou.Mind.Intention1";
      after = [ "cybou-eventd.service" ];
      wants = [ "cybou-eventd.service" ];
    };

    cybou-predictord = mkService {
      description = "Cybou prediction organ";
      binary = "cybou-predictord";
      busName = "org.cybou.Mind.Predictor1";
      after = [ "cybou-eventd.service" ];
      wants = [ "cybou-eventd.service" ];
    };

    cybou-workspaced = mkService {
      description = "Cybou global workspace organ";
      binary = "cybou-workspaced";
      busName = "org.cybou.Mind.Workspace1";
      after = [ "cybou-eventd.service" ];
      wants = [ "cybou-eventd.service" ];
    };

    cybou-selfd = mkService {
      description = "Cybou self-model organ";
      binary = "cybou-selfd";
      busName = "org.cybou.Mind.Self1";
      after = [
        "cybou-eventd.service"
        "cybou-identityd.service"
        "cybou-intentiond.service"
        "cybou-predictord.service"
      ];
      wants = [
        "cybou-eventd.service"
        "cybou-identityd.service"
        "cybou-intentiond.service"
        "cybou-predictord.service"
      ];
    };

    cybou-presenced = mkService {
      description = "Cybou presentation backend";
      binary = "cybou-presenced";
      busName = "org.cybou.Mind.Presence1";
      after = [
        "cybou-eventd.service"
        "cybou-identityd.service"
        "cybou-intentiond.service"
        "cybou-predictord.service"
        "cybou-selfd.service"
        "cybou-workspaced.service"
      ];
      wants = [
        "cybou-eventd.service"
        "cybou-identityd.service"
        "cybou-intentiond.service"
        "cybou-predictord.service"
        "cybou-selfd.service"
        "cybou-workspaced.service"
      ];
    };
  };
}
