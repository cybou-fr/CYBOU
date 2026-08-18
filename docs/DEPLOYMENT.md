<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Build and Deployment Environments

## Active environment: NixOS on WSL2

The active Linux/Nix build and evaluation environment is the local `NixOS` WSL2 distribution.
Windows remains the editing host; Nix evaluation and Linux packages run inside WSL2. The working
tree is copied from `/mnt/c` into a temporary Linux filesystem so NTFS permissions and path behavior
do not become build inputs.

Run the normal gate from PowerShell:

```powershell
wsl -d NixOS -- bash /mnt/c/Users/cybou/Documents/CYBOU/scripts/wsl-checks.sh fast
```

Focused checks accept the same names as flake checks:

```powershell
wsl -d NixOS -- bash /mnt/c/Users/cybou/Documents/CYBOU/scripts/wsl-checks.sh web-ui desktop-shell rust-foundation
```

`full` includes KVM-backed NixOS VM gates and is valid only when `/dev/kvm` is present inside WSL2.
A green local run is developer evidence; tag CI and the release process remain authoritative.

## Retired environment: OVH

The former OVH VPS and `nixosConfigurations.cybou-vps` are retired and are not an active build, deployment, or
evaluation target. An attempted in-place conversion from Debian 13 to NixOS ended with SSH becoming
unavailable. The configuration remains in the repository only as historical and recovery input.

The following entry points deliberately refuse to execute:

- `scripts/vps-checks.sh`;
- `scripts/deploy-vps.sh`;
- `scripts/prepare-vps-nixos.sh`.

Do not repeat an in-place NixOS conversion on a replacement OVH Debian installation. If OVH is used
again later, treat Debian as a fresh independent host, install only the Nix package manager, and
introduce a new reviewed deployment design. No listener, gateway, session, credential, or personal
Journal is authorized there by the archived configuration.

## Boundaries

- WSL checks may build repository artifacts but do not activate the Windows host or an external
  server.
- `nix build` is distinct from `nixos-rebuild switch`.
- A desktop VM gate requires KVM and visible interaction evidence before it can establish W2 parity.
- Remote Living Canvas remains W4 work and requires TLS, authentication, revocation, origin policy,
  rate limits, and a separately reviewed deployment boundary.
