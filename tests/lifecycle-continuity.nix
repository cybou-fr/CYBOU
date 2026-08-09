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

    with subtest("close the reboot probe and prepare Event1 owner work"):
        user_bus = "busctl --user --machine=cybou@"
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 "
            "/org/cybou/Mind/Lifecycle1 org.cybou.Mind.Lifecycle1 "
            "FinishRun ss interrupted reboot-probe-complete | grep -q true"
        )
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 "
            "/org/cybou/Mind/Lifecycle1 org.cybou.Mind.Lifecycle1 "
            "Transition s awake | grep -q true"
        )
        machine.succeed("systemctl --user -M cybou@ start cybou-predictord.service")
        machine.wait_until_succeeds("systemctl --user -M cybou@ is-active cybou-predictord.service")

    with subtest("reboot after owner commit replays without a duplicate effect"):
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Predictor1 "
            "/org/cybou/Mind/Predictor1 org.cybou.Mind.Predictor1 "
            "Observe sd vm-owner-boundary 1.0 | grep -q true"
        )
        owner_before = int(machine.succeed(
            f"{user_bus} call org.cybou.Mind.Event1 "
            "/org/cybou/Mind/Event1 org.cybou.Mind.Event1 Count "
            "| awk '{print $2}'"
        ).strip())
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 "
            "/org/cybou/Mind/Lifecycle1 org.cybou.Mind.Lifecycle1 "
            "Transition s idle | grep -q true"
        )
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 "
            "/org/cybou/Mind/Lifecycle1 org.cybou.Mind.Lifecycle1 "
            "RequestRunAtCurrentHead ssasas consolidation vm-owner-crash "
            "1 predictor 0 | grep -q '^s'"
        )
        machine.succeed(
            "systemctl --user -M cybou@ set-environment "
            "CYBOU_LIFECYCLE_FAILPOINT=after-owner-commit; "
            "systemctl --user -M cybou@ restart cybou-lifecycled.service"
        )
        machine.wait_until_succeeds(
            "grep -q '\"mode\":\"recovering\"' "
            "/home/cybou/.local/state/cybou/lifecycle/state.json"
        )
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 /org/cybou/Mind/Lifecycle1 "
            "org.cybou.Mind.Lifecycle1 ResumeRun | grep -q true"
        )
        machine.fail(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 "
            "/org/cybou/Mind/Lifecycle1 org.cybou.Mind.Lifecycle1 Dispatch"
        )
        machine.succeed(
            "systemctl --user -M cybou@ unset-environment CYBOU_LIFECYCLE_FAILPOINT; "
            "systemctl --user -M cybou@ stop cybou-lifecycled.service"
        )
        owner_after_crash = int(machine.succeed(
            f"{user_bus} call org.cybou.Mind.Event1 "
            "/org/cybou/Mind/Event1 org.cybou.Mind.Event1 Count "
            "| awk '{print $2}'"
        ).strip())
        assert owner_after_crash == owner_before + 1

        machine.reboot()
        machine.wait_for_unit("user@1000.service")
        machine.succeed("systemctl --user -M cybou@ start cybou-lifecycled.service cybou-predictord.service")
        machine.wait_until_succeeds(
            "grep -q '\"mode\":\"recovering\"' "
            "/home/cybou/.local/state/cybou/lifecycle/state.json"
        )
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 /org/cybou/Mind/Lifecycle1 "
            "org.cybou.Mind.Lifecycle1 ResumeRun | grep -q true"
        )
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 /org/cybou/Mind/Lifecycle1 "
            "org.cybou.Mind.Lifecycle1 Dispatch | grep -q true"
        )
        owner_after_replay = int(machine.succeed(
            f"{user_bus} call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 "
            "org.cybou.Mind.Event1 Count | awk '{print $2}'"
        ).strip())
        assert owner_after_replay == owner_after_crash
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 /org/cybou/Mind/Lifecycle1 "
            "org.cybou.Mind.Lifecycle1 FinishRun ss completed owner-reboot-recovered "
            "| grep -q true"
        )

    with subtest("reboot after terminal commit reuses the accepted Outcome"):
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Predictor1 /org/cybou/Mind/Predictor1 "
            "org.cybou.Mind.Predictor1 Observe sd vm-terminal-boundary 2.0 | grep -q true"
        )
        terminal_before = int(machine.succeed(
            f"{user_bus} call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 "
            "org.cybou.Mind.Event1 Count | awk '{print $2}'"
        ).strip())
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 /org/cybou/Mind/Lifecycle1 "
            "org.cybou.Mind.Lifecycle1 Transition s idle | grep -q true"
        )
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 /org/cybou/Mind/Lifecycle1 "
            "org.cybou.Mind.Lifecycle1 RequestRunAtCurrentHead ssasas consolidation "
            "vm-terminal-crash 1 predictor 0 | grep -q '^s'"
        )
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 /org/cybou/Mind/Lifecycle1 "
            "org.cybou.Mind.Lifecycle1 Dispatch | grep -q true"
        )
        machine.succeed(
            "systemctl --user -M cybou@ set-environment "
            "CYBOU_LIFECYCLE_FAILPOINT=after-terminal-commit; "
            "systemctl --user -M cybou@ restart cybou-lifecycled.service"
        )
        machine.wait_until_succeeds(
            "grep -q '\"mode\":\"recovering\"' "
            "/home/cybou/.local/state/cybou/lifecycle/state.json"
        )
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 /org/cybou/Mind/Lifecycle1 "
            "org.cybou.Mind.Lifecycle1 ResumeRun | grep -q true"
        )
        machine.fail(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 /org/cybou/Mind/Lifecycle1 "
            "org.cybou.Mind.Lifecycle1 FinishRun ss completed terminal-reboot-recovered"
        )
        machine.succeed(
            "systemctl --user -M cybou@ unset-environment CYBOU_LIFECYCLE_FAILPOINT; "
            "systemctl --user -M cybou@ stop cybou-lifecycled.service"
        )
        terminal_after_crash = int(machine.succeed(
            f"{user_bus} call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 "
            "org.cybou.Mind.Event1 Count | awk '{print $2}'"
        ).strip())
        assert terminal_after_crash == terminal_before + 2

        machine.reboot()
        machine.wait_for_unit("user@1000.service")
        machine.succeed("systemctl --user -M cybou@ start cybou-lifecycled.service")
        machine.wait_until_succeeds(
            "grep -q '\"mode\":\"recovering\"' "
            "/home/cybou/.local/state/cybou/lifecycle/state.json"
        )
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 /org/cybou/Mind/Lifecycle1 "
            "org.cybou.Mind.Lifecycle1 ResumeRun | grep -q true"
        )
        machine.succeed(
            f"{user_bus} call org.cybou.Mind.Lifecycle1 /org/cybou/Mind/Lifecycle1 "
            "org.cybou.Mind.Lifecycle1 FinishRun ss completed terminal-reboot-recovered "
            "| grep -q true"
        )
        terminal_after_replay = int(machine.succeed(
            f"{user_bus} call org.cybou.Mind.Event1 /org/cybou/Mind/Event1 "
            "org.cybou.Mind.Event1 Count | awk '{print $2}'"
        ).strip())
        assert terminal_after_replay == terminal_after_crash
  '';
}
