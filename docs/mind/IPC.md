<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Local Cognitive IPC

## Interfaces

```text
org.cybou.Mind.Event1
org.cybou.Mind.Health1
org.cybou.Mind.Lifecycle1
org.cybou.Mind.Identity1
org.cybou.Mind.Intention1
org.cybou.Mind.Predictor1
org.cybou.Mind.Self1
org.cybou.Mind.Workspace1
org.cybou.Mind.Presence1
```

## Encoding

Event1 keeps the M3 versioned CognitiveEnvelope CBOR.

Other organ projections use fabric CBOR version 1:

```text
{
  version: 1,
  value: <typed QVariant/QCbor value>
}
```

This representation is not canonical Journal hashing.

## Baseline service contract

Every organ exposes at least:

```text
Ready() -> bool
Health() -> string
```

P6.1 defines schema-v1 `CapabilitySnapshot` CBOR and P6.2 serves it from Health1. Health1 exposes
`Ready`, aggregate `Health`, `LastError`, `HasSnapshot`, `Snapshot`, `Refresh`, and `Changed`.
The snapshot uses its own versioned protocol encoding rather than the generic fabric wrapper.

## Resilient asynchronous calls

Calls that must not block a shell or owner event loop use the typed policy in
[RPC Resilience](RPC_RESILIENCE.md). Retry requires explicit read-only or idempotent semantics;
non-idempotent timeout remains `UnknownOutcome` and is never automatically replayed.

## Presentation signal ordering

workspaced emits `Workspace1.Changed` after it admits an Event1 accepted contribution.
presenced converts that to `Presence1.Changed`. QML then refreshes its cached snapshot.
