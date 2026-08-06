<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0019: Degraded Modes and Capability Deficits

## Status

Proposed

## Context

A distributed Mind must remain honest when an organ or node is unavailable.

## Decision

Expose Healthy, Degraded, Isolated, Recovering, and Conflicted. Each failure maps to a specific unavailable capability shown by Presence.

## Consequences

Partial failure no longer becomes fictional success.

## Alternatives Considered

A generic awake flag was rejected as sufficient health reporting.
