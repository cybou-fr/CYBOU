<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Local Cognitive IPC

## Current transport

M3 implements Qt D-Bus Event1.

```text
service   org.cybou.Mind.Event1
object    /org/cybou/Mind/Event1
interface org.cybou.Mind.Event1
```

## Current Event1 methods

```text
Ready() -> bool
SchemaVersion() -> int

Submit(envelopeCbor) -> submitResultCbor

Count() -> uint64
Head() -> bytes
Verify() -> uint64

Recent(limit) -> envelopeListCbor
Episode(correlationId) -> envelopeListCbor

Contains(messageId) -> bool
Contribution(messageId) -> envelopeCbor
EvidenceFor(messageId) -> uuidListCbor
HasOutcomeFor(causeId, originOrgan) -> bool
```

## Current signal

```text
Accepted(envelopeCbor, sequence)
```

It is emitted only after the Journal COMMIT succeeds.

## CBOR versioning

Envelope IPC carries `ipcVersion = 1`. It transports all CognitiveEnvelope fields but is separate
from Journal canonical hash bytes. An IPC representation change must not alter historical hashes.

## Client abstraction

Organs depend on `EventStore`. `EventClient` implements that contract over Event1. This is the
transport abstraction required by ADR-0013.

## Future

M4 adds versioned interfaces for the remaining process-isolated organs. Free-form organ-to-organ
natural-language chat remains prohibited.
