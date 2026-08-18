<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cybou Architecture

`MIND_MODEL.md` describes what the cognitive architecture means. This document describes the
current process topology, ownership boundaries, failure domains, and ordering.

## Current M6 process topology

```text
                          ┌───────────────────────┐
                          │      Plasma/QML       │
                          │ Presence proxy only   │
                          └───────────┬───────────┘
                                      │ Presence1
                                      ▼
                             ┌─────────────────┐
                             │ cybou-presenced │
                             └───────┬─────────┘
                                     │
          ┌──────────────┬───────────┼───────────┬──────────────┐
          ▼              ▼           ▼           ▼              ▼
   identityd       intentiond   predictord      selfd      workspaced
          │              │           │           │              │
          └──────────────┴───────────┴───────────┴──────┬───────┘
                                                        │ Event1
                                                        ▼
                                                 cybou-eventd
                                                        │
                                                        ▼
                                                   Journal v2
```

Each daemon is a separate executable, D-Bus name, and `systemd --user` service.

The presentation fan-out is complemented by two coordination owners:

```text
cybou-healthd     -- Health1 ----> typed capability and homeostasis projections
cybou-lifecycled  -- Lifecycle1 -> persistent bounded lifecycle runs
```

## Cognitive topology

The same processes have different semantic responsibilities:

```text
                       accepted durable history
                               │
       ┌──────────────┬────────┼──────────┬──────────────┐
       ▼              ▼        ▼          ▼              ▼
    identity       intention prediction   self        workspace
    continuity     state     calibration projection   attention
       │              │        │          │              │
       └──────────────┴────────┴──────────┴───────┬──────┘
                                                 ▼
                                             Presence
```

| Process | Semantic responsibility |
|---|---|
| `cybou-eventd` | canonical durable event history |
| `cybou-healthd` | capability dependency graph and current health snapshot |
| `cybou-lifecycled` | lifecycle/run orchestration and recovery metadata |
| `cybou-identityd` | identity and logical-session continuity |
| `cybou-intentiond` | unresolved commitments and terminal intention state |
| `cybou-predictord` | prediction and calibration state |
| `cybou-selfd` | structured self projection and assessment |
| `cybou-workspaced` | bounded transient attention and current moment |
| `cybou-presenced` | outward aggregation of organ projections |

This split is a state-ownership boundary, not an analogy to biological brain anatomy.

## Shared libraries

Shared libraries contain protocol/domain code and IPC utilities. They do not create a hidden
second Mind behind the QML surface.

The QML Presence library links only fabric/runtime client code; it no longer owns domain organ
objects.

Future faculties must follow the same rule: shared code may implement protocols and algorithms,
but it must not accidentally create a second owner for biography, identity, intentions, attention,
or authorization state.

## Failure domains

- stopping `predictord` does not terminate eventd, identityd, intentiond, workspaced, or presenced;
- restarting presenced does not start a new identity session;
- restarting identityd in the same login resumes the current identity through its runtime marker;
- restarting workspaced reconstructs bounded attention from Event1 history.

M6 is implemented through P6.6, with P6.7 latency hardening complete. Health1 owns capability
deficits and typed homeostasis; Presence1 projects them, gates commands by actual dependencies, and
uses one monotonic deadline for each compound read or mutation. Lifecycle1 owns evidence-bound
automatic scheduling, durable user-activity cooldown arbitration, recovery, and terminal state.

Lifecycle scheduling policy lives in lifecycled. Healthd supplies immutable capability and
homeostatic observations; it never transitions lifecycle mode or creates a run. The current
`EvaluateScheduling` path is deliberately dry-run: it computes worker eligibility and 32/8 backlog
hysteresis from Event1's durable `lifecycle.consolidation` consumer offset. Homeostasis v2
authorizes only the reviewed `event-backlog-v1` policy when that measurement is current. Event1 excludes consolidation-scoped outputs
from their own pressure, so a completed run cannot schedule itself again. Presence projects the
decision and its reason; evaluation itself remains read-only, while the lifecycled trigger owns
the separate bounded mutation.

Execution is a separate Lifecycle1 command. It binds the decision to both Health1 snapshot UUIDs,
revalidates them to close the evaluation/execution race, and derives the lifecycle run UUID from
that evidence. The same command is therefore idempotent across an unknown D-Bus result, process
restart, terminal completion, and replacement of the current run projection.

Lifecycled owns the trigger as well as the transaction: a 100 ms Health1-change debounce provides
reactivity and a 30-second timer provides verification. The cycle is a no-op for blocked/deferred
decisions. Existing scheduled recovery is resumed before any new evaluation, so a crash between
run creation and dispatch cannot fork lifecycle work.
Presence commands report user activity to Lifecycle1. A durable 60-second cooldown defers new
automatic work across restart, and activity interrupts an active automatic backlog run without
cancelling manual maintenance.
Production owner dispatch is sequential and asynchronous. Each idempotent owner operation uses a
deterministic key, and its callback must still match the active run before Lifecycle1 records the
result or advances to the next owner. This keeps the Lifecycle1 D-Bus event loop responsive during
slow owner work and fences late replies after interruption.

