<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0011: Single-Writer Event Journal

## Status

Accepted

## Context

Multiple production writers make ordering, ownership, and validation ambiguous. M1 established the
semantic boundary `COMMIT -> accepted` locally, which allows M3 to move ownership without changing
organ semantics.

## Decision

Only `cybou-eventd` writes the canonical production `journal.db`.

Other current organs depend on `EventStore` and use `EventClient` to submit proposals and query
history through versioned Event1 D-Bus IPC.

`cybou-eventd`:

- owns the production Journal object/SQLite connection;
- performs Journal validation and sequence/hash assignment;
- appends atomically;
- emits Event1 `Accepted` only after Journal COMMIT.

Direct Journal construction remains permitted in isolated unit tests and explicit temporary test
runtimes; it is not a frontend presentation path.

## Consequences

Persistence ownership and ordering are explicit. Eventd is now a real service/failure domain.

If eventd is unavailable, the default runtime does not silently open SQLite itself.

M4 can isolate the remaining organs while preserving the `EventStore` domain contract.

## Evidence

The `eventd-integration` test runs with a private D-Bus session and verifies submission, accepted
ordering, queries, Presence use of eventd, service-name exclusivity, and no local fallback.

## Alternatives Considered

SQLite locking in every production organ was rejected as the long-term design.

A write-only broker with organs still opening SQLite for queries was rejected because it would
leave Journal ownership ambiguous.
