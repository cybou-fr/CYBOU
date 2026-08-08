# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Base system: everything that is not desktop, branding or applications.
{ pkgs, ... }:
{
  # ADR-0006. Pins stateful-service defaults; never bumped automatically on upgrade.
  # This is not a "system version". Installed systems keep whatever
  # nixos-generate-config writes for them - Cybou does not override that.
  system.stateVersion = "26.05";

  nix.settings = {
    experimental-features = [
      "nix-command"
      "flakes"
    ];
    auto-optimise-store = true;
  };

  # Also decides the VM runner's name: docs/BUILDING.md documents
  # ./result/bin/run-cybou-vm, and that script is named after the host.
  networking.hostName = "cybou";
  networking.networkmanager.enable = true;

  time.timeZone = "UTC";
  i18n.defaultLocale = "en_US.UTF-8";
  console.keyMap = "us";

  # PipeWire, not PulseAudio. Gate A requires an active audio service.
  services.pulseaudio.enable = false;
  security.rtkit.enable = true;
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;
  };

  fonts = {
    enableDefaultPackages = true;
    packages = with pkgs; [
      # spec/design-tokens.json is visual authority; Inter/JetBrains Mono use Noto fallback.
      # Package names verified against the locked nixpkgs revision, not assumed.
      inter
      jetbrains-mono
      noto-fonts
      noto-fonts-color-emoji # renamed from noto-fonts-emoji in 26.05
    ];
    fontconfig.defaultFonts = {
      sansSerif = [
        "Inter"
        "Noto Sans"
      ];
      monospace = [
        "JetBrains Mono"
        "Noto Sans Mono"
      ];
    };
  };

  environment.systemPackages = with pkgs; [
    git
    vim
  ];

  # No telemetry and no required cloud dependency (README.md and docs/security/README.md).
  # Nothing here may phone home.
}
