<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cybou Architecture

## Body

NixOS, Plasma, devices, processes, storage, networking, and reversible system generations.

## Mind

Typed cognitive contributions, biography, identity, intentions, predictions, self-model,
attention, and future faculties.

## Presence

The presentation boundary: Plasma UI, explanations, inspection, and user commands.

## Current

```text
plasmashell
├── Presence wrapper ─┐
├── Presence wrapper ─┼── one shared in-process PresenceRuntime per canonical state root
└── Presence wrapper ─┘        │
                               ├── Journal v2
                               ├── Identity
                               ├── Intentions
                               ├── Predictor
                               ├── SelfModel
                               └── Workspace
```

Current properties:

- QML may instantiate more than one Presence wrapper without creating more than one current Mind
  backend for the same data root;
- the shared runtime is process-local and dies when its last wrapper is destroyed;
- new default state is stable under `$XDG_STATE_HOME/cybou` on Unix;
- Journal emits `accepted` only after successful COMMIT;
- Workspace follows that accepted stream live;
- `rehydrate()` is a startup/recovery mechanism rather than the normal update loop;
- organs remain in-process C++ components and share one Journal object in the Presence runtime.

## Current local cognitive flow

```text
proposal / organ action
        ↓
Journal v2 validation
        ↓
BEGIN IMMEDIATE
        ↓
COMMIT
        ↓
accepted(envelope, seq)
        ↓
Workspace admission + Presence notification
```

This ordering is the local precursor of the M3 eventd contract.

## Target

```text
Plasma/QML
    │
    ▼
cybou-presenced
    │
    ▼
Typed cognitive fabric
    ├── cybou-eventd
    ├── cybou-identityd
    ├── cybou-intentiond
    ├── cybou-predictord
    ├── cybou-selfd
    └── cybou-workspaced
```

Target properties:

- `eventd` exclusively owns the durable Journal;
- accepted-contribution semantics survive the move across IPC;
- Presence projects state rather than owning cognitive lifecycle;
- Mind survives Plasma restart;
- persistent resources have one process-level owner;
- failures become explicit capability deficits.

## Migration position

Completed:

1. typed protocol and in-process Mind baseline;
2. causal/reference/privacy invariants;
3. Journal v2;
4. right-side Presence/Mind Dock;
5. M1 shared in-process Presence runtime;
6. stable canonical state root with legacy migration;
7. live Workspace admission from post-COMMIT accepted events.

Next:

1. extract Journal ownership into `eventd`;
2. expose the accepted stream through typed IPC;
3. extract Presence lifecycle from `plasmashell`;
4. isolate remaining organs;
5. add health/degraded-mode semantics.

## Naming rule

A source directory ending in `d` does not prove a daemon exists. Process topology is determined by
built executables/services and runtime tests.
