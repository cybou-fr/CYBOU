<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0002: Cognitive Causality and Journal Invariants

## Status

Proposed

## Context

Journal v1 does not fully enforce acyclic causality, evidence integrity, privacy inheritance, lifecycle uniqueness, or complete hash coverage.

## Decision

Protocol v2 forbids self-causation and self-evidence, requires existing prior references, defines explicit roots, prevents cause/evidence duplication, inherits restrictive privacy, and permits one terminal outcome where required. Legacy v1 rows are not rewritten.

## Consequences

New writers become stricter while old biography remains verifiable. Implementation and migration tests are required before acceptance.

## Alternatives Considered

Using self-causation as a root marker was rejected because it creates graph cycles.
