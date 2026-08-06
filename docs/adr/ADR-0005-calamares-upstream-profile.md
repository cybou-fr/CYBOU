<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0005: Calamares Installer - Upstream Profile

## Status
Accepted

## Context
Cybou needs an installer for the live ISO. Options include:
1. Fork Calamares and heavily customize it
2. Use upstream Calamares with minimal branding changes
3. Create a custom installer from scratch
4. Use another existing installer (Anaconda, Ubiquity, etc.)

The decision affects:
- Maintenance burden
- Branding consistency
- User experience
- Release timeline

## Decision
**Use upstream Calamares from NixOS graphical installation-CD profile with minimal branding changes.**

Branding is capped at:
- Product name: "Cybou" (instead of "NixOS")
- Aperture logo (cybou-aperture.svg)
- Token colors from design-tokens.json

Deep installer theming is **optional and must not block the release**.

## Rationale

### Why Upstream Calamares?
1. **Maintenance**: Upstream maintains Calamares, reducing our burden
2. **Stability**: Well-tested installer used by NixOS
3. **Compatibility**: Already integrated with NixOS
4. **Features**: Full feature set (partitioning, user creation, etc.)

### Why Minimal Branding?
1. **Risk Reduction**: Deep theming can break with Calamares updates
2. **Release Velocity**: Minimal changes mean faster releases
3. **User Expectations**: Users familiar with NixOS installer will recognize it
4. **Focus**: Allows us to focus on Cybou-specific features

## Implementation Details

### Branding Changes (systems/iso-calamares-branding.nix)
- Replace `nix-snowflake.svg` with `cybou-aperture.svg`
- Replace `white.png` with rasterized Aperture logo (256x256)
- Update `versionedName`, `shortVersionedName`, `shortProductName`, `bootloaderEntryName` to "Cybou"
- Update sidebar colors from design-tokens.json:
  - Surface: `#171D27`
  - Text: `#70E1C8`
  - Canvas: `#0A0D12`
  - Accent: `#F2F5F8`

### What NOT to Change
- `componentName`: Must remain "nixos" for Calamares to find the branding
- Upstream URLs: No real Cybou website exists yet
- File formats: Don't write SVG into PNG files (renders as nothing)

## Consequences

### Positive
- Fast implementation (reuse existing work)
- Low maintenance burden
- Familiar to NixOS users
- Can be enhanced later

### Negative
- Limited customization initially
- Looks similar to NixOS installer
- Some branding inconsistencies possible

## Alternatives Considered

### Alternative: Fork Calamares
- Full control over installer
- **Rejected**: High maintenance burden, diverges from upstream

### Alternative: Custom Installer
- Tailored to Cybou's needs
- **Rejected**: Significant development effort, delays release

### Alternative: Different Installer
- Use Anaconda, Ubiquity, etc.
- **Rejected**: Not integrated with NixOS, more work

## Related
- systems/iso.nix - ISO configuration using Calamares
- systems/iso-calamares-branding.nix - Branding customizations
- ADR-0008 - Similar isolation principle for Mind Dock
