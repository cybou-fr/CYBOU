<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Roadmap

Status snapshot: 2026-08-07.

Milestone numbers describe the intended architectural sequence. Implementation work can land out
of order; Journal v2 (M2) is already complete while M1 still has open runtime obligations.

## M0 — Green Build

**Status: In progress / fast gate green.**

Goal: CMake, C++, QML, QtTest, REUSE, package validation, and Nix checks pass.

Current:

- `cybou-mind` builds and runs all nine CTest suites;
- `cybou-presence-applet` builds and passes package/QML validation;
- formatting, REUSE, and built-theme package metadata pass in fast CI;
- the ordinary push workflow is green on the current snapshot.

Still required before calling the complete validation matrix permanently closed:

- keep the tag-only full `nix flake check` / VM gate healthy;
- expand fast package coverage when runtime-relevant packages become mandatory gates.

## M1 — One Presence, One Journal

**Status: In progress.**

Goal: one Presentation backend per user session; Workspace reflects new accepted contributions.

Remaining:

- enforce or extract one Presence backend per session;
- introduce one accepted-contribution path instead of unrelated direct Journal writes;
- update Workspace live after every accepted contribution;
- move persistent Mind state to a stable owner-independent location;
- prove session/Plasma lifecycle behavior with runtime or VM tests.

## M2 — Journal v2

**Status: Complete.**

Implemented:

- database, envelope, and hash versions;
- canonical full-envelope encoding;
- v1 → v2 migration with retained backup;
- reference existence validation;
- privacy inheritance validation;
- normalized evidence storage;
- serialized writers with `BEGIN IMMEDIATE`;
- terminal-Outcome uniqueness backed by SQLite;
- version-aware history verification preserving v1 hashes.

## M3 — eventd

**Status: Planned.**

Exclusive Journal owner with IPC and accepted-contribution signals.

The M1 accepted-contribution abstraction should be designed so it can move behind `eventd`
without changing organ semantics.

## M4 — Process-Isolated Organs

**Status: Planned.**

`identityd`, `intentiond`, `predictord`, `selfd`, `workspaced`, and `presenced`.

## M5 — Continuity

**Status: Planned.**

Reboot-surviving intention, stable state locations, verified identity migration, and architecture
transition records.

## M6 — Degraded Modes

**Status: Planned.**

Health, capability deficits, recovery, and reconciliation.

## M7 — Distributed Node Prototype

**Status: Planned.**

Selective replication and partition handling.

## M8 — Optional Language Faculty

**Status: Planned.**

Language is a faculty, not identity or executor.

## M9 — Authorized Action Boundary

**Status: Planned.**

Typed proposal, criticism, authorization, Nix build/test, confirmation, execution, outcome, and
rollback.
