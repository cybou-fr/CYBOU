<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Presence API

Presence remains the normal QML/UI boundary.

## Default runtime

Default/QML Presence uses a shared process-local PresenceRuntime whose event backend is
`EventClient`.

It does not open the canonical Journal. Commands and projections reach `cybou-eventd` through
Event1.

Multiple Presence surfaces in one `plasmashell` process still share one runtime and Identity
session as established by M1.

## Explicit local constructor

`Presence(dataDir)` remains for isolated tests/tools and uses a temporary/local Journal backend.
It is not the QML production constructor.

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

## Current commands

```text
promise(description)
reflect()
fulfillIndex(index)
abandonIndex(index)
observe(subject, value)
predict(subject)
```

Successful durable changes propagate back through Event1 `Accepted` and then update Workspace and
Presence notifications.

M4 replaces this process-local presentation runtime with `presenced`.
