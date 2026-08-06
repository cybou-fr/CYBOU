<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cybou Architecture

## Body

NixOS, Plasma, devices, processes, storage, networking, and reversible system generations.

## Mind

Typed cognitive contributions, biography, identity, intentions, predictions, self-model, attention, and future faculties.

## Presence

The presentation boundary: Plasma UI, notifications, explanations, and inspection tools. Presence does not own cognition.

## Current

```text
Plasma/QML
    │
    ▼
Presence QObject
    ├── Journal
    ├── Identity
    ├── Intentions
    ├── Predictor
    ├── SelfModel
    └── Workspace
```

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

## Migration

1. Stabilize one Presence, one Journal, and a green build.
2. Introduce Journal v2 and protocol invariants.
3. Extract eventd.
4. Move Presence outside `plasmashell`.
5. Extract organs.
6. Add health and degraded modes.
7. Add network transport.
