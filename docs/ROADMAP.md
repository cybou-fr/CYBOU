<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Roadmap

## M0 — Green Build

**Status: ongoing gate discipline.**

## M1 — One Presence, One Journal

**Status: Complete for the in-process runtime.**

Shared Presence runtime, live accepted-contribution Workspace, and stable persistent state root.

## M2 — Journal v2

**Status: Complete.**

Versioned schema/hash, canonical encoding, migration, validation, serialized writes, normalized
evidence, and terminal Outcome constraints.

## M3 — eventd

**Status: Complete.**

Implemented:

- `cybou-eventd` executable;
- normal production ownership of `journal.db` moved out of `plasmashell`;
- `EventStore` transport abstraction;
- `EventClient` Qt D-Bus client;
- versioned `org.cybou.Mind.Event1` interface;
- versioned CBOR CognitiveEnvelope transport;
- query IPC needed by current organs;
- post-COMMIT `Accepted` signal bridged from Journal to D-Bus and back to Workspace;
- D-Bus service activation from the Nix package;
- integration tests under an isolated session bus;
- no silent local-SQLite fallback when eventd is unavailable.

## M4 — Process-Isolated Organs

**Status: Next.**

Extract Identity, Intentions, Predictor, SelfModel, Workspace, and Presence into user services.
`presenced` becomes the shared presentation backend and survives Plasma surface restarts.

## M5 — Continuity

**Status: Planned.**

Reboot-surviving intention and verified identity/architecture transitions.

## M6 — Degraded Modes

**Status: Planned.**

Health, capability deficits, recovery, and reconciliation.

## M7 — Distributed Node Prototype

**Status: Planned.**

Selective replication and partition handling.

## M8 — Optional Language Faculty

**Status: Planned.**

Language is a faculty, not identity or executor.

## M9 — Authorized Action Boundary

**Status: Planned.**

Typed proposal, criticism, authorization, build/test, confirmation, execution, outcome, rollback.
