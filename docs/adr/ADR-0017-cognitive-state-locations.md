<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0017: Cognitive State Locations and Ownership

## Status

Proposed

## Context

State paths derived from the hosting Plasma process are unstable.

## Decision

Persistent state uses `$XDG_STATE_HOME/cybou`, runtime state uses `$XDG_RUNTIME_DIR/cybou`, and caches use `$XDG_CACHE_HOME/cybou`. Each resource has one owner.

## Consequences

State remains stable when process names and UI hosts change.

## Alternatives Considered

Using plasmashell application-data paths was rejected.
