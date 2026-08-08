<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Cybou Mind Model

## Purpose

Cybou is built around a persistent cognitive runtime rather than around a language model.

Mind is the subsystem that preserves and transforms typed cognitive state across UI restarts,
process restarts, and — as later milestones are implemented — stronger continuity boundaries such
as reboots, upgrades, and multiple nodes.

Mind is **not** a chatbot, language model, vector database, shell agent, or one monolithic service.

The architectural target is that identity, durable biography, commitments, predictions,
self-model, and bounded attention remain meaningful independently of any particular language model
or user interface.

Language, perception, planning, and execution are faculties or capabilities attached to Mind
through explicit protocols. They do not become the owner of identity or biography.

## Terminology and scope

Terms such as **Mind**, **identity**, **self**, **attention**, and **cognitive** describe software
architecture and state ownership in this project. They are not claims that Cybou is conscious,
sentient, or biologically equivalent to a human mind.

This document mixes two scopes, which are always labeled:

- **Current substrate** — behavior implemented by the M1–M4 architecture and its current APIs.
- **Future target** — capabilities planned by M5–M9 and proposed ADRs.

`CURRENT_STATE.md` remains authoritative for what is implemented today.

## Body, Mind, and Presence

ADR-0001 defines three top-level domains:

```text
Body
  operating environment, hardware, NixOS, Plasma, processes, system state

Mind
  persistent cognitive substrate and its typed state transitions

Presence
  presentation boundary through which Mind is exposed to a user or another surface
```

No organ, model, database, process, or UI component is Cybou by itself.

In particular:

```text
Plasma ≠ Mind
Journal ≠ Mind
presenced ≠ Mind
language model ≠ Mind
executor ≠ Mind
```

## Cognitive topology

The current process topology is described in `ARCHITECTURE.md`. The same system can be viewed by
cognitive responsibility:

```text
                         environment / user
                                │
                           Observation
                                │
                                ▼
                         ┌──────────────┐
                         │    eventd    │
                         │  Journal v2  │
                         └──────┬───────┘
                                │
                      durable causal history
                                │
            ┌──────────┬────────┼────────┬───────────┐
            ▼          ▼        ▼        ▼           ▼
        identityd  intentiond predictord selfd   workspaced
            │          │        │        │           │
            └──────────┴────────┴────────┴─────┬─────┘
                                               ▼
                                          presenced
                                               │
                                               ▼
                                           Presence
```

The diagram is intentionally not a neural analogy. Each organ has a narrow software ownership
boundary.

## Organ semantics

| Organ | Engineering question | Owned responsibility |
|---|---|---|
| `cybou-eventd` | What happened? | canonical durable event history and Event1 acceptance |
| `cybou-identityd` | Which identity continues through this history? | identity state and logical session continuity |
| `cybou-intentiond` | What remains unresolved or committed? | intentions and their terminal state |
| `cybou-predictord` | What is expected, and how well have expectations performed? | prediction and calibration state |
| `cybou-selfd` | What structured assessment does the system have of itself? | self projection and explicit assessment |
| `cybou-workspaced` | What is important now? | bounded transient context, coalitions, salience, focus |
| `cybou-presenced` | What projection should be exposed outward? | presentation aggregation across organ APIs |

These questions are semantic descriptions. The exact RPC contracts remain defined by the organ and
IPC documentation.

## Core invariants

### 1. Durable before visible

A contribution that is supposed to become part of biography is committed before it is admitted to
Workspace and before Presence presents the resulting state.

Current ordering:

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

This prevents the UI or transient attention state from presenting a durable cognitive event that
never became part of the accepted history.

### 2. Causality over loose memory

Journal v2 is not an unstructured message history.

For new v2 contributions:

- only `Observation` may be a root contribution;
- non-root contributions require direct cause or evidence;
- references must already exist;
- malformed self/duplicate/null reference patterns are rejected;
- derived privacy may not be weaker than referenced contributions.

This means a derived claim, intention, prediction, or outcome can be tied to prior accepted state
instead of being an unexplained blob in a conversation transcript.

### 3. Identity is not a process

A daemon restart is not a birth event.

`identityd` owns persistent identity state and uses a volatile login-session marker so a restart
inside one login can resume that logical session rather than incrementing it.

M5/M6 strengthen continuity beyond the current restart guard.

### 4. Attention is not biography

Journal is durable history.

Workspace is bounded, transient, reconstructible active context.

`workspaced` may select a current focus or coalition from accepted history, but it does not become a
second biography owner.

### 5. Models are faculties

A future language model may interpret language, retrieve typed context, propose hypotheses or plans,
and formulate explanations.

It does not become:

