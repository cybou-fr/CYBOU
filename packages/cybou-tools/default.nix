# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# User-facing recovery tools. The reset script is a release gate, not a convenience:
# spec/acceptance.yaml requires reset_to_breeze and configuration_backup.
{
  lib,
  writeShellApplication,
  kdePackages,
  coreutils,
}:
writeShellApplication {
  name = "cybou-reset-desktop";

  # plasma-apply-* rather than hand-edited config: the tools notify the right services, and a
  # file edited behind Plasma's back needs a logout to take effect anyway.
  runtimeInputs = [
    coreutils
    kdePackages.plasma-workspace
  ];

  text = builtins.readFile ./cybou-reset-desktop.sh;

  meta = {
    description = "Return the Cybou desktop to stock Breeze, with a backup";
    license = lib.licenses.mit;
  };
}
