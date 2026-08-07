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

Presence is the only normal surface boundary, but the current implementation still constructs and
owns the in-process Mind object graph. The Target architecture removes that lifecycle ownership
from the Plasma process.

## Current

```text
plasmashell
    │
    ▼
Presence QObject
    ├── Journal v2
    ├── Identity
    ├── Intentions
    ├── Predictor
    ├── SelfModel
    └── Workspace
```

Current properties:

- the Presence QML type is loaded in `plasmashell`;
- the components above are C++ objects/libraries, not independent daemons;
- new Journal writes use schema/hash v2 and are transactionally serialized;
- multiple components currently write through the shared Journal abstraction;
- Workspace can rehydrate bounded recent history;
- direct organ Journal writes do not yet form one live accepted-contribution stream;
- persistent Mind state is still derived from the hosting Qt application-data location.

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

- `cybou-eventd` exclusively owns durable Journal writes;
- organs communicate through typed, versioned local IPC;
- Presence projects state but does not own cognition;
- Mind survives a Plasma restart;
- each persistent resource has one authoritative owner;
- persistent Mind state uses stable XDG locations rather than a UI host-derived path;
- failures become explicit capability deficits.

## Current migration position

Completed:

1. typed protocol and in-process Mind baseline;
2. causal/reference/privacy invariants for v2 contributions;
3. Journal v2 schema, canonical hashing, migration, atomic writer serialization, and terminal
   Outcome uniqueness;
4. Presence Mind Dock surface and read projections.

Open before process extraction is considered mature:

1. complete M1 with one Presence backend, live accepted-contribution admission, and stable state
   ownership;
2. extract `eventd`;
3. move Presence lifecycle outside `plasmashell`;
4. extract remaining organs;
5. add health and degraded modes;
6. add network transport only after local ownership and recovery are proven.

## Naming rule

A directory named `eventd`, `identityd`, `presenced`, and so on does not prove a daemon exists.
Process topology is determined by built executables/services and runtime tests, not by source
directory names.
