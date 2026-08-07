<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Current State

Status date: 2026-08-07.

## Implemented milestones

- M1 — shared Presence runtime, stable state root, live accepted-contribution Workspace;
- M2 — Journal v2;
- M3 — `cybou-eventd` single production Journal writer and Event1 IPC.

## Current process topology

```text
user D-Bus
│
├── cybou-eventd
│     └── Journal v2
│
└── plasmashell
      ├── Presence surface(s)
      └── shared PresenceRuntime
            ├── EventClient ────────────────┐
            ├── Identity                   │
            ├── Intentions                 │ EventStore
            ├── Predictor                  │
            ├── SelfModel                  │
            └── Workspace ◄────────────────┘
```

The daemon-like source directory names for Identity/Intentions/Predictor/Self/Workspace/Presence
still do not mean those components are separate processes. `cybou-eventd` is the first real
process-isolated Mind service.

## Journal ownership

Normal default/QML Presence no longer opens `journal.db`.

Organs depend on `EventStore`. The default runtime supplies `EventClient`, which talks to:

```text
service   org.cybou.Mind.Event1
object    /org/cybou/Mind/Event1
interface org.cybou.Mind.Event1
```

`cybou-eventd` owns the actual `Journal` object and SQLite connection.

The explicit `Presence(dataDir)` constructor remains a local test/tool seam and may instantiate a
Journal for isolated temporary directories. It is not the production QML path.

## Accepted ordering

```text
proposal
→ EventClient
→ Event1 Submit
→ eventd
→ Journal validation
→ BEGIN IMMEDIATE
→ COMMIT
→ Journal accepted
→ Event1 Accepted
→ EventClient accepted
→ Workspace accept
→ Presence changed
```

A rejected or rolled-back proposal never produces `Accepted`.

## State migration

The existing M1 legacy-state migration remains in Presence bootstrap and runs before the first
Event1 call. This is intentional: the first Event1 call may D-Bus-activate eventd, so legacy files
must be moved to `$XDG_STATE_HOME/cybou` before eventd opens the canonical Journal.

## D-Bus activation

The Nix package installs:

```text
share/dbus-1/services/org.cybou.Mind.Event1.service
```

The first Event1 method call can therefore start `cybou-eventd` without coupling it to Plasma
startup ordering.

## Automated tests

The Mind package now runs eleven CTest suites. The new `eventd-integration` suite runs under its own
`dbus-run-session` and checks:

- Event1 startup and schema version;
- post-COMMIT accepted delivery;
- rejection produces no accepted signal;
- query round trips;
- default Presence uses eventd and preserves one shared session;
- a second eventd cannot own the same bus service;
- after eventd dies, default Presence/EventClient do not silently fall back to local SQLite.

## Current limitations

- Identity, Intentions, Predictor, SelfModel, Workspace, and Presence still run in `plasmashell`;
- Identity JSON state is still written by the in-process Identity component;
- Event1 reads are currently synchronous and can be chatty for large projections;
- explicit process health/degraded-mode projection is not implemented;
- same-user D-Bus authorization/capability policy is not yet a security boundary;
- remaining organ process isolation is M4.

## Next milestone

M4 — process-isolated organs and `presenced`.
