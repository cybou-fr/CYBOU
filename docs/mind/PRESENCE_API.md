<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Presence API

Presence is the normal UI interface to Mind.

## Current runtime lifecycle

A `Presence` QObject is now a presentation wrapper. Wrappers using the same canonical data root
share one process-local `PresenceRuntime`.

Consequences:

- a second Plasma/QML Presence object does not increment the Identity session;
- surfaces see the same Journal, obligations, predictions, and Workspace;
- accepted Journal events notify every subscribed wrapper;
- after the final wrapper is destroyed, a later runtime starts a normal new Identity session.

This is M1's in-process solution. Future surfaces connect to `presenced` instead of sharing a C++
object directly.

## Current properties

```text
awake
narration
obligations
attention
contributions
stats
identityState
calibrations
coalitions
moment
```

## Current projections

```text
activity(limit)
detailedObligations()
stats()
identityState()
calibrations()
coalitions()
moment()
```

## Current commands

```text
promise(description)
reflect()
fulfillIndex(index)
abandonIndex(index)
observe(subject, value)
predict(subject)
```

Successful biography-changing commands no longer need ad-hoc `changed()` emissions to force a
Workspace refresh. Their durable Journal contributions generate accepted events, Workspace updates
first, and Presence wrappers are then notified.

## Target API direction

`presenced` should retain stable snapshot/command semantics across D-Bus. Command IDs should move
from UI list indexes to stable contribution IDs. Refresh/read operations must never write
biography.
