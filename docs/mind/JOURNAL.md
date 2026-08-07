<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cognitive Journal

## Purpose

The Journal is Cybou's append-only biography.

## Ownership

**Current:** one Journal object is shared by the current in-process Presence runtime. New Presence
surface wrappers for the same state root reuse that runtime.

**Target:** only `cybou-eventd` opens the durable Journal for writes.

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
→ insert contribution/evidence
→ COMMIT
→ emit accepted(envelope, sequence)
```

Any failure before COMMIT rolls back and emits no accepted event.

## Accepted contribution event

`Journal::accepted` is the current local runtime boundary between durability and live cognition.

It exists so consumers never infer acceptance from an attempted proposal:

```text
append returned 0
→ no accepted signal
→ no Workspace admission
```

Workspace consumes this event today. M3 should preserve the semantics while moving the durable
append behind `eventd`/IPC.

## Hash v2

The hash binds sequence, previous hash, and canonical envelope bytes including schema, IDs,
origin, kind, wall/monotonic/logical time, confidence, evidence, payload, privacy, and capability
scope.

## Migration

Opening v1 creates `journal.db.v1.bak`, then migrates transactionally. V1 hashes are never
rewritten. Migration fails closed for malformed legacy evidence, duplicate terminal Outcomes,
damaged legacy history, partial schemas, or unsupported newer schema versions.

## Remaining architecture step

M3 replaces the in-process Journal owner with `cybou-eventd`; the accepted-event contract is
intended to survive that extraction.
