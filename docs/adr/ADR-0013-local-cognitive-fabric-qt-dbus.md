<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0013: Local Cognitive Fabric Uses Qt D-Bus

## Status

Accepted

## Context

Process-isolated local Mind components need inspectable typed IPC integrated with Qt.

M3 is the first concrete process boundary: `cybou-eventd`.

## Decision

Use versioned Qt D-Bus interfaces locally and versioned CBOR where an extensible typed payload is
needed. Domain logic depends on a transport abstraction rather than directly on QDBus classes.

The first implemented interface is:

```text
org.cybou.Mind.Event1
```

CognitiveEnvelope transport is CBOR `ipcVersion = 1`. Primitive query parameters/results remain
typed D-Bus values.

Current organs depend on `EventStore`; `EventClient` is the D-Bus implementation.

## Consequences

Local IPC is inspectable, versioned, and separated from domain logic.

The CBOR IPC encoding is intentionally independent of canonical Journal hashing.

M4 can add Identity/Intention/Prediction/Self/Workspace/Presence interfaces without changing the
transport decision.

## Alternatives Considered

Free-form natural-language organ chat was rejected.

Unversioned JSON blobs were rejected because they provide a weaker protocol boundary and invite
silent schema drift.
