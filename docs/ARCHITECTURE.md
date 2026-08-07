<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cybou Architecture

## Current

```text
Plasma/QML
   │
   ▼
Presence wrappers
   │
   ▼
shared PresenceRuntime (plasmashell)
   ├── Identity
   ├── Intentions
   ├── Predictor
   ├── SelfModel
   ├── Workspace
   └── EventClient
          │
          │ org.cybou.Mind.Event1
          ▼
     cybou-eventd
          │
          ▼
       Journal v2
```

M3 changes the persistence boundary without pretending the remaining organs are already daemons.

## Event boundary

All current organ code depends on `EventStore`, not `Journal`.

Two implementations exist:

```text
Journal      low-level/local implementation, used by eventd and isolated tests
EventClient  production runtime transport to eventd
```

The semantic order is:

```text
proposal
→ durable validation and COMMIT in eventd
→ Accepted
→ Workspace admission
→ presentation notification
```

The same ordering established locally in M1 is preserved across D-Bus.

## IPC

Event1 uses:

```text
service   org.cybou.Mind.Event1
object    /org/cybou/Mind/Event1
interface org.cybou.Mind.Event1
```

Cognitive envelopes use versioned CBOR that is deliberately independent of canonical Journal hash
encoding.

## Target after M4

```text
Plasma/QML
    │
    ▼
cybou-presenced
    │
    ▼
typed local fabric
    ├── cybou-eventd
    ├── cybou-identityd
    ├── cybou-intentiond
    ├── cybou-predictord
    ├── cybou-selfd
    └── cybou-workspaced
```

`eventd` is already a real process. The others remain future process boundaries.
