# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Remote evaluation host: OVH VPS `vps-d0669a91.vps.ovh.net` (51.255.46.58).
#
# This is the deployment and integration-test target described in docs/DEPLOYMENT.md. It is
# headless on purpose: Plasma, SDDM, and the branding modules are not imported, because a server
# without a seat cannot honestly exercise a desktop session. What it does carry is the Mind
# service topology, the Nix toolchain that builds every gate, and one static preview surface.
#
# Hardware facts below were read from the running machine before the NixOS conversion
# (`lsblk`, `fdisk -l`, `networkctl status ens3`), not assumed from an OVH product page.
{
  lib,
  pkgs,
  ...
}:
{
  imports = [
    ../modules/base.nix
    ../modules/mind-services.nix
    ../modules/web-preview.nix
  ];

  # base.nix names the desktop image `cybou`. A remote host that appears in SSH known_hosts,
  # deploy logs, and OVH's control panel needs its own name.
  networking.hostName = lib.mkForce "cybou-vps";

  # The VPS is a QEMU guest booted through legacy BIOS even though the OVH image carries a GPT
  # label with an unused EFI System Partition (/dev/sda15). /sys/firmware/efi does not exist on
  # the running machine, so GRUB is installed into the protective MBR and the 3 MiB BIOS boot
  # partition (/dev/sda14) that the same label already provides.
  boot.loader.grub = {
    enable = true;
    device = "/dev/sda";
  };
  boot.loader.timeout = 3;

  # OVH exposes a serial console in its panel; keeping tty1 as well means a broken network
  # deployment is still recoverable without a rescue image.
  boot.kernelParams = [
    "console=tty1"
    "console=ttyS0,115200"
  ];

  boot.initrd.availableKernelModules = [
    "virtio_pci"
    "virtio_scsi"
    "virtio_blk"
    "virtio_net"
    "ahci"
    "sd_mod"
    "sr_mod"
  ];

  # The root partition is reused in place by the Debian-to-NixOS conversion
  # (scripts/prepare-vps-nixos.sh); the UUID is the one already carrying the filesystem.
  fileSystems."/" = {
    device = "/dev/disk/by-uuid/9b085fa0-4a45-4ee4-8528-4ed80730760d";
    fsType = "ext4";
    options = [
      "rw"
      "discard"
    ];
  };

  # The image ships without swap. Nix builds of Qt/Chromium-sized closures on 7.6 GiB fail at
  # link time without it, and a failed remote build is expensive to diagnose.
  swapDevices = [
    {
      device = "/var/swapfile";
      size = 4096;
    }
  ];

  # NetworkManager is a desktop dependency; a headless host uses networkd so the interface
  # configuration is declarative and reviewable.
  networking.networkmanager.enable = lib.mkForce false;
  networking.useDHCP = false;
  systemd.network = {
    enable = true;
    networks."10-ens3" = {
      matchConfig.Name = "ens3";

      # IPv4 comes from OVH's DHCP server, which hands out a /32 plus the classless route to
      # 51.255.32.1. IPv6 is not announced: the address and gateway are static, and the gateway
      # is off-link relative to the /128, so it needs GatewayOnLink.
      networkConfig = {
        DHCP = "ipv4";
        IPv6AcceptRA = false;
      };
      address = [ "2001:41d0:305:2100::1:5413/128" ];
      routes = [
        {
          Gateway = "2001:41d0:305:2100::1";
          GatewayOnLink = true;
        }
      ];
      linkConfig.RequiredForOnline = "routable";
    };
  };
  networking.nameservers = [
    "213.186.33.99"
    "2001:41d0:3:163::1"
  ];

  # Audio has no meaning without a seat, and both units would otherwise be started by base.nix.
  services.pipewire.enable = lib.mkForce false;
  security.rtkit.enable = lib.mkForce false;

  services.openssh = {
    enable = true;
    settings = {
      PasswordAuthentication = false;
      KbdInteractiveAuthentication = false;
      PermitRootLogin = "prohibit-password";
    };
  };

  networking.firewall.allowedTCPPorts = [
    22
    80
  ];

  # The deploy account keeps the name the OVH image created, because every existing key,
  # known_hosts entry, and script already addresses `debian@`.
  users.users.debian = {
    isNormalUser = true;
    description = "Cybou deployment operator";
    extraGroups = [
      "wheel"
      "kvm"
    ];
    openssh.authorizedKeys.keys = [
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEornKPBKD4WDO+cnEkVWJ1b32kYTyDXecoMQ2yBolIp cybou@AI-EVO"
    ];
  };

  # Deploys run non-interactively over SSH; a sudo password prompt would hang the pipe rather
  # than protect anything, since the same key already reaches the account.
  security.sudo.wheelNeedsPassword = false;

  # Separate account for the Mind owners: the deploy operator must not be the identity whose
  # Journal and lifecycle state the integration tests write.
  users.users.cybou = {
    isNormalUser = true;
    description = "Cybou Mind runtime identity";
    extraGroups = [ "kvm" ];
    openssh.authorizedKeys.keys = [
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEornKPBKD4WDO+cnEkVWJ1b32kYTyDXecoMQ2yBolIp cybou@AI-EVO"
    ];

    # The Mind daemons are D-Bus-activated user services. Without lingering there is no user
    # manager and no session bus between SSH logins, so an activation attempt would fail for a
    # reason that has nothing to do with the code under test.
    linger = true;
  };

  nix.settings = {
    trusted-users = [
      "root"
      "debian"
    ];
    # 4 vCPU. Leaving one core unbooked keeps SSH responsive during a full rebuild.
    max-jobs = 3;
    cores = 0;
  };

  # Store growth is the predictable way this host dies: every deployed generation pins a closure.
  nix.gc = {
    automatic = true;
    dates = "weekly";
    options = "--delete-older-than 30d";
  };

  environment.systemPackages = with pkgs; [
    curl
    htop
    rsync
    sqlite
    tmux
  ];
}
