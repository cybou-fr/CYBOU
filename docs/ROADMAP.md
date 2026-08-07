<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Roadmap

Status snapshot: 2026-08-07.

Milestone numbers describe architectural dependencies. Implementation can land out of numeric
order.

## M0 — Green Build

**Status: In progress / fast gate green.**

Core C++/QML/package gates are green. The tag-only full `nix flake check` / VM gate remains the
heavy validation path.

## M1 — One Presence, One Journal

**Status: Complete for the current in-process runtime.**

Implemented:

- multiple Presence surface objects for the same state root share one in-process Mind runtime;
- one shared Journal/Identity/organ/Workspace object graph exists for those surfaces;
- opening another Presence surface does not increment the Identity session;
- Journal emits an accepted-contribution event only after successful COMMIT;
- Workspace consumes every accepted contribution live, including direct organ `Journal::append()`;
- Workspace no longer requires a full rehydrate after each normal action;
- the default persistent Mind root is stable at `$XDG_STATE_HOME/cybou` on Unix;
- legacy host-derived Presence state is migrated fail-closed without overwriting canonical state;
- focused M1 runtime tests cover shared Presence, live Workspace, accepted-after-commit, and state
  migration.

Scope note: this is one backend inside the current `plasmashell` process. Session-wide
cross-process ownership belongs to `presenced`/M4.

## M2 — Journal v2

**Status: Complete.**

Schema/hash versions, canonical encoding, v1→v2 migration, reference/privacy validation,
serialized writes, normalized evidence, and terminal-Outcome uniqueness are implemented.

## M3 — eventd

**Status: Next.**

Make `cybou-eventd` the exclusive Journal owner.

The M1 semantic boundary is intentionally reusable:

```text
proposal
→ durable validation/COMMIT
→ accepted contribution
→ Workspace / Presence projections
```

M3 changes ownership and transport; it should not change that ordering.

## M4 — Process-Isolated Organs

**Status: Planned.**

`identityd`, `intentiond`, `predictord`, `selfd`, `workspaced`, and `presenced`.

## M5 — Continuity

**Status: Planned.**

Reboot-surviving intention and verified identity/architecture transitions.

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
