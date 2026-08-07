<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Current State

Status date: 2026-08-07.

This file describes implemented behavior only. Target architecture belongs in the architecture,
roadmap, and ADR documents and must be labeled accordingly.

## Implemented

### Body and desktop

- NixOS 26.05 / KDE Plasma 6 / Wayland foundation;
- Cybou Horizon branding;
- VM, ISO, and Hyper-V development targets;
- first-login layout versioning;
- right-side Mind Dock;
- static KDE package and QML API validation.

### Mind runtime

- in-process Identity, Intentions, Predictor, SelfModel, Workspace, Journal, and Presence;
- multiple Presence QObject/QML surfaces for one canonical data root share one in-process runtime;
- a second surface does not create a second Journal or increment the current Identity session;
- the shared runtime remains alive while at least one Presence wrapper still references it;
- after all wrappers are gone, a later runtime begins a new Identity session normally.

### Live accepted-contribution path

`Journal::append()` now emits:

```text
accepted(envelope, sequence)
```

only after a successful COMMIT.

Workspace subscribes to that event and admits the contribution idempotently. Therefore direct
writes from Identity, Intentions, Predictor, SelfModel, or Presence update the live Workspace
without rereading recent Journal history after each action.

Startup/recovery still uses `Workspace::rehydrate()`.

### Persistent state root

On Unix, the default Mind root is:

```text
$XDG_STATE_HOME/cybou
```

with `~/.local/state/cybou` as the XDG fallback.

Before opening the canonical Journal, Presence checks the previous host-derived AppDataLocation
root. Legacy entries are migrated into the canonical root without overwriting existing target
entries. A collision fails closed. Existing entries such as `desktop-layout-version` are
preserved.

Explicit data directories passed by tests/tools remain supported.

### Journal v2

- SQLite database schema v2;
- canonical hash v2;
- v1 hash preservation;
- v1→v2 backup/migration;
- normalized evidence;
- cause/evidence existence validation;
- privacy inheritance;
- `BEGIN IMMEDIATE` writer serialization;
- terminal Outcome uniqueness.

## Current process topology

```text
plasmashell
├── Presence surface A ─┐
├── Presence surface B ─┼── shared PresenceRuntime
└── Presence surface N ─┘        ├── Journal v2
                                 ├── Identity
                                 ├── Intentions
                                 ├── Predictor
                                 ├── SelfModel
                                 └── Workspace
```

The shared runtime is process-local. There is still no `presenced` or `eventd` daemon.

## Current event ordering

```text
organ / Presence
       │
       ▼
 Journal::append
       │
 validate + BEGIN IMMEDIATE
       │
      COMMIT
       │
       ▼
 Journal::accepted
       │
       ├──► Workspace::accept
       │
       └──► Presence wrappers changed()
```

Workspace installs its Journal subscription when it is created, before Presence wrappers
subscribe, so a surface handling the accepted event sees the updated Workspace.

## Automated tests

The Mind package now has ten CTest suites:

```text
protocol
journal
identity
intentions
predictor
selfmodel
workspace
presence
presence-extended
m1-runtime
```

The M1 suite specifically covers:

- accepted events only after successful durable append;
- direct Journal writes becoming visible to Workspace immediately;
- idempotent Workspace admission;
- two Presence surfaces sharing one Identity session/runtime;
- runtime lifetime across multiple surfaces;
- XDG state-root selection;
- legacy state merge and collision fail-closed behavior.

## Not implemented

- `cybou-eventd`;
- an exclusive process-level Journal owner;
- stable local D-Bus contracts;
- process-isolated organs;
- cross-process `presenced`;
- Mind survival across `plasmashell` restart;
- process-level health/degraded-mode reporting;
- inter-node distribution;
- language-model faculties;
- authorized autonomous operating-system mutation.

## Milestone position

- **M0:** fast gate green; heavy full/VM validation remains a separate gate.
- **M1:** complete for the current in-process architecture.
- **M2:** complete.
- **M3:** next — extract exclusive Journal ownership into `eventd`.

## Documentation rule

Current claims require code/tests. Future behavior must be labeled Target, Proposed, Planned, or
Pending.
