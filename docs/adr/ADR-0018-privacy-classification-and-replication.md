<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0018: Privacy Classification and Replication

## Status

Proposed

## Context

Distributed operation requires formal rules for what may leave a device.

## Decision

Use Local, Node, Household, and Public. Derived data inherits the most restrictive source. Replication requires explicit trust and compatible policy.

## Consequences

Privacy becomes enforceable at protocol and transport boundaries.

## Alternatives Considered

Treating privacy as display metadata only was rejected.
