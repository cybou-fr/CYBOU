<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Roadmap

`MIND_MODEL.md` describes the long-form cognitive/control model. This roadmap states what new
engineering capability each milestone adds.

For ordered implementation packages and current exit gates, see
[Next Engineering Steps](NEXT_STEPS.md). `CURRENT_STATE.md` is authoritative for implemented
behavior.

## M0 — Green Build

**Ongoing gate discipline.**

Meaning: architecture claims do not count unless the repository can build and test them
reproducibly.

## M1 — One Presence / live accepted contribution

**Complete.**

Meaning: an accepted durable contribution can become visible through one Presence path.

## M2 — Journal v2

**Complete.**

Meaning: durable biography gains stricter causal/evidence/privacy/hash semantics.

## M3 — eventd

**Complete.**

Meaning: Journal has one canonical process owner.

## M4 — Process-Isolated Organs

**Implementation present; gates are acceptance authority.**

Meaning: cognitive responsibilities have explicit ownership and failure domains.

## M5 — Continuity and Cognitive Lifecycle

**Evaluation milestone complete.**

Meaning: continuity gains explicit lifecycle, consolidation, interruption, and recovery.

## M6 — Degraded Modes

**Complete through P6.6; P6.7 resilience hardening is also complete.**

Meaning: loss of one organ becomes loss of a capability, not automatic death of Mind.

## M7 — Grounded and Distributed Mind Prototype

**Next engineering milestone.**

Engineering scope includes provenance-bearing perception, epistemics, contradiction/reconciliation,
retention/sensitivity/erasure, associative context/governed delivery, value constraints, and a later
distributed prototype.

System meaning:

> Cybou grounds world state in evidence and governs what may be retained/disclosed before broad
> agency exists.

## M8 — Language and Meaning

Engineering scope includes typed `MeaningInterpretation`, `CognitiveAct`, reference resolution,
append-only corrections, `ResponsePlan`, replaceable language implementations, and operation with no
generative model.

System meaning:

> Human language crosses an inspectable meaning boundary without a model owning cognition.

## M9 — Lifelong Learning

Engineering scope includes evidence-linked learning candidates, behavioral adaptation, skill
induction, learned-artifact lineage, evaluation/promotion/rollback, erasure propagation, and optional
neural adaptation.

System meaning:

> Cybou learns without turning facts, skills, parameters, or preferences into authority.

## M10 — Governed Action and Remediation Boundary

```text
proposal
→ criticism/checks
→ decision
→ capability authorization
→ typed executor/broker
→ confirmation when required
→ execution
→ observation
→ outcome
→ rollback/containment where possible
```

System meaning:

> Cybou can affect the environment without turning uncertain cognition into privileged authority.

See ADR-0022.

## M11 — Agent, Worker, Model, and Tool Runtime

Engineering scope:

- Faculty / Worker / Agent identities;
- task-scoped worker lifecycle and grants;
- context/network/resource/retention/delegation bounds;
- local/remote model brokerage;
- provider/model attribution and capability degradation;
- governed tools and MCP server/method/resource mediation;
- credential handles rather than raw credential distribution;
- actor/tool/network attribution;
- prompt-injection-resistant capability boundaries.

System meaning:

> AI execution becomes disposable and governed while Mind remains persistent.

See ADR-0034 and ADR-0035.

## M12 — Autonomous Security and Operations Control Plane

Engineering scope:

- desired state versus observed state;
- firewall/network-exposure governance;
- endpoint/process/persistence monitoring;
- service/package/configuration integrity;
- SSH/access and credential governance;
- agent/worker/MCP behavior monitoring;
- risk-tiered autonomous response;
- standing authorization;
- reversible containment/self-healing;
- post-action verification;
- baseline enforcement without model availability.

System meaning:

> Cybou can protect and maintain the managed machine when the person is absent without granting an
> AI unrestricted root authority.

See ADR-0036.

## M13 — Distributed Perimeter and Multi-node Governance

Engineering scope:

- extend M7 distributed continuity into operational governance;
- cross-node capabilities and policy;
- perimeter/network trust;
- remote-node health/remediation;
- cross-node worker grants;
- partition/conflict behavior;
- separation of replicated cognition from delegated security authority.

System meaning:

> One Cybou identity can govern a multi-device environment without treating every node/agent/path as
> equally trusted.

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
M6  partial failure and pressure become explicit degraded cognition
 │
M7  world state, evidence, retention, sensitivity, context, and distribution become governed
 │
M8  human language crosses an explicit meaning boundary
 │
M9  experience becomes governed learned behaviour and skills
 │
M10 external mutation crosses authorization + observation
 │
M11 agents, workers, models, and tools become governed runtime subjects
 │
M12 security and operations become continuously self-maintaining under standing policy
 │
M13 governance extends across nodes and perimeter
```

The milestone order is intentional: agency is added after memory, ownership, continuity,
consolidation, degraded behavior, provenance, retention, cognitive governance, distribution, and
model replaceability have explicit boundaries. Agent autonomy follows the action boundary;
unattended security follows governed actors/tools/models.

The sequence above is the canonical capability progression.

## Cross-cutting Presence modernization

The proposed [web-first Presence architecture](WEB_UI_ARCHITECTURE.md) is a delivery track rather
than a new cognitive milestone. It may advance beside M7/M8 provided it does not claim M10 action
authority, M11 agent runtime, or M12 control-plane capability.

Its order is:

```text
web contracts and fixtures
→ read-only gateway beside Plasma
→ opt-in Chromium/Wayland desktop
→ parity for current bounded commands
→ authenticated remote read-only access
→ governed remote actions only after M10
→ Plasma retirement after replacement gates
```

The same frontend may be delivered locally and remotely; session trust, context disclosure, and
capability grants remain different. See [ADR-0037](adr/ADR-0037-web-first-presence-and-desktop.md).

## Cross-cutting Rust migration

All new product implementation targets Rust; Living Canvas targets Rust/WASM. Existing C++/Qt
owners are replaced incrementally through shared fixtures and reversible, one-owner cutovers rather
than a flag-day rewrite. The sequence is foundation and contracts, web UI/gateway, shared runtime,
leaf/derived organs, lifecycle and Journal owners, then removal of QML/Qt/CMake after parity and
continuity evidence. See [ADR-0038](adr/ADR-0038-rust-first-codebase.md).

Like Presence modernization, this does not reorder cognitive milestones. A milestone may advance
only when its implementation does not create new long-lived C++ debt in a component whose Rust
migration has begun.

A shorter product reading:

```text
M7  understand the world
M8  understand the person
M9  learn from experience
M10 act under authority
M11 govern AI workers, tools, and models
M12 protect and maintain the system unattended
M13 govern the perimeter across nodes
```
