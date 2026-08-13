# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
{ pkgs, cybouPackages }:
pkgs.testers.runNixOSTest {
  name = "cybou-m6-recovery-boundary";

  nodes.machine =
    { ... }:
    {
      imports = [
        ../modules/base.nix
        ../modules/desktop-plasma.nix
        ../modules/branding.nix
        ../modules/mind-services.nix
      ];
      _module.args.cybouPackages = cybouPackages;

      users.users.cybou = {
        isNormalUser = true;
        uid = 1000;
        password = "cybou";
      };
      services.displayManager.autoLogin = {
        enable = true;
        user = "cybou";
      };
      environment.sessionVariables = {
        KWIN_COMPOSE = "Q";
        LIBGL_ALWAYS_SOFTWARE = "1";
      };
      virtualisation = {
        memorySize = 4096;
        cores = 2;
      };
    };

  testScript = ''
    machine.start()
    machine.wait_for_unit("graphical.target")
    machine.wait_until_succeeds("test -e /run/user/1000/wayland-0")
    machine.wait_until_succeeds(
        "systemctl --user -M cybou@ is-active plasma-plasmashell.service"
    )
    user_bus = (
        "sudo -u cybou XDG_RUNTIME_DIR=/run/user/1000 "
        "DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus busctl --user"
    )

    with subtest("D-Bus activates Presence without replacing Plasma or owners"):
        machine.wait_until_succeeds(
            "grep -q org.cybou.presence "
            "/home/cybou/.config/plasma-org.kde.plasma.desktop-appletsrc"
        )
        plasma_pid = int(machine.succeed(
            "systemctl --user -M cybou@ show -p MainPID --value plasma-plasmashell.service"
        ).strip())
        machine.succeed("systemctl --user -M cybou@ stop cybou-presenced.service")
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Presence1 /org/cybou/Mind/Presence1 "
            "org.cybou.Mind.Presence1 Ready | grep -q true"
        )
        machine.wait_until_succeeds(
            "systemctl --user -M cybou@ is-active cybou-presenced.service"
        )
        assert int(machine.succeed(
            "systemctl --user -M cybou@ show -p MainPID --value plasma-plasmashell.service"
        ).strip()) == plasma_pid

    with subtest("timeout cannot invent lifecycle interruption"):
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 /org/cybou/Mind/Lifecycle1 "
            "org.cybou.Mind.Lifecycle1 Transition s idle | grep -q true"
        )
        run_id = machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 /org/cybou/Mind/Lifecycle1 "
            "org.cybou.Mind.Lifecycle1 RequestRun sstasas maintenance m6-timeout "
            "0 0 0 | sed 's/^s \"//;s/\"$//'"
        ).strip()
        assert run_id
        lifecycle_before_timeout = machine.succeed(
            "cat /home/cybou/.local/state/cybou/lifecycle/state.json"
        ).strip()
        machine.succeed(
            "systemctl --user -M cybou@ set-environment "
            "CYBOU_PRESENCE_INTERRUPT_DELAY_MS=6000; "
            "systemctl --user -M cybou@ restart cybou-presenced.service"
        )
        machine.succeed(
            f"timeout 8s {user_bus} call org.cybou.Mind.Presence1 /org/cybou/Mind/Presence1 "
            "org.cybou.Mind.Presence1 InterruptLifecycle s m6-timeout | grep -q false"
        )
        assert machine.succeed(
            "cat /home/cybou/.local/state/cybou/lifecycle/state.json"
        ).strip() == lifecycle_before_timeout
        machine.succeed(
            "systemctl --user -M cybou@ unset-environment CYBOU_PRESENCE_INTERRUPT_DELAY_MS; "
            "systemctl --user -M cybou@ restart cybou-presenced.service"
        )
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Presence1 /org/cybou/Mind/Presence1 "
            "org.cybou.Mind.Presence1 InterruptLifecycle s m6-recovered | grep -q true"
        )
        assert machine.succeed(
            "cat /home/cybou/.local/state/cybou/lifecycle/state.json"
        ).strip() != lifecycle_before_timeout

    with subtest("unresponsive Event1 rejects Promise and preserves count"):
        # What this subtest claims is that a Promise rejected because Event1 was unreachable leaves
        # no commitment behind. The Journal count was standing in for that and is not a faithful
        # stand-in: healthd records a capability transition when it observes eventd frozen and
        # cannot write it until eventd resumes, so that write lands inside the window and is
        # charged to the Promise. Measured at roughly one failure in four, while the rejection
        # itself always behaved.
        #
        # intentiond's open set says the same thing precisely and is unmoved by what other owners
        # record about the outage. Measured directly: a Promise that succeeds takes this from 70
        # bytes to 604, so a leaked commitment cannot hide in it.
        commitments_before = machine.succeed(
            f"{user_bus} call org.cybou.Mind.Intention1 /org/cybou/Mind/Intention1 "
            "org.cybou.Mind.Intention1 Open"
        ).strip()
        # Earlier subtests leave commitments open. Asserted rather than assumed, because comparing
        # an empty set against an empty set would prove nothing at all.
        assert len(commitments_before) > len("ay 0"), commitments_before
        count_before = int(machine.succeed(
            f"{user_bus} call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 "
            "org.cybou.Mind.Event1 Count | awk '{print $2}'"
        ).strip())
        machine.succeed(
            "systemctl --user -M cybou@ set-environment "
            "CYBOU_PRESENCE_COMMAND_TIMEOUT_MS=1000; "
            "systemctl --user -M cybou@ restart cybou-presenced.service"
        )
        event_pid = int(machine.succeed(
            "systemctl --user -M cybou@ show -p MainPID --value cybou-eventd.service"
        ).strip())
        machine.succeed(f"kill -STOP {event_pid}")
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Health1 /org/cybou/Mind/Health1 "
            "org.cybou.Mind.Health1 Refresh | grep -q true"
        )
        machine.succeed(
            f"{user_bus} --timeout=3s call org.cybou.Mind.Presence1 /org/cybou/Mind/Presence1 "
            "org.cybou.Mind.Presence1 Promise s rejected-without-event1 | grep -q '^s \"\"$'"
        )
        machine.succeed(f"kill -CONT {event_pid}")
        machine.succeed(
            "systemctl --user -M cybou@ unset-environment "
            "CYBOU_PRESENCE_COMMAND_TIMEOUT_MS; "
            "systemctl --user -M cybou@ restart cybou-presenced.service"
        )
        machine.wait_until_succeeds(
            f"{user_bus} call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 "
            "org.cybou.Mind.Event1 Ready | grep -q true"
        )
        commitments_after = machine.succeed(
            f"{user_bus} call org.cybou.Mind.Intention1 /org/cybou/Mind/Intention1 "
            "org.cybou.Mind.Intention1 Open"
        ).strip()
        assert commitments_after == commitments_before

        count_after = int(machine.succeed(
            f"{user_bus} call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 "
            "org.cybou.Mind.Event1 Count | awk '{print $2}'"
        ).strip())
        # Only what is actually true across an outage: history is append-only, so it may grow while
        # eventd is unreachable and must never shrink.
        assert count_after >= count_before, f"count {count_before} -> {count_after}"
        assert int(machine.succeed(
            "systemctl --user -M cybou@ show -p MainPID --value plasma-plasmashell.service"
        ).strip()) == plasma_pid
  '';
}
