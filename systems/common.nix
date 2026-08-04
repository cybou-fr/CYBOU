# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Shared composition. VM and ISO both build on this.
{
  imports = [
    ../modules/base.nix
    ../modules/desktop-plasma.nix
    ../modules/branding.nix
  ];
}
