<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Data Ownership

One persistent resource should have one authoritative owner. The repository does not yet implement
the final ownership topology.

## Current

```text
plasmashell
└── Presence object graph
    ├── Journal
    ├── Identity state
    ├── Intentions
    ├── Predictor
    ├── SelfModel
    └── Workspace
```

Current behavior:

- Presence constructs the object graph;
- Identity, Intentions, Predictor, SelfModel, and Presence can use the shared Journal abstraction;
- there is no `eventd` process and therefore no exclusive process-level Journal owner;
- SQLite `BEGIN IMMEDIATE` serializes current write transactions;
- Workspace state is transient/in-memory and can rehydrate from Journal history;
- QML talks to Presence rather than opening `journal.db` directly;
- Mind persistence currently uses a path derived from Qt `QStandardPaths::AppDataLocation` plus
  `cybou`, so its location still depends on the hosting application identity.

The desktop-layout version marker is a separate resource and already uses
`$XDG_STATE_HOME/cybou`.

## Target

| Resource | Authoritative owner |
|---|---|
| `journal.db` | `cybou-eventd` |
| identity state | `cybou-identityd` |
| transient workspace | `cybou-workspaced` |
| presentation snapshots | `cybou-presenced` |
| QML view state | Plasma applet |
| component cache | creating component |

Target locations:

```text
persistent state  $XDG_STATE_HOME/cybou
runtime state     $XDG_RUNTIME_DIR/cybou
cache             $XDG_CACHE_HOME/cybou
```

## Invariants

- QML must not open cognitive databases.
- A presentation surface must not become the authoritative owner of biography.
- Shared libraries must not hide an additional mutable copy of persistent cognitive state.
- Opening another panel or tab must not create a new identity session.
- Moving Presence out of `plasmashell` must not silently move or fork the subject's biography.

ADR-0017 defines the target state-location decision; implementation is still pending.
