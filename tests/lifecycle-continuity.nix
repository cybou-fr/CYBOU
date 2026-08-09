# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
{ pkgs, cybouPackages }:
pkgs.testers.runNixOSTest {
  name = "cybou-lifecycle-continuity";

  nodes.machine =
    { ... }:
    {
      imports = [ ../modules/mind-services.nix ];
      _module.args.cybouPackages = cybouPackages;

      system.stateVersion = "26.05";
      users.users.cybou = {
        isNormalUser = true;
        uid = 1000;
        linger = true;
      };

      virtualisation = {
        memorySize = 1024;
        cores = 1;
      };
    };

  testScript = ''
    machine.start(allow_reboot=True)
    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("user@1000.service")

    with subtest("D-Bus activated owners persist identity and an active run"):
        machine.succeed(
            "systemctl --user -M cybou@ start cybou-identityd.service "
            "cybou-lifecycled.service"
        )
        machine.wait_until_succeeds(
            "systemctl --user -M cybou@ is-active cybou-identityd.service "
            "cybou-lifecycled.service"
        )

        user_bus = "busctl --user --machine=cybou@"
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 "
            "/org/cybou/Mind/Lifecycle1 org.cybou.Mind.Lifecycle1 "
            "Transition s idle | grep -q true"
        )
        run_id = machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 "
            "/org/cybou/Mind/Lifecycle1 org.cybou.Mind.Lifecycle1 "
            "RequestRun sstasas consolidation reboot-test 0 1 journal 0 "
            "| sed 's/^s \"//;s/\"$//'"
        ).strip()
        assert run_id
        run_blob = machine.succeed(
            "sed -n 's/.*\"run\":\"\\([^\"]*\\)\".*/\\1/p' "
            "/home/cybou/.local/state/cybou/lifecycle/state.json"
        ).strip()
        assert run_blob

        identity_before = machine.succeed(
            "grep '\"identityId\"' /home/cybou/.local/state/cybou/identity.json "
            "| sed 's/.*: *\"//;s/\".*//'"
        ).strip()
        count_before = int(machine.succeed(
            "grep '\"sessionCount\"' /home/cybou/.local/state/cybou/identity.json "
            "| sed 's/[^0-9]//g'"
        ).strip())

    with subtest("system reboot recovers the run and advances the logical session"):
        machine.reboot()
        machine.wait_for_unit("multi-user.target")
        machine.wait_for_unit("user@1000.service")
        machine.succeed(
            "systemctl --user -M cybou@ start cybou-identityd.service "
            "cybou-lifecycled.service"
        )
        machine.wait_until_succeeds(
            "grep -q '\"mode\":\"recovering\"' "
            "/home/cybou/.local/state/cybou/lifecycle/state.json"
        )
        recovered_blob = machine.succeed(
            "sed -n 's/.*\"run\":\"\\([^\"]*\\)\".*/\\1/p' "
            "/home/cybou/.local/state/cybou/lifecycle/state.json"
        ).strip()
        assert recovered_blob == run_blob

        identity_after = machine.succeed(
            "grep '\"identityId\"' /home/cybou/.local/state/cybou/identity.json "
            "| sed 's/.*: *\"//;s/\".*//'"
        ).strip()
        count_after = int(machine.succeed(
            "grep '\"sessionCount\"' /home/cybou/.local/state/cybou/identity.json "
            "| sed 's/[^0-9]//g'"
        ).strip())
        assert identity_after == identity_before
        assert count_after == count_before + 1
  '';
}
