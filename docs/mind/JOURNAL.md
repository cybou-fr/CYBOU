<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cognitive Journal

## Purpose

The Journal is Cybou's append-only biography.

## Ownership

**Current:** opened through the Presence-owned object graph.

**Target:** only `cybou-eventd` writes `journal.db`.

## Required versions

- SQLite `PRAGMA user_version`;
- envelope schema version;
- row hash version.

## Correct append order

```text
validate
→ BEGIN IMMEDIATE
→ verify uniqueness and references
→ read tail and determine sequence
→ calculate canonical hash
→ insert contribution and evidence
→ COMMIT
```

Any failure rolls back.

## Hash v2

Hash every semantic envelope field plus previous hash and row sequence.

## Migration

- never rewrite v1 rows only to adopt v2 hashing;
- create a backup;
- migrate transactionally;
- verify after migration;
- reject unsupported newer schemas;
- do not advance the version after failure.

## Outcomes

A lifecycle that is single-terminal must be protected by both application checks and SQLite constraints.