- identity authority;
- biography owner;
- canonical Journal writer;
- intention owner;
- authorization authority;
- privileged executor.

Replacing or disabling a language model must not, by itself, create a new Cybou identity or erase
existing biography and commitments.

### 6. External actions return as observations

The future action architecture is a closed cognitive loop.

An external mutation is not complete when a command is issued. The observed result returns through
the typed cognitive protocol and becomes evidence for outcome, prediction calibration, future
attention, and self assessment.

Target loop:

```text
Perception / user request
        │
        ▼
    Observation
        │
        ▼
       Mind
        │
        ▼
    Intention
        │
        ▼
Planning faculty
        │
        ▼
Authorized Action Boundary
        │
        ▼
Typed capability / executor
        │
        ▼
 Body / external environment
        │
        ▼
Observed consequence
        │
        └──────────────► Observation / Outcome
```

M9 owns the authorization/execution boundary. It is not implemented by the current M4 substrate.

## The current substrate

The current M4 design establishes the process and ownership boundaries required for the larger
model:

```text
Plasma/QML
    │
    ▼
Presence QML proxy
    │ Presence1
    ▼
cybou-presenced
    ├── Identity1   ─► cybou-identityd
    ├── Intention1  ─► cybou-intentiond
    ├── Predictor1  ─► cybou-predictord
    ├── Self1       ─► cybou-selfd
    ├── Workspace1  ─► cybou-workspaced
    └── Event1      ─► cybou-eventd ─► Journal v2
```

The Plasma-hosted Presence object is a remote proxy/cache. Destroying and recreating the UI does not
create another domain-organ graph.

This substrate currently provides the foundations for:

- durable causal event history;
- one explicit identity owner;
- intention state;
- prediction/calibration state;
- self projection;
- bounded Workspace attention;
- one presentation aggregator;
- process-level failure isolation.

It does **not** yet provide the full future agent:

- no M6 capability-deficit model;
- no inter-node transport/replication;
- no optional language faculty implementation;
- no authorized action executor boundary.

## Prediction and calibration

Prediction is deliberately separate from language generation.

The architectural purpose of `predictord` is to let expectations become state that can later be
compared with observations.

A future system can therefore distinguish:

```text
what was expected
what actually happened
how confident the expectation was
how the prediction family performs over time
```

This supports calibration as a system property instead of treating model confidence text as
ground truth.

A language faculty may formulate a prediction in natural language, but the durable/typed prediction
state belongs to the prediction contract, not to the model's hidden context.

## Self model

`selfd` owns structured self projection and explicit assessment.

The intent is not to make an LLM invent autobiographical prose. Facts exposed through self
narration should originate from typed Mind state — for example identity/session state, commitments,
prediction history, integrity, or health.

A future language faculty may turn that structured state into a more natural explanation, but the
model is not the authority for those facts.

## Workspace and global attention

Workspace is the bounded layer between large durable history and the small set of things relevant
now.

Its concepts include:

```text
coalition
salience
focus
organs involved
current moment
attention
```

Every accepted durable contribution is eligible for Workspace admission.

Presentation observes Workspace changes after accepted admission rather than treating raw event
arrival as equivalent to current attention.

## Presence

Presence is an outward projection boundary.

`cybou-presenced` aggregates organ projections. The Plasma QML Presence object caches that remote
snapshot.

The current presentation may expose:

```text
awake
narration
obligations
attention
contributions
stats
identityState
calibrations
coalitions
moment
organHealth
```

Presentation must not become a second owner of domain state.

The same rule applies to future surfaces: CLI, mobile, voice, remote console, or language UI should
observe/command Mind through explicit boundaries rather than reimplementing cognition inside the
surface.

## Future degraded cognition — M6

Process isolation by itself is not degraded cognition.

The current health contract can identify whether an organ is reachable, but a missing organ can
still make the aggregated readiness state fail.

M6 should turn component health into explicit capability deficits.

Target example:

```text
eventd       healthy
identityd    healthy
intentiond   healthy
selfd        healthy
workspaced   healthy
predictord   unavailable

Mind state:
DEGRADED

missing capability:
prediction
```

Loss of prediction should not automatically mean loss of identity, durable memory, intentions, or
all presentation.

The exact policy belongs to the M6 design and ADR-0019.

## Future distributed continuity — M7

M7 introduces a distributed-node prototype.

The target is not blind synchronization of every local file. Replication must respect:

- ownership;
- privacy classification;
- causal history;
- conflict/partition behavior;
- node-local versus identity-level state.

A future one-identity/multiple-node topology may look like:

```text
                one verified identity
                       │
              ┌────────┼────────┐
              ▼        ▼        ▼
           laptop    phone    server
```

