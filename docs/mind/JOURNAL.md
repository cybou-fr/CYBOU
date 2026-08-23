<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cognitive Journal

## Current owner

`cybou-eventd` is the normal production owner of `journal.db`.

The remaining organs do not link their domain APIs to Journal. They submit and query through the
`EventStore` abstraction, whose production implementation is `EventClient`.

## Durable append

```text
Event1 Submit
→ decode versioned CBOR
→ Journal::append
→ structural/reference/privacy validation
→ BEGIN IMMEDIATE
→ assign sequence/hash
→ insert contribution/evidence
→ COMMIT
→ Journal::accepted
→ Event1 Accepted
```

No accepted signal is emitted on rollback.

## Journal v2

Journal v2 behaviour is unchanged: schema/hash versions, v1 migration and backup, canonical
hashing, normalized evidence, reference/privacy validation, writer serialization, and terminal
Outcome uniqueness remain inside the low-level Journal implementation.

## Test seam

Unit tests may instantiate a Journal directly with a temporary path. This is not the default
production Presence path and does not weaken eventd ownership of the canonical user Journal.
