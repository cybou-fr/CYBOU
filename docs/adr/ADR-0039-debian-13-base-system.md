<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0039: Debian 13 Base System

## Status

Accepted

## Context

Cybou originally used NixOS configurations, Calamares live ISO generation, and Nix expressions as
the system deployment baseline (ADR-0005, ADR-0006). While NixOS provided declarative environment
definitions, it introduced substantial developer friction, custom package overhead for the target
Rust/WASM + Wayland/Chromium stack, and divergence from standard server/VPS deployment environments.

With the adoption of a Rust-first codebase (ADR-0038) and web-first presence (ADR-0037), Cybou needs
a stable, widely supported, tier-1 Linux distribution as its base platform for development, CI, and
production deployments.

## Decision

**Debian 13 (Trixie) is the official base operating system for Cybou.**

Key architecture and operational decisions:
1. **Target platform**: All production services, background daemons, integration gates, and VPS
   deployments target Debian 13 (x86_64 / amd64).
2. **Service orchestration**: System services and cognitive Mind processes are managed exclusively
   via `systemd --user` units and standard D-Bus session activation.
3. **Standard Linux filesystem hierarchy**: Runtime paths adhere to standard XDG Base Directory
   specifications (`$XDG_RUNTIME_DIR`, `$XDG_DATA_HOME`, `$XDG_CONFIG_HOME`) and standard system
   identity files (`/etc/os-release`, `/etc/machine-id`, `/proc/sys/kernel/random/boot_id`).
4. **Desktop presentation**: Wayland compositor with a lightweight Chromium runtime or browser-native
   client displaying the Living Canvas Rust/WASM bundle.
5. **NixOS status**: NixOS expressions, flake definitions, and legacy C++/Qt packages are frozen as
   compatibility evidence and legacy oracles only, not as active deployment targets.

## Consequences

- **Positive**: Simplified CI/CD pipelines, predictable deb packaging, direct compatibility with cloud
  VPS providers (e.g. OVH Debian 13 targets), standardized systemd integration, and elimination of
  Nix-specific runtime quirks.
- **Negative**: Requires maintaining standard Debian packaging and systemd service files instead of
  declarative Nix modules.

## Related Documents

- [Debian Build and Deployment](../DEPLOYMENT.md)
- [ADR-0037](ADR-0037-web-first-presence-and-desktop.md)
- [ADR-0038](ADR-0038-rust-first-codebase.md)
- [ADR-0006](ADR-0006-state-version-pinning.md) (Superseded)
