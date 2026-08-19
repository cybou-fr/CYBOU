<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0006: State Version Pinning

## Status

Superseded by [ADR-0039](ADR-0039-debian-13-base-system.md).

The decision to pin NixOS state version applied when NixOS was the target platform.
Cybou has transitioned to Debian 13 (Trixie) as the active deployment target, so NixOS
`system.stateVersion` is no longer applicable.

## Context
NixOS allows system configurations to specify a `system.stateVersion` which pins the system to a specific NixOS version. This prevents automatic upgrades that might break compatibility.

When upgrading NixOS, stateful services (databases, etc.) may need migrations. If these migrations fail, the system can become unusable. ADR-0006 establishes how Cybou handles this.

## Decision
**Pin `system.stateVersion` to the NixOS version used for development (26.05).**

This means:
- `system.stateVersion = "26.05";` in all configurations
- Never automatically bump this version
- Manual intervention required for major NixOS upgrades

## Rationale

### Why Pin?
1. **Stability**: Prevents accidental upgrades that break the system
2. **Control**: Upgrades are explicit and tested
3. **Reproducibility**: All systems run the same NixOS version
4. **Safety**: Avoids untested migration paths

### Pinning Strategy
- State version is pinned to the **NixOS release** (26.05), not the exact Git commit
- This allows bug fixes within the 26.05 release to be applied
- Major version upgrades (26.05 → 26.11) require manual intervention

## Implementation

In all NixOS configurations:
```nix
system.stateVersion = "26.05";
```

In flake.nix:
```nix
nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
```

## Consequences

### Positive
- Stable, predictable system behavior
- No surprise upgrades
- Clear upgrade path (manual, tested)
- Matches NixOS stable release cycle

### Negative
- Manual intervention required for major upgrades
- May lag behind latest NixOS features
- Need to test migrations before upgrading

## Upgrade Procedure

1. Create new branch for upgrade testing
2. Update flake.nix to point to new NixOS version
3. Update `system.stateVersion` in all configurations
4. Test all services, especially stateful ones
5. Document migration steps if needed
6. Merge after successful testing

## Related
- modules/base.nix - Pins stateful-service defaults
- flake.nix - Uses NixOS 26.05
- ADR-0004 - CI workflow (tests must pass before upgrade)
