<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Process Model

## Current

Mind is not process-isolated.

```text
plasmashell
└── Presence
    ├── Journal
    ├── Identity
    ├── Intentions
    ├── Predictor
    ├── SelfModel
    └── Workspace
```

The daemon-like source directory names describe intended component boundaries, but the current
build produces libraries/QML integration rather than one user service per organ.

A `plasmashell` restart therefore also destroys the current in-process Mind object graph.

## Target

```text
cybou-eventd
cybou-identityd
cybou-intentiond
cybou-predictord
cybou-selfd
cybou-workspaced
cybou-presenced
```

Each cognitive process is intended to be a `systemd --user` service and a separate
`QCoreApplication` or appropriate Qt application, with GUI-specific code confined to the
presentation boundary.

## Target lifecycle

- explicit startup dependencies;
- restart after recoverable failure;
- reconstruction from owned state or Journal;
- capability-deficit reporting;
- Presence reconnect after UI restart;
- Plasma restart does not restart the Mind;
- one authoritative owner for each persistent resource.

## Target health model

```text
Available
Starting
Healthy
Degraded
Unavailable
Recovering
```

These process-level health states are not implemented by the current in-process prototype.
