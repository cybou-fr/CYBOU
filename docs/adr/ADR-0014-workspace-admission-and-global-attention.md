<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0014: Workspace Admission and Global Attention

## Status

Proposed

## Context

Journal contributions can currently bypass Workspace and leave attention stale.

## Decision

Every accepted contribution is signaled by eventd and considered by workspaced. Workspace owns bounded transient context and deterministic salience.

## Consequences

Attention follows accepted durable events and remains reconstructible.

## Alternatives Considered

Periodic UI refresh without backend admission was rejected.
