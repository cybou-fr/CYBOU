<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Local Cognitive IPC

## Interfaces

```text
org.cybou.Mind.Event1
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

P6.1 additionally defines schema-v1 `CapabilitySnapshot` CBOR in
[Capability and Health Contract](HEALTH.md). It is not yet served by D-Bus; `Health1` and its owner
belong to P6.2.

## Presentation signal ordering

workspaced emits `Workspace1.Changed` after it admits an Event1 accepted contribution.
presenced converts that to `Presence1.Changed`. QML then refreshes its cached snapshot.
