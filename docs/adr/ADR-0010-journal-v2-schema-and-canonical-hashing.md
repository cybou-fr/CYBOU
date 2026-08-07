<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0010: Journal v2 Schema and Canonical Hashing

## Status

Accepted

## Context

Journal v1 had no explicit database or hash version, stored evidence as comma-separated text,
and omitted semantic fields from the hash. Reading the tail before acquiring a write lock also
allowed concurrent writers to derive the same sequence and previous hash.

## Decision

Journal v2 introduces:

- SQLite `PRAGMA user_version = 2`;
- per-row `schema_version` and `hash_version`;
- canonical big-endian, length-prefixed encoding of every semantic envelope field;
- evidence relations in `contribution_evidence`;
- `BEGIN IMMEDIATE` before reference validation and tail selection;
- a partial unique index allowing one terminal `Outcome` per cause;
- transactional v1→v2 migration with a `journal.db.v1.bak` backup;
- version-aware verification that preserves the exact v1 hash algorithm.

Existing v1 rows retain schema/hash version 1 and their original hashes. The first v2 row chains
to the final v1 hash. Historical v1 contributions are not retroactively rejected by v2 structural
rules.

## Consequences

Mutation of privacy, confidence, evidence, origin node, monotonic time, capability scope, or any
other semantic field is detectable for v2 rows. Multiple local writers serialize correctly.
Malformed legacy evidence and duplicate legacy terminal Outcomes make migration fail closed.

The migration backup is retained for manual recovery and is not deleted automatically.

## Alternatives Considered

Rehashing old history was rejected because it would replace the biography it claims to protect.
Application-only Outcome checks were rejected because they are race-prone without a database
constraint.
