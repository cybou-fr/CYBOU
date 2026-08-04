# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# User-facing desktop tools. Neither is a convenience: spec/acceptance.yaml requires
# reset_to_breeze, configuration_backup and first_login_idempotent.
#
# Both are writeShellApplication, so shellcheck runs at build time. That is currently the
# project's only automated check for shell, and it has already earned its place.
{
  lib,
  symlinkJoin,
  writeShellApplication,
  kdePackages,
  coreutils,
}:
let
  runtimeInputs = [
    coreutils
    kdePackages.plasma-workspace
  ];

  # plasma-apply-* rather than hand-edited config: the tools notify the right services, and a
  # file edited behind Plasma's back needs a logout to take effect anyway.
  reset = writeShellApplication {
    name = "cybou-reset-desktop";
    inherit runtimeInputs;
    text = builtins.readFile ./cybou-reset-desktop.sh;
  };

  firstLogin = writeShellApplication {
    name = "cybou-first-login";
    inherit runtimeInputs;
    text = builtins.readFile ./cybou-first-login.sh;
  };
in
symlinkJoin {
  name = "cybou-tools";
  paths = [
    reset
    firstLogin
  ];

  meta = {
    description = "Cybou desktop initialisation and recovery tools";
    license = lib.licenses.mit;
  };
}
