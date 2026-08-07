<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Local Cognitive IPC

## Interfaces

```text
org.cybou.Mind.Event1
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

## Lifecycle contract

Every organ exposes at least:

```text
Ready() -> bool
Health() -> string
```

## Presentation signal ordering

workspaced emits `Workspace1.Changed` after it admits an Event1 accepted contribution.
presenced converts that to `Presence1.Changed`. QML then refreshes its cached snapshot.
