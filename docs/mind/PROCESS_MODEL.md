<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Process Model

## Current after M3

There are now two failure domains:

```text
cybou-eventd
└── Journal v2

plasmashell
└── shared PresenceRuntime
    ├── EventClient
    ├── Identity
    ├── Intentions
    ├── Predictor
    ├── SelfModel
    └── Workspace
```

`cybou-eventd` is D-Bus-activated and is no longer a library object inside Presence.

The remaining daemon-like source directories are still in-process components.

## M4 target

```text
cybou-eventd
cybou-identityd
cybou-intentiond
cybou-predictord
cybou-selfd
cybou-workspaced
cybou-presenced
```

M4 moves lifecycle ownership out of `plasmashell`; M6 later adds explicit health/degraded states.
