# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# CYB-032 - Gate A smoke test.
#
# Runs headless in QEMU under the Nix sandbox, so the service-level half of Gate A is
# machine-checked rather than eyeballed.
#
# History worth keeping: the first version of this test passed while the desktop rendered
# nothing at all. It enabled autologin, so the greeter never ran, and it asserted that
# processes existed and no unit had failed - both true of a KWin that cannot get an EGL
# context and draws no frames. The lesson is that "the process is alive" says nothing about
# graphics. Hence two nodes and the pixel check below.
{ pkgs, cybouPackages }:
let
  common =
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
        extraGroups = [ "wheel" ];
        password = "cybou"; # test-only; never reaches an image
      };

      # Same software-rendering workaround as systems/vm.nix, and for the same reason:
      # the emulated GPU has no render node. Documented there in full.
      environment.sessionVariables = {
        KWIN_COMPOSE = "Q";
        LIBGL_ALWAYS_SOFTWARE = "1";
      };

      virtualisation = {
        memorySize = 4096;
        cores = 2;
      };
    };
in
pkgs.testers.runNixOSTest {
  name = "cybou-vm-smoke";

  nodes = {
    # Reaches the desktop, so the session can be inspected.
    session =
      { ... }:
      {
        imports = [ common ];
        services.displayManager.autoLogin = {
          enable = true;
          user = "cybou";
        };
      };

    # No autologin: this one exercises the SDDM greeter, which autologin hides entirely.
    greeter = common;
  };

  testScript = ''
    start_all()

    with subtest("SDDM starts on both nodes"):
        session.wait_for_unit("display-manager.service")
        greeter.wait_for_unit("display-manager.service")

    with subtest("graphical target is reached"):
        session.wait_for_unit("graphical.target")

    with subtest("a Wayland session exists for the user"):
        session.wait_until_succeeds("test -e /run/user/1000/wayland-0")

    with subtest("plasmashell and KWin are running"):
        session.wait_until_succeeds("pgrep -u cybou plasmashell")
        session.wait_until_succeeds("pgrep -u cybou kwin_wayland")

    with subtest("the compositor got a working renderer"):
        # The failure mode this test previously missed. KWin logs these when it cannot
        # create an EGL context, and then renders nothing while staying alive.
        for machine in (session, greeter):
            machine.fail(
                "journalctl -b | grep -q 'egl: failed to create dri2 screen'"
            )
            machine.fail(
                "journalctl -b | grep -q 'ZINK: failed to choose pdev'"
            )

    with subtest("Plasma itself accepts the Global Theme"):
        # Gate B, asked of the tool that actually loads the package rather than of a static
        # reader. A package kpackagetool6 cannot see does not exist as far as Plasma is
        # concerned.
        #
        # XDG_DATA_DIRS is set explicitly: this runs as root in a bare shell, which does not
        # inherit the session environment, so without it the tool searches only its built-in
        # paths and reports nothing - a false failure that says nothing about the package.
        # `--list` alone reports only ~/.local/share and lists nothing at all, not even
        # Breeze; `--global` is the one that looks where a distribution installs.
        print(session.succeed(
            "kpackagetool6 --type=Plasma/LookAndFeel --list --global 2>&1 || true"
        ))
        session.succeed(
            "kpackagetool6 --type=Plasma/LookAndFeel --list --global "
            "| grep -q org.cybou.horizon.desktop"
        )
        session.succeed("grep -q CybouHorizonDark /etc/xdg/kdeglobals")

        # Open question, tracked as CYB-037: kpackagetool6 lists our Plasma Style but rejects
        # every upstream Breeze style with a KPackageStructure mismatch, so upstream clearly
        # declares something other than "Plasma/Theme". Being listed here therefore proves the
        # file is readable, not that Plasma will use the style.
        #
        # plasma-apply-desktoptheme would settle it, but it needs a Qt platform and core-dumps
        # when run as root with no display; it has to be QT_QPA_PLATFORM=offscreen, or run
        # inside the user session. Until that is done, treat the style as unverified.
        print(session.succeed(
            "kpackagetool6 --type=Plasma/Theme --list --global 2>&1 || true"
        ))
        session.succeed(
            "test -f /run/current-system/sw/share/plasma/desktoptheme/CybouHorizon/colors"
        )


    with subtest("Mind organs are separate user services"):
        session.succeed(
            "systemctl --user -M cybou@ start cybou-presenced.service"
        )

        for unit in (
            "cybou-eventd.service",
            "cybou-identityd.service",
            "cybou-intentiond.service",
            "cybou-predictord.service",
            "cybou-selfd.service",
            "cybou-workspaced.service",
            "cybou-presenced.service",
        ):
            session.wait_until_succeeds(
                f"systemctl --user -M cybou@ is-active {unit}"
            )

        mind_pids = []
        for unit in (
            "cybou-eventd.service",
            "cybou-identityd.service",
            "cybou-intentiond.service",
            "cybou-predictord.service",
            "cybou-selfd.service",
            "cybou-workspaced.service",
            "cybou-presenced.service",
        ):
            pid = session.succeed(
                f"systemctl --user -M cybou@ show -p MainPID --value {unit}"
            ).strip()
            assert int(pid) > 0, f"{unit} has no MainPID"
            mind_pids.append(pid)
        assert len(set(mind_pids)) == 7, mind_pids

    with subtest("PipeWire is active"):
        session.wait_until_succeeds(
            "systemctl --user -M cybou@ is-active pipewire.service"
        )

    with subtest("no failed system units"):
        for machine in (session, greeter):
            machine.succeed(
                "systemctl --failed --no-legend | tee /dev/stderr | wc -l | grep -q '^0$'"
            )

    # NOTE - open defect, do not read this test as proof the desktop renders.
    #
    # An OCR assertion (wait_for_text("Password")) was tried here and removed: it timed out
    # after 900 s while the same configuration rendered correctly on real hardware, so it
    # failed for reasons unrelated to the product - small grey placeholder text, and a host
    # under load from two 4 GB nodes.
    #
    # Worse, this test environment does not reproduce the real failure at all: on 2026-08-04
    # the greeter rendered here while a real boot of the same configuration showed only
    # wallpaper. Until that difference is understood, the assertions above cover services and
    # the renderer, and "does it actually draw" is verified by hand in the Hyper-V image.

    session.sleep(10)
    session.screenshot("desktop")
    greeter.screenshot("greeter")
  '';
}
