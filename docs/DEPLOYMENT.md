<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Deployment and the Remote Evaluation Host

## Purpose and status

Cybou is developed on Windows workstations and built with Nix, which cannot run there. This
document defines the one remote machine that closes that gap: a NixOS host built from this flake,
used to deploy, build, and test the system away from a developer's laptop.

The host is an evaluation and integration target. It is not a production service, it holds no
person's real Journal, and it does not serve Presence to a browser. Everything it currently
exposes to the network is static content and SSH. When the web gateway of
[Living Canvas Web UI Architecture](WEB_UI_ARCHITECTURE.md) exists, it arrives here through its
own module, its own ADR-level review, and phase W4's transport requirements — not by widening
what already listens.

## The host

| Property | Value |
|---|---|
| SSH target | `debian@vps-d0669a91.vps.ovh.net` |
| Address | `51.255.46.58`, `2001:41d0:305:2100::1:5413` |
| Provider | OVH VPS, QEMU/virtio guest |
| Resources | 4 vCPU, 7.6 GiB RAM, 75 GiB disk, `/dev/kvm` present |
| Firmware | legacy BIOS boot on a GPT label |
| Operating system | NixOS, built from `nixosConfigurations.cybou-vps` |
| Configuration | [`systems/vps.nix`](../systems/vps.nix) |

`/dev/kvm` is present, so the NixOS VM tests — `vm-smoke`, `p4-plasma-lifecycle`,
`lifecycle-continuity`, `m6-recovery-boundary` — can run on this host. That is the practical
reason it exists: those gates are the project's strongest evidence and no Windows workstation can
execute them.

The deploy account is `debian` because that is the account the provider image created and the one
every existing key and `known_hosts` entry already addresses. Mind runs as a separate `cybou`
account with lingering enabled, so the operator identity is never the identity whose lifecycle
state a test writes.

## What the host runs

Implemented today:

- NixOS built from this repository's flake, including `modules/base.nix`;
- the D-Bus-activated Mind user services from `modules/mind-services.nix`, under the `cybou`
  account;
- the Nix toolchain used by every repository gate, including the KVM-backed VM tests;
- `modules/web-preview.nix`: nginx serving the committed `www/` site over plain HTTP.

Deliberately absent:

- Plasma, SDDM, and the branding modules. A machine without a seat cannot honestly exercise a
  desktop session, and importing them would produce units that report success while rendering
  nothing.
- `cybou-web-gateway` and the Rust/WASM Living Canvas. They are not implemented; see
  [Rust Migration Plan](RUST_MIGRATION.md).
- TLS, sessions, cookies, and authentication. The preview listener has none, so no projection,
  credential, or personal data may be introduced on it. Remote access to real Presence data is
  phase W4 work and requires TLS at the external boundary.

## Deploying

All three scripts push the **working tree**, not `HEAD`. Testing an unfinished change is the
main reason the host exists; a deploy pinned to the last commit could not do it. The build runs
on the target, because the workstation cannot build a NixOS closure.

```bash
scripts/deploy-vps.sh switch
```

| Action | Effect |
|---|---|
| `dry-activate` | shows what activation would change |
| `build` | builds the closure only |
| `test` | activates without touching the bootloader; a reboot returns to the previous system |
| `boot` | installs for the next boot without activating now |
| `switch` | activates and makes it the boot default |

Prefer `test` for anything that could remove your own access. If activation breaks SSH, a reboot
undoes it; `switch` does not give you that.

Rolling back a bad `switch` is a generation change, not a redeploy:

```bash
ssh debian@vps-d0669a91.vps.ovh.net "sudo nixos-rebuild switch --rollback"
```

The host and paths come from [`scripts/vps-env.sh`](../scripts/vps-env.sh) and can be overridden
with `CYBOU_VPS_HOST`, `CYBOU_VPS_SRC`, and `CYBOU_VPS_FLAKE_ATTR`.

## Testing on the host

```bash
scripts/vps-checks.sh fast
scripts/vps-checks.sh full
scripts/vps-checks.sh lifecycle-continuity
```

`fast` is the set CI runs on every push. `full` adds the four NixOS VM tests. A VM gate takes
minutes and a large amount of store space, so it is requested explicitly rather than implied.

See [Testing](TESTING.md) for what each gate proves. A green run on this host is evidence about
this host's kernel, KVM support, and store contents; it does not replace the evidence CI records
for a tag.

## Server preparation: Debian to NixOS

The provider delivers Debian 13. The conversion to NixOS uses the procedure NixOS documents for
installing from a running Linux distribution — install Nix, build the closure, set the system
profile, hand the root filesystem to `NIXOS_LUSTRATE` — rather than a third-party infect script
executed as root. The only external artifact is the version-pinned Nix installer, verified
against a checksum recorded in the script.

```bash
scripts/prepare-vps-nixos.sh preflight
scripts/prepare-vps-nixos.sh nix
scripts/prepare-vps-nixos.sh build
CYBOU_VPS_CONFIRM=I-UNDERSTAND scripts/prepare-vps-nixos.sh convert
scripts/prepare-vps-nixos.sh reboot
scripts/prepare-vps-nixos.sh verify
```

The stages are separated by reversibility. `preflight`, `nix`, and `build` change nothing about
how the machine boots and can be abandoned at any point. `convert` sets the system profile and
installs GRUB; after it, the machine boots NixOS and the Debian image cannot be recovered in
place — only reinstalled from the OVH panel. That is why `convert` refuses to run without
`CYBOU_VPS_CONFIRM=I-UNDERSTAND`.

`preflight` reads the machine and refuses if it stops matching `systems/vps.nix`: architecture,
root filesystem UUID, disk node, boot firmware, interface name, and free space. A configuration
that no longer describes the hardware must fail before the bootloader is touched, not after.

Two properties are preserved across the conversion on purpose:

- **SSH host keys.** `NIXOS_LUSTRATE` keeps them, so `known_hosts` stays valid. Treating a
  changed host key as routine is the habit an interception attack depends on.
- **The old root filesystem.** Stage 2 moves it to `/old-root` instead of deleting it, and
  Debian's `/boot` is kept as `/boot.bak` until the new system has proved it boots. Reclaim both
  only after `verify` passes.

If the host does not return after `reboot`, use OVH's serial console — `systems/vps.nix` keeps
`console=ttyS0,115200` on the kernel command line for exactly that case.

## Boundaries this deployment must not cross

- No secret, credential, or real personal Journal on an evaluation host reachable over plain
  HTTP.
- No claim that a green gate here is a release: [Release Process](RELEASE.md) is unchanged.
- No new listener without the security baseline in
  [Living Canvas Web UI Architecture](WEB_UI_ARCHITECTURE.md) and a threat-model update.
- No divergence between the deployed system and `systems/vps.nix`. Anything configured by hand on
  the host is lost at the next `switch`, which is the intended behavior; changes belong in the
  repository.
