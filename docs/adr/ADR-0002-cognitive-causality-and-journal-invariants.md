<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0002: Cognitive Causality and Journal Invariants

## Status

Accepted

## Context

Journal v1 did not fully enforce causal/evidence integrity, privacy inheritance, lifecycle
uniqueness, or complete semantic hash coverage.

Protocol/Journal v2 now has implementation and migration tests for the invariants selected by this
decision.

## Decision

For new v2 contributions:

- only `Observation` is a root contribution;
- a non-root contribution requires a direct cause or evidence;
- self-causation and self-evidence are forbidden;
- null or duplicate evidence is forbidden;
- the direct cause may not also be repeated as evidence;
- cause/evidence references must already exist in the Journal;
- derived privacy may not be weaker than referenced contributions;
- terminal Outcome duplication is rejected and backed by a database constraint;
- legacy v1 rows are preserved rather than rewritten to satisfy new structural rules.

Because references can only point to already persisted contributions, new v2 writes cannot create
a forward-reference cycle.

Canonical full-envelope hashing and concrete storage/migration details are specified by ADR-0010.

## Consequences

New v2 writers are stricter while old biography remains verifiable.

Malformed new contributions fail before commit. Reference and privacy validation runs inside the
serialized write transaction. Legacy history keeps its original hash algorithm.

The current implementation still has multiple in-process Journal users; ADR-0011 remains the
separate Target decision for exclusive `eventd` ownership.

## Acceptance evidence

The current `cybou-mind` build contains focused protocol and Journal tests, including reference,
migration, tamper-detection, rollback, terminal-Outcome, and concurrent-writer coverage.

## Alternatives Considered

Using self-causation as a root marker was rejected because it creates graph cycles.

Rewriting v1 history to make it satisfy v2 structural rules was rejected because that would
replace the biography the Journal is intended to preserve.
