# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
{ pkgs, cybouPackages }:
pkgs.testers.runNixOSTest {
  name = "cybou-p4-plasma-lifecycle";

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
    machine.succeed("systemctl --user -M cybou@ start cybou-presenced.service")
    machine.wait_until_succeeds(
        "systemctl --user -M cybou@ is-active cybou-presenced.service"
    )

    user_bus = (
        "sudo -u cybou XDG_RUNTIME_DIR=/run/user/1000 "
        "DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus busctl --user"
    )
    machine.succeed(
        f"{user_bus} call org.cybou.Mind.Lifecycle1 /org/cybou/Mind/Lifecycle1 "
        "org.cybou.Mind.Lifecycle1 Transition s idle | grep -q true"
    )
    run_id = machine.succeed(
        f"{user_bus} call org.cybou.Mind.Lifecycle1 /org/cybou/Mind/Lifecycle1 "
        "org.cybou.Mind.Lifecycle1 RequestRun sstasas consolidation p4-plasma "
        "0 0 0 | sed 's/^s \"//;s/\"$//'"
    ).strip()
    assert run_id
    run_blob = machine.succeed(
        "sed -n 's/.*\"run\":\"\\([^\"]*\\)\".*/\\1/p' "
        "/home/cybou/.local/state/cybou/lifecycle/state.json"
    ).strip()
    def journal_count():
        return int(machine.succeed(
            f"{user_bus} call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 "
            "org.cybou.Mind.Event1 Count | awk '{print $2}'"
        ).strip())

    # The claim below is that restarting plasmashell contributes nothing. That is only a claim about
    # the restart if the Journal was quiet when the baseline was taken. It is not: healthd records a
    # capability transition the first time it observes each owner, and an owner that is still
    # starting is observed later - so a baseline read early enough saw two of those transitions land
    # during the restart window and blamed them on the restart. Adding an owner to the registry made
    # this visible; it was always latent.
    machine.wait_until_succeeds(
        f"{user_bus} call org.cybou.Mind.Health1 /org/cybou/Mind/Health1 "
        "org.cybou.Mind.Health1 Refresh | grep -q true"
    )
    settled = journal_count()
    for _ in range(30):
        machine.sleep(2)
        again = journal_count()
        if again == settled:
            break
        settled = again
    else:
        raise AssertionError("the Journal never went quiet before the baseline")
    event_count = settled
    plasma_pid = int(machine.succeed(
        "timeout 10s systemctl --user -M cybou@ show -p MainPID --value "
        "plasma-plasmashell.service"
    ).strip())
    assert plasma_pid > 0

    machine.succeed(
        "timeout 10s systemctl --user -M cybou@ restart --no-block "
        "plasma-plasmashell.service"
    )
    # What is being tested is that Plasma comes back and that Mind neither lost its run nor
    # recorded anything for the restart - not how fast a software-rendered compositor restarts in a
    # VM. Thirty seconds turned out to be inside that spread: it failed roughly one run in four
    # while the count assertions passed, which is a timeout measuring the host rather than the code.
    machine.wait_until_succeeds(
        "timeout 5s systemctl --user -M cybou@ is-active plasma-plasmashell.service",
        timeout=120,
    )
    machine.wait_until_succeeds(
        "test \"$(timeout 5s systemctl --user -M cybou@ show -p MainPID --value "
        f"plasma-plasmashell.service)\" != \"{plasma_pid}\"",
        timeout=120,
    )
    machine.wait_until_succeeds(
        f"timeout 5s {user_bus} introspect org.kde.plasmashell /PlasmaShell "
        "org.kde.PlasmaShell | grep -q evaluateScript",
        timeout=120,
    )
    machine.wait_until_succeeds(
        "grep -q org.cybou.presence "
        "/home/cybou/.config/plasma-org.kde.plasma.desktop-appletsrc",
        timeout=120,
    )

    assert machine.succeed(
        "sed -n 's/.*\"run\":\"\\([^\"]*\\)\".*/\\1/p' "
        "/home/cybou/.local/state/cybou/lifecycle/state.json"
    ).strip() == run_blob
    after = journal_count()
    assert after == event_count, f"count {event_count} -> {after}"
  '';
}
