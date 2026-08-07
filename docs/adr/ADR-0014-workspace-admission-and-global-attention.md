<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0014: Workspace Admission and Global Attention

## Status

Accepted

## Context

Before M1/M4, durable contributions could bypass Workspace and leave attention stale.

M1 introduced post-COMMIT accepted admission locally. M3 moved durable acceptance behind eventd.
M4 moved Workspace into a real workspaced process.

## Decision

Every accepted durable contribution is signaled by eventd and considered by workspaced.

Workspace owns bounded transient context and deterministic salience. It does not own biography.

Presentation observes Workspace1 `Changed` after admission rather than presenting a raw Event1
signal directly.

## Consequences

Attention follows accepted durable events, remains reconstructible from Event1 history, and has one
process-level owner.

Restarting workspaced can rehydrate the bounded moment without changing Journal history.

## Evidence

M1 unit tests verify accepted-only admission and idempotence.

The M4 process integration test verifies that accepted events cross Event1 into the separate
workspaced process and become visible through Presence.

## Alternatives Considered

Periodic UI refresh without backend admission was rejected.

Letting presenced maintain a second attention copy was rejected because it would create two owners.
