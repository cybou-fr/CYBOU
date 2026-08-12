<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0015: Terminal Outcome Semantics

## Status

Accepted

Enforced by the Journal, which carries a unique index admitting one terminal Outcome per cause,
and by lifecycled, which refuses to complete a run without one.

## Context

A generic Outcome can be attached repeatedly or to the wrong lifecycle entity.

## Decision

Every terminal outcome identifies its domain and target. Intention and Prediction lifecycles allow at most one terminal outcome unless a future protocol defines revisions.

## Consequences

Commitment state and calibration remain deterministic, backed by database constraints.

## Alternatives Considered

UI-only duplicate prevention was rejected.
