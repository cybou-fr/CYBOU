<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0011: Single-Writer Event Journal

## Status

Proposed

## Context

Multiple writers make ordering, concurrency, and validation ambiguous.

## Decision

Only eventd writes journal.db. Other organs submit proposals. eventd validates, assigns sequence, appends, and signals accepted contributions.

## Consequences

Ownership and ordering become explicit. eventd becomes a critical service requiring degraded-mode behavior.

## Alternatives Considered

SQLite locking in every organ was rejected as the long-term design.
