<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Roadmap

`MIND_MODEL.md` describes the long-form cognitive model. This roadmap states what new engineering
capability each milestone adds.

For the ordered implementation packages, test matrices, and exit gates beginning from the completed
M6/P6.7 baseline, see [Next Engineering Steps](NEXT_STEPS.md).

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

## M5 — Continuity and Cognitive Lifecycle

**Evaluation milestone complete.**

Engineering scope:

- stronger restart/reboot continuity;
- recovery and reconstruction guarantees;
- architecture-transition records;
- explicit reconciliation rules;
- verification that durable identity/commitments survive supported transitions.
- explicit `Awake`, `Idle`, `Consolidating`, `Maintenance`, `Recovering`, and `Suspended` modes;
- bounded, interruptible consolidation runs with durable terminal records;
- checkpoints/high-water marks and idempotent recovery after partial consolidation;
- owner-specific calibration, intention review, integrity, and attention-maintenance requests;
- temporal freshness/expiry semantics for maintained projections.

System meaning:

> Cybou can survive supported component and system transitions without falsely claiming a new
> identity or silently inventing seamless continuity.

It can also maintain accumulated experience without creating a central `sleepd` that owns every
organ. A lifecycle coordinator requests typed work; existing organs retain ownership and all
durable results cross Event1.

A stable UUID alone is not enough. Continuity must be supported by verified state/history and
explicit transitions.

The implemented evaluation boundary and unsupported transition paths are recorded in
[M5 Evaluation Evidence](M5_EVALUATION.md). In-place upgrade reconciliation remains an explicit
hardening track rather than an unverified M5 claim.

## M6 — Degraded Modes

**Complete through P6.6; P6.7 resilience hardening is also complete.**

Engineering scope:

- richer health states;
- explicit capability deficits;
- partial availability policy;
- recovery/reconciliation behavior;
- degraded continuity representation where needed.
- homeostatic signals for storage, backlog, latency, freshness, and calibration drift;
- capability-aware consolidation scheduling and interruption;
- metacognitive projection of unknown, stale, assumed, unsupported, and degraded state.

System meaning:

> Loss of one organ becomes loss of a capability, not automatic death of the whole Mind.

Implemented representative behavior:

```text
predictord unavailable
→ prediction capability unavailable

identity + biography + intentions + workspace remain usable
→ Mind = degraded, not absent
```

The capability matrix, command gates, recovery progression, automatic lifecycle scheduling, and
representative optional/required-owner failures are explicit and tested. See
[Current State](CURRENT_STATE.md) for the exact accepted boundary.

## M7 — Grounded and Distributed Mind Prototype

**Next engineering milestone.** Begin with one local provenance-bearing vertical slice; distributed
transport remains deferred until local epistemic and retention semantics are testable.

Engineering scope:

- typed perception adapters with source, freshness, privacy, and provenance;
- an epistemic projection distinguishing observed, reported, inferred, assumed, disputed,
  superseded, and unknown state;
- explicit contradiction detection and reconciliation records;
- retention/forgetting policy, including propagation to derived and replicated material;
- bounded executive attention for interruption, deferral, and competing intentions;
- typed value constraints covering user authority, safety, privacy, reversibility, cost, urgency,
  evidence quality, and resource budget;
- inter-node transport;
- selective replication;
- privacy-aware state movement;
- partition/conflict behavior;
- node-local versus identity-level ownership.

System meaning:

> Cybou can ground knowledge in provenance, represent disagreement and uncertainty, govern what is
> retained, and then carry one verified identity across nodes without treating blind file sync as
> continuity.

Replication must preserve causal, privacy, and ownership rules.

M7 is intentionally a minimal vertical slice, not a claim to solve general knowledge
representation. Grounding/retention semantics precede replication so the system knows what a state
means and whether it is allowed to move before transporting it.

## M8 — Language and Meaning

Engineering scope:

- a typed `MeaningInterpretation` and `CognitiveAct` boundary;
- explicit reference resolution, with ambiguity that stays ambiguous;
- correction semantics that append rather than rewrite;
- context-delivery integration;
- a semantic `ResponsePlan` before any surface realization;
- replaceable language implementations;
- operation with no generative model at all.

System meaning:

> Cybou understands and expresses meaning through inspectable typed state, without a language model
> becoming the owner of cognition.

Selected context carries provenance, epistemic status, freshness, privacy, and capability deficits;
fluency must not erase those qualifications. A language implementation may be replaced or removed
without creating a new Cybou identity.

See ADR-0021 and ADR-0031.

## M9 — Lifelong Learning

Engineering scope:

- learning candidates that cite accepted evidence;
- fast reconstructible linguistic and behavioural adaptation;
- procedural skill induction with replay and evaluation;
- consolidation integration under ADR-0024;
- learned-artifact lineage and immutable generations;
- promotion, rejection and rollback;
- retention and erasure propagation into learned state;
- optional local neural adaptation.

System meaning:

> Cybou improves from experience while keeping facts, learning, parameters, skills and authority as
> separate governed state.

A completed training run is not a promotion, and an erased source does not become forgotten merely
because the artifact it influenced is hard to read.

See ADR-0032 and ADR-0033.

## M10 — Authorized Action Boundary

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

The target is not `LLM → privileged shell`. A skill learned in M9 may instantiate a proposal here;
it does not grant its own execution authority.

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
M5  continuity gains lifecycle, consolidation, and recovery
 │
M6  partial failure and internal pressure become explicit degraded cognition
 │
M7  perception, knowledge, retention, values, association, and distribution become governed
 │
M8  human language crosses an explicit meaning boundary
 │
M9  experience can become governed learned behaviour and skills
 │
M10 external agency crosses authorization + observation
```

The milestone order is intentional: agency is added after memory, ownership, continuity,
consolidation, degraded behavior, provenance, retention, cognitive governance, distribution, and
model replaceability have explicit boundaries.

ADR-0024 defines lifecycle/consolidation. ADR-0025 defines grounding, epistemics, retention,
homeostasis, executive attention, and value constraints.

A shorter product reading:

```text
M7  understand the world
M8  understand the person
M9  learn from experience
M10 act under authority
```
