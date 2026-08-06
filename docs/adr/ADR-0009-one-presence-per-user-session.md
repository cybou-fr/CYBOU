<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0009: One Presence Instance per User Session

## Status

Proposed

## Context

Multiple QML components can instantiate Presence independently and open the same state.

## Decision

One Presentation backend exists per user session. Tabs receive that shared backend. Future surfaces connect to presenced instead of creating Mind objects.

## Consequences

Opening a tab no longer creates a new identity session or database writer.

## Alternatives Considered

One Presence per tab was rejected.
