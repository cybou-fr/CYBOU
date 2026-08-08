<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Roadmap

`MIND_MODEL.md` describes the long-form cognitive model. This roadmap states what new engineering
capability each milestone adds.

## M0 — Green Build

**Ongoing gate discipline.**

Meaning: architecture claims do not count unless the repository can build and test them
reproducibly.

## M1 — One Presence / live accepted contribution

**Complete.**

Meaning: an accepted durable contribution can become visible through one Presence path without
creating a second hidden cognitive owner in the UI.

## M2 — Journal v2

**Complete.**

Meaning: durable biography gains stricter causal/evidence/privacy invariants, canonical hashing,
migration, and tamper-detection semantics.

## M3 — eventd

**Complete after the Package 06 prerequisite compile repair.**

Meaning: Journal has one canonical process owner and accepted-event semantics cross one explicit
Event1 boundary.

Exclusive canonical Journal ownership and Event1 semantics are unchanged.

## M4 — Process-Isolated Organs

**Implementation present; gates are acceptance authority.**

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

System meaning: cognitive responsibilities now have explicit process/state ownership and independent
failure domains. This creates the substrate required for continuity and degraded cognition.

## M5 — Continuity

**Next.**

Engineering scope:

- stronger restart/reboot continuity;
- recovery and reconstruction guarantees;
- architecture-transition records;
- explicit reconciliation rules;
- verification that durable identity/commitments survive supported transitions.

System meaning:

> Cybou can survive supported component and system transitions without falsely claiming a new
> identity or silently inventing seamless continuity.

A stable UUID alone is not enough. Continuity must be supported by verified state/history and
explicit transitions.

## M6 — Degraded Modes

Engineering scope:

- richer health states;
- explicit capability deficits;
- partial availability policy;
- recovery/reconciliation behavior;
- degraded continuity representation where needed.

System meaning:

> Loss of one organ becomes loss of a capability, not automatic death of the whole Mind.

Example target:

```text
predictord unavailable
→ prediction capability unavailable

identity + biography + intentions + workspace remain usable
→ Mind = degraded, not absent
```

The exact capability matrix must be explicit and testable.

## M7 — Distributed Node Prototype

Engineering scope:

- inter-node transport;
- selective replication;
- privacy-aware state movement;
- partition/conflict behavior;
- node-local versus identity-level ownership.

System meaning:

> One verified identity may be represented across multiple nodes without treating blind file sync as
> continuity.

Replication must preserve causal, privacy, and ownership rules.

## M8 — Optional Language Faculty

Engineering scope:

- replaceable local/remote language faculty;
- typed context selection;
- request interpretation;
- hypothesis/plan proposal;
- explanation formulation;
- model absence/replacement behavior.

System meaning:

> Language becomes an optional capability attached to Mind, not the owner of identity, biography,
> intentions, authorization, or execution.

A model may be replaced or disabled without creating a new Cybou identity.

See ADR-0021.

## M9 — Authorized Action Boundary

Engineering scope:

```text
typed proposal
→ criticism
→ decision
→ capability authorization
→ typed executor
→ Nix build/test where applicable
→ confirmation when required
→ execution/switch
→ observation
→ outcome
→ rollback where possible
```

System meaning:

> Cybou may affect its operating environment only through an explicit policy-controlled boundary,
> and the observed result returns to cognition.

The target is not `LLM → privileged shell`.

Every attempted external action must become observable enough to determine what was attempted, what
actually happened, and whether the intended outcome was reached.

See ADR-0022.

## Capability progression

```text
M1  accepted contribution becomes live
 │
M2  biography becomes causally stricter
 │
M3  biography gets one canonical writer
 │
M4  cognitive responsibilities become isolated owners
 │
M5  identity/commitments gain stronger continuity
 │
M6  partial failure becomes explicit degraded cognition
 │
M7  continuity/privacy are tested across nodes
 │
M8  replaceable language attaches as a faculty
 │
M9  external agency crosses authorization + observation
```

The milestone order is intentional: agency is added after memory, ownership, continuity, degraded
behavior, and model replaceability have explicit boundaries.