This is a target architecture, not a current implementation.

ADR-0018 and ADR-0016 define the relevant privacy and continuity direction.

## Future language faculty — M8

M8 connects language as a replaceable faculty.

Target data flow into Mind:

```text
user language
    │
    ▼
language faculty
    │
interpretation / hypothesis / proposal
    │
    ▼
typed protocol
    │
    ▼
Mind APIs
```

Target data flow out of Mind:

```text
typed Mind context
    │
    ▼
language faculty
    │
    ▼
human-readable explanation
```

The language model should receive selected typed context rather than unrestricted ownership of the
Journal database.

ADR-0021 contains the normative model boundary.

## Future authorized agency — M9

M9 is where Cybou may begin affecting the operating environment through a policy-controlled
boundary.

The target is explicitly **not**:

```text
LLM → privileged shell
```

The target is closer to:

```text
proposal
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

Every attempted external action should produce a typed record of:

- what was proposed;
- what was authorized;
- what was attempted;
- what actually happened;
- what evidence was observed;
- whether the intended outcome was reached.

ADR-0022 contains the normative action-boundary direction.

## Example future loop

The following is an **illustrative M8/M9 scenario**, not current M4 behavior.

User intent:

```text
Keep this server healthy. If disk usage becomes critical,
investigate it, but do not remove important user data.
```

Possible future flow:

```text
1. language faculty interprets the request
2. request becomes a typed observation
3. an intention is formed from that accepted cause
4. monitoring observes disk usage
5. predictor estimates whether the threshold is likely to be crossed
6. Workspace raises the issue as salience increases
7. a planning faculty proposes safe cleanup candidates
8. action policy permits low-risk cleanup and rejects protected data deletion
9. typed executor performs the authorized action
10. system state is observed again
11. observed result becomes outcome/evidence
12. predictor can be calibrated against the result
13. intention/self/attention projections update
14. Presence explains what happened and why
```

The important property is not the specific cleanup behavior. It is that perception, memory,
commitment, prediction, authorization, action, and observed consequence remain separate typed
responsibilities.

## Milestone meaning

The roadmap can be read as progressive cognitive capability:

| Milestone | System meaning |
|---|---|
| M1 | accepted durable contributions become visibly live |
| M2 | biography has stricter causal and hash semantics |
| M3 | Journal has one canonical writer |
| M4 | cognitive responsibilities have process-level ownership and failure isolation |
| M5 | continuity/recovery becomes stronger across restarts, reboots, and transitions |
| M6 | organ failure becomes an explicit capability deficit instead of whole-Mind collapse |
| M7 | continuity/privacy are tested across multiple nodes |
| M8 | replaceable language faculty attaches without becoming identity, memory, or executor |
| M9 | external action crosses an explicit authorization and observation boundary |

## Design test for future features

When a new AI, UI, planner, sensor, or executor is proposed, ask:

1. What domain does it belong to: Body, Mind, Presence, faculty, or capability?
2. What state does it own?
3. What state must it never own?
4. Does it write durable biography? If yes, through which accepted Event1 path?
5. What causes or evidence justify its derived contribution?
6. Can it be restarted or replaced without creating a new identity?
7. What capability is lost if it is unavailable?
8. If it acts externally, where is authorization decided?
9. How is the real result observed and returned to Mind?
10. Does the design preserve the difference between transient attention and durable biography?

If these questions have no clear answers, the feature is probably crossing an architectural
boundary.

## Related documents

- `CURRENT_STATE.md` — what is implemented now.
- `ARCHITECTURE.md` — process topology, ownership, failure domains, and ordering.
- `ROADMAP.md` — milestone progression.
- `mind/COGNITIVE_PROTOCOL.md` — typed contribution protocol.
- `mind/JOURNAL.md` — durable biography.
- `mind/ORGAN_CONTRACTS.md` — organ contracts.
- `mind/WORKSPACE.md` — bounded attention.
- `mind/PRESENCE_API.md` — presentation API.
- `adr/ADR-0001-system-architecture.md` — Body / Mind / Presence.
- `adr/ADR-0002-cognitive-causality-and-journal-invariants.md` — causal invariants.
- `adr/ADR-0014-workspace-admission-and-global-attention.md` — Workspace admission.
- `adr/ADR-0016-identity-continuity.md` — continuity direction.
- `adr/ADR-0018-privacy-classification-and-replication.md` — privacy/replication direction.
- `adr/ADR-0019-degraded-modes-and-capability-deficits.md` — degraded cognition direction.
- `adr/ADR-0021-language-models-are-optional-faculties.md` — language-model boundary.
- `adr/ADR-0022-authorized-action-boundary.md` — future action boundary.
