<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Roadmap

## M0 — Green Build

Ongoing gate discipline.

## M1 — One Presence / live accepted contribution

**Complete.**

## M2 — Journal v2

**Complete.**

## M3 — eventd

**Complete after the Package 06 prerequisite compile repair.**

Exclusive canonical Journal ownership and Event1 semantics are unchanged.

## M4 — Process-Isolated Organs

**Complete after Package 06 gates pass.**

Implemented:

- `cybou-identityd`;
- `cybou-intentiond`;
- `cybou-predictord`;
- `cybou-selfd`;
- `cybou-workspaced`;
- `cybou-presenced`;
- one versioned D-Bus endpoint per organ;
- `systemd --user` `Type=dbus` units;
- D-Bus activation through those units;
- QML Presence reduced to a remote proxy;
- identity restart guard for one logical login;
- process integration tests;
- VM smoke assertions for the service graph.

## M5 — Continuity

**Next.**

Stronger restart/reboot continuity, recovery, architecture-transition records, and reconstruction
guarantees.

## M6 — Degraded Modes

Health state, capability deficits, recovery, reconciliation.

## M7 — Distributed Node Prototype

Selective replication and partition handling.

## M8 — Optional Language Faculty

Language is a faculty, not identity or executor.

## M9 — Authorized Action Boundary

Typed proposal, criticism, authorization, build/test, confirmation, execution, outcome, rollback.
