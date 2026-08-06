<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0020: Presence Surface for v0.1

## Status

Proposed

## Context

ADR-0008 selects a separate right-side dock, but duplicate applet instances and lifecycle remain unstable.

## Decision

For v0.1 use one top-panel Presence applet with compact and full representations backed by one Presence instance. Reconsider a separate dock after presenced exists.

## Consequences

The first release has one clear UI lifecycle and lower Plasma integration risk.

## Alternatives Considered

A second panel containing another Presence instance is deferred.

## Related

If accepted, this ADR supersedes ADR-0008.
