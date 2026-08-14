# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# D-Bus-activated Mind services. They are not eagerly WantedBy the graphical target: the QML
# Presence proxy performs the one-time legacy state migration before Presence1 activation.
#
# Dependencies use Wants+After rather than Requires. A crashed optional organ must not make
# systemd tear down the rest of Mind; healthd turns that missing process into an explicit
# capability deficit.
{ cybouPackages, ... }:
let
  mind = cybouPackages.cybou-mind;

  # Shared hardening for every Mind daemon. This reduces what a compromised Mind process can reach.
  # It is deliberately NOT a fix for the open same-user D-Bus question: another process in the same
  # session is a peer, not a child, so none of these directives constrain it. Recording that here
  # keeps this from being counted as progress on the authorization boundary.
  #
  # Everything below is enforced through seccomp, rlimits, or the no-new-privileges bit, all of
  # which apply to unprivileged `systemd --user` units. Namespace-based options - ProtectSystem,
  # PrivateTmp, ProtectHome, PrivateNetwork - are omitted on purpose: they need privileges a user
  # manager does not reliably have, and a directive that fails to apply is worse than an absent one
  # because it reads as protection that is not there. ProtectHome would also be wrong regardless,
  # since the canonical Journal lives under $XDG_STATE_HOME in the user's home directory.
  hardening = {
    NoNewPrivileges = true;

    # Mind is entirely local: D-Bus, the Journal, and state files. Nothing opens a network socket,
    # so AF_UNIX is the whole legitimate surface.
    RestrictAddressFamilies = [ "AF_UNIX" ];

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

    # Deliberately not set: MemoryDenyWriteExecute. Qt allocates executable pages for its JIT, and
    # forbidding them trades a real startup failure for a speculative gain.
    #
    # Deliberately not set: CapabilityBoundingSet. A user manager cannot drop the bounding set and
    # fails the unit at step CAPABILITIES with status 218 - which is how the first attempt at this
    # hardening broke every Mind daemon. Unprivileged user units hold no capabilities to drop, so
    # the directive was redundant as well as fatal.
  };

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
      }
      // hardening;
    };
in
{
  systemd.user.services = {
    cybou-eventd = mkService {
      description = "Cybou durable cognitive event journal";
      binary = "cybou-eventd";
      busName = "org.cybou.Mind.Event1";
    };
    cybou-lifecycled = mkService {
      description = "Cybou cognitive lifecycle coordinator";
      binary = "cybou-lifecycled";
      busName = "org.cybou.Mind.Lifecycle1";
      after = [ "cybou-eventd.service" ];
      wants = [ "cybou-eventd.service" ];
    };

    cybou-healthd = mkService {
      description = "Cybou capability and health owner";
      binary = "cybou-healthd";
      busName = "org.cybou.Mind.Health1";
      after = [
        "cybou-eventd.service"
        "cybou-lifecycled.service"
        "cybou-identityd.service"
        "cybou-intentiond.service"
        "cybou-predictord.service"
        "cybou-selfd.service"
        "cybou-workspaced.service"
        "cybou-presenced.service"
        "cybou-perceptiond.service"
        "cybou-epistemicd.service"
        "cybou-contextd.service"
      ];
      wants = [
        "cybou-eventd.service"
        "cybou-lifecycled.service"
        "cybou-identityd.service"
        "cybou-intentiond.service"
        "cybou-predictord.service"
        "cybou-selfd.service"
        "cybou-workspaced.service"
        "cybou-presenced.service"
        "cybou-perceptiond.service"
        "cybou-epistemicd.service"
        "cybou-contextd.service"
      ];
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

    # Reads the identity of the running system and proposes it as an Observation. It contributes
    # to biography and nothing else: it owns no state, mutates no configuration, and does not decide
    # whether what it reported is still true.
    cybou-perceptiond = mkService {
      description = "Cybou local perception adapter";
      binary = "cybou-perceptiond";
      busName = "org.cybou.Mind.Perception1";
      after = [ "cybou-eventd.service" ];
      wants = [ "cybou-eventd.service" ];
    };

    # Derives what is known from accepted observations. It reads Event1 and never writes to it:
    # perception proposes, this says what that amounts to, and the Journal remains the authority
    # over both.
    cybou-epistemicd = mkService {
      description = "Cybou epistemic projection owner";
      binary = "cybou-epistemicd";
      busName = "org.cybou.Mind.Epistemic1";
      after = [ "cybou-eventd.service" ];
      wants = [ "cybou-eventd.service" ];
    };

    cybou-contextd = mkService {
      description = "Cybou associative context owner";
      binary = "cybou-contextd";
      busName = "org.cybou.Mind.Context1";
      after = [ "cybou-eventd.service" ];
      wants = [ "cybou-eventd.service" ];
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
        "cybou-lifecycled.service"
      ];
      wants = [
        "cybou-eventd.service"
        "cybou-identityd.service"
        "cybou-intentiond.service"
        "cybou-predictord.service"
        "cybou-selfd.service"
        "cybou-workspaced.service"
        "cybou-lifecycled.service"
      ];
    };
  };
}
