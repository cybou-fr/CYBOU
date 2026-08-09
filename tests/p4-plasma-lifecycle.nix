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
    event_count = int(machine.succeed(
        f"{user_bus} call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 "
        "org.cybou.Mind.Event1 Count | awk '{print $2}'"
    ).strip())
    plasma_pid = int(machine.succeed(
        "timeout 10s systemctl --user -M cybou@ show -p MainPID --value "
        "plasma-plasmashell.service"
    ).strip())
    assert plasma_pid > 0

    machine.succeed(
        "timeout 10s systemctl --user -M cybou@ restart --no-block "
        "plasma-plasmashell.service"
    )
    machine.wait_until_succeeds(
        "timeout 5s systemctl --user -M cybou@ is-active plasma-plasmashell.service",
        timeout=30,
    )
    machine.wait_until_succeeds(
        "test \"$(timeout 5s systemctl --user -M cybou@ show -p MainPID --value "
        f"plasma-plasmashell.service)\" != \"{plasma_pid}\"",
        timeout=30,
    )
    machine.wait_until_succeeds(
        f"timeout 5s {user_bus} introspect org.kde.plasmashell /PlasmaShell "
        "org.kde.PlasmaShell | grep -q evaluateScript",
        timeout=30,
    )
    machine.wait_until_succeeds(
        "grep -q org.cybou.presence "
        "/home/cybou/.config/plasma-org.kde.plasma.desktop-appletsrc",
        timeout=30,
    )

    assert machine.succeed(
        "sed -n 's/.*\"run\":\"\\([^\"]*\\)\".*/\\1/p' "
        "/home/cybou/.local/state/cybou/lifecycle/state.json"
    ).strip() == run_blob
    assert int(machine.succeed(
        f"{user_bus} call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 "
        "org.cybou.Mind.Event1 Count | awk '{print $2}'"
    ).strip()) == event_count
  '';
}
