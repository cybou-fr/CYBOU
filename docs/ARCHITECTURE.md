<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cybou Architecture

`MIND_MODEL.md` describes what the cognitive architecture means. This document describes the
current process topology, ownership boundaries, failure domains, and ordering.

## Current M5 process topology

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

Full capability-deficit policy is M6, while the current M5 topology provides the isolation and
continuity required for it.

Process isolation means that a capability can fail independently. It does not yet mean the rest of
Mind can always continue usefully; M6 defines that policy.

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

Current restart/reboot semantics are documented in `CURRENT_STATE.md`; in-place upgrade
reconciliation remains a separate hardening track and capability-specific recovery belongs to M6.

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

M8 adds optional language capability without moving identity or memory authority into a model.

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
implemented M5 evaluation boundary. Resource/capability-aware operation belongs to M6. ADR-0024
and ADR-0026 are normative.

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

M9 is intentionally outside the current M5 organ topology.

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

M5 strengthens continuity/recovery and introduces lifecycle/consolidation.

M6 turns process health and internal pressure into explicit capability deficits and homeostasis.

M7 adds grounded perception, epistemic/retention governance, then tests them across nodes.

M8 adds replaceable language faculties.

M9 adds policy-controlled external agency.

See `MIND_MODEL.md` for the conceptual model and `ROADMAP.md` for milestone semantics.
