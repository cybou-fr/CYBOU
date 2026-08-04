# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# CYB-032 - Gate A smoke test.
#
# This runs headless in QEMU under the Nix sandbox, so the service-level half of Gate A is
# machine-checked rather than eyeballed: SDDM starts, the graphical target is reached, a
# Wayland Plasma session comes up, plasmashell and KWin stay alive, and no unit failed.
#
# It does not replace the full VM. Layout, decoration, high-DPI and anything about how the
# desktop *looks* still needs a real session (docs/09-development-on-windows.md).
{ pkgs }:
pkgs.testers.runNixOSTest {
  name = "cybou-vm-smoke";

  nodes.machine =
    { ... }:
    {
      imports = [
        ../modules/base.nix
        ../modules/desktop-plasma.nix
      ];

      users.users.cybou = {
        isNormalUser = true;
        uid = 1000;
        extraGroups = [ "wheel" ];
        # Test-only credential; never reaches an image.
        password = "cybou";
      };

      services.displayManager.autoLogin = {
        enable = true;
        user = "cybou";
      };

      virtualisation = {
        memorySize = 4096;
        cores = 2;
        qemu.options = [ "-device virtio-gpu-pci" ];
      };
    };

  testScript = ''
    machine.wait_for_unit("multi-user.target")

    with subtest("SDDM starts"):
        machine.wait_for_unit("display-manager.service")

    with subtest("graphical target is reached"):
        machine.wait_for_unit("graphical.target")

    with subtest("a Wayland session exists for the user"):
        machine.wait_until_succeeds("test -e /run/user/1000/wayland-0")

    with subtest("plasmashell and KWin are running"):
        machine.wait_until_succeeds("pgrep -u cybou plasmashell")
        machine.wait_until_succeeds("pgrep -u cybou kwin_wayland")

    with subtest("PipeWire is active"):
        machine.wait_until_succeeds("systemctl --user -M cybou@ is-active pipewire.service")

    with subtest("no failed system units"):
        machine.succeed("systemctl --failed --no-legend | tee /dev/stderr | wc -l | grep -q '^0$'")

    # Evidence for the acceptance log. Not a visual check - it only proves something rendered.
    machine.sleep(5)
    machine.screenshot("desktop")
  '';
}
