<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cognitive Journal

## Purpose

The Journal is Cybou's append-only biography.

## Ownership

**Current:** opened through the Presence-owned object graph. Multiple local connections are
serialized with `BEGIN IMMEDIATE`.

**Target:** only `cybou-eventd` writes `journal.db`.

## Current versions

- database schema: `PRAGMA user_version = 2`;
- new envelope schema: `2`;
- new row hash: `2`;
- migrated historical rows: envelope/hash version `1`.

## Append order

```text
structural validation
→ BEGIN IMMEDIATE
→ reject duplicate messageId
→ verify cause/evidence existence and privacy
→ reject an existing terminal Outcome
→ read tail and determine sequence
→ calculate canonical v2 hash
→ insert contribution
→ insert evidence relations
→ COMMIT
```

Any failure rolls back.

## Hash v2

The hash binds sequence, previous hash, and canonical envelope bytes. Canonical encoding covers:

- envelope schema version;
- message, correlation, and causation UUIDs;
- organ and node;
- contribution kind;
- UTC wall time in milliseconds;
- monotonic and logical clocks;
- IEEE-754 confidence bits;
- evidence as a sorted semantic set;
- exact payload bytes;
- privacy;
- capability scope.

Evidence keeps its original ordinal in SQLite for round trips, while hashing sorts UUID bytes so
semantically equivalent evidence sets hash identically.

## Migration

Opening a v1 database creates `journal.db.v1.bak` with `VACUUM INTO`, then migrates in one
transaction. V1 hashes are never rewritten. Migration fails closed when:

- evidence contains an invalid, duplicate, or missing UUID;
- more than one historical Outcome targets the same cause;
- the old hash chain is already damaged;
- the database declares a newer unsupported schema.

## Remaining architecture step

ADR-0011 will replace multiple in-process writers with a single `cybou-eventd` owner. Journal v2
makes the current implementation safe enough to reach that boundary; it does not replace it.
