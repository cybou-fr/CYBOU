<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Testing Strategy

## Layers

- protocol invariant tests;
- Journal integrity, migration, concurrency, and rollback tests;
- organ lifecycle tests;
- Presence projection tests;
- QML construction and module-loading tests;
- Nix package tests;
- NixOS VM continuity and failure tests.

## Required invariants

```text
reject self-causation
reject self-evidence
reject missing cause
reject missing evidence
reject duplicate terminal outcome
preserve v1 Journal during migration
detect mutation of every hashed field
serialize concurrent writers
recover after process restart
keep Mind alive after plasmashell restart
```

## Definition of Done

A change is complete only when tests exist, configure and compilation pass, QtTest passes, packages build, required CI gates are green, and documentation matches behavior.