Presence1 endpoint readiness intentionally means that the presentation boundary answers. It does
not depend on Health1 because healthd probes presenced; coupling the two readiness checks would form
a cycle. Aggregate/per-capability state and raw deficits remain a separate projection sourced from
Health1; Presence additionally groups them into UI-ready details with causes, impact, verification
time, and recovery progress. Consequently loss of predictord disables Observe/Predict while identity,
commitments, biography, attention, lifecycle control, and the Presence endpoint remain usable.
Presence also publishes the command-to-capability mapping for QML, while every backend command
independently enforces the same gate. Lifecycle mode and aggregate health remain separate axes.

## Durable-to-visible ordering

Presence listens to Workspace1 `Changed`, not directly to raw Event1 `Accepted`:

```text
command
→ owning organ process
→ Event1
→ eventd
→ Journal COMMIT
→ Event1 Accepted
→ workspaced admission
→ Workspace1 Changed
→ presenced Changed
→ QML proxy refresh
```

This ordering implements the **durable before visible** invariant described in `MIND_MODEL.md`.

Workspace is allowed to forget bounded transient context. Journal is not.

## Identity versus process lifetime

The architecture deliberately separates:

```text
process lifetime
logical login session
persistent identity
durable biography
```

These are not interchangeable.

A service restart should not create a new identity. A future continuity failure should be
represented explicitly rather than silently inventing seamless continuity.

Current restart/reboot and capability-specific recovery semantics are documented in
`CURRENT_STATE.md`; stronger in-place upgrade reconciliation remains a separate hardening track.

## Presentation boundary

`cybou-presenced` is an aggregator, not a hidden monolith.

It may combine organ projections into a snapshot for Presence, but ownership remains with the
organ process that defines the state.

The Plasma Presence QObject is a remote proxy/cache only.

Therefore:

```text
Plasma restart         ≠ identity restart
QML object recreation  ≠ biography reset
Presence refresh       ≠ cognitive event
```

Future presentation surfaces must preserve the same boundary.

## Future faculty boundary

M8 adds an explicit meaning boundary without moving identity or memory authority into a model. No
generative model is required to cross it.

Target relationship:

```text
typed Mind context
      │
      ▼
language faculty
      │
interpretation / explanation / proposal
      │
      ▼
typed protocol
      │
      ▼
Mind
```

A model is replaceable. Mind state is not model hidden context.

The normative direction is ADR-0021.

## Lifecycle and consolidation boundary

Cybou requires a maintenance cycle analogous in purpose to sleep, but not a biological simulation
and not a central owner of cognition.

Implemented core relationship:

```text
lifecycle policy / trigger
        │
        ▼
coordinator selects accepted high-water mark
        │
        ├── typed maintenance requests to current owners
        │
        ▼
derived Event1 contributions
        │
        ▼
accepted terminal lifecycle record
```

The coordinator owns run orchestration only. It does not write organ storage or Journal. Accepted
history is never rewritten into a more convenient past; summaries, calibration, contradiction, and
expiry decisions remain derived records with evidence.

Lifecycle1, persistent run state, recovery, owner dispatch, durable owner results, accepted
terminal outcomes, Presence projection, and process/Plasma/reboot fault-injection gates form the
implemented M5 evaluation boundary. M6 adds capability-aware scheduling, degraded operation, and
recovery without moving state ownership into Lifecycle1. ADR-0024 and ADR-0026 are normative.

## Future grounding and cognitive-governance boundary

Journal answers what was accepted into causal history. A future epistemic projection separately
answers what is currently observed, reported, inferred, assumed, disputed, superseded, stale, or
unknown.

```text
Body / user / sensor
        │
        ▼
perception adapter + provenance
        │
        ▼
candidate Observation → Event1 → Journal
                              │
                              ▼
                  epistemic reconciliation
                              │
              governed retention and context
```

Perception is not truth. Confidence is not authorization. Retention is explicit policy, including
derived and replicated material. Executive attention and value constraints guide selection and
criticism but do not grant permission to execute.

ADR-0025 defines this direction. Concrete new processes are deferred until ownership, persistence,
failure, and privacy contracts are precise.

## Future action boundary

M10 is intentionally outside the current M1–M6 organ topology, as are the M11 agent, worker and
model runtime and the M12 security control plane.

No language model or UI component should become a privileged executor.

Target relationship:

```text
Mind / planning
      │
      ▼
proposal + criticism
      │
      ▼
authorization
      │
      ▼
typed capability / executor
      │
      ▼
Body / external environment
      │
      ▼
observation + outcome
      │
      └──────────────► Mind
```

The return path matters: action outcome must re-enter cognition as observed state rather than being
treated as a fire-and-forget shell side effect.

The normative direction is ADR-0022.

## Next

M1–M6 and P6.7 provide the current accepted substrate: typed memory, isolated owners, lifecycle,
continuity, degraded operation, recovery, evidence-bound scheduling, and bounded compound IPC.

M7 is next: add one local grounded-perception slice with provenance, epistemic/retention
governance, and fault evidence before attempting inter-node transport.

M8 adds a typed meaning boundary with replaceable language implementations.

M10 adds policy-controlled external agency; M11 governs the agents, workers, models and tools that
propose it, and M12 makes bounded operation continuous under standing policy.

See `MIND_MODEL.md` for the conceptual model and `ROADMAP.md` for milestone semantics.
