<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Data Ownership

One persistent resource should have one authoritative owner.

## Current

Within one `plasmashell` process and canonical state root, multiple Presence surface wrappers share
one `PresenceRuntime`.

```text
Presence wrappers
       ↓
shared PresenceRuntime
       ├── one Journal object
       ├── one Identity object/session
       ├── one Intentions object
       ├── one Predictor object
       ├── one SelfModel object
       └── one Workspace object
```

This closes duplicate ownership caused by multiple QML Presence instances in the current
architecture.

It is not yet process-level ownership: there is no `eventd`/`presenced` daemon.

## Current persistent location

On Unix:

```text
$XDG_STATE_HOME/cybou
```

with fallback:

```text
~/.local/state/cybou
```

The path is independent of the hosting Qt application's name.

Presence migrates the former AppDataLocation-derived Cybou directory into this root before opening
the default Journal. Existing target entries are never overwritten; collisions fail closed.

The desktop `desktop-layout-version` marker may coexist in the same Cybou state root and is
preserved during legacy Mind migration.

## Target owners

| Resource | Authoritative process |
|---|---|
| `journal.db` | `cybou-eventd` |
| identity state | `cybou-identityd` |
| transient workspace | `cybou-workspaced` |
| presentation snapshots | `cybou-presenced` |
| QML view state | Plasma applet |

Target runtime/cache roots remain `$XDG_RUNTIME_DIR/cybou` and `$XDG_CACHE_HOME/cybou`.

## Invariants

- QML does not open cognitive databases.
- Opening another Presence surface does not create a second current backend/session.
- A UI host rename must not move or fork persistent Mind state.
- A legacy/canonical state collision is an error, not an implicit merge policy.
- M3 must not create a second Journal owner while the old owner remains active.
