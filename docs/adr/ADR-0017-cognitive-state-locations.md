<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0017: Cognitive State Locations and Ownership

## Status

Accepted

Enforced by StatePaths and the per-owner state roots, including the legacy migration that moves
state to the canonical location rather than leaving two.

## Context

State paths derived from the transient presentation process are unstable.

## Decision

Persistent state uses `$XDG_STATE_HOME/cybou`, runtime state uses `$XDG_RUNTIME_DIR/cybou`, and caches use `$XDG_CACHE_HOME/cybou`. Each resource has one owner.

## Consequences

State remains stable when process names and UI hosts change.

## Alternatives Considered

Using transient presentation application-data paths was rejected.
