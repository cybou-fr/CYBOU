<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0021: Language and Models Are Optional Faculties

## Status

Proposed

This revision replaces the earlier Proposed wording of ADR-0021. No Accepted decision is being
silently rewritten: the prior ADR was deliberately Proposed because M8 had not been implemented.

## Context

Cybou is a persistent cognitive runtime, not a language model. Durable biography, identity,
intentions, epistemic state, associative context, attention, lifecycle, and future authorization
must remain meaningful when no generative model is installed or running.

The earlier wording already rejected a central LLM agent as the owner of memory, identity, planning,
and action. It still treated a language model as the default implementation of future language
support and allowed local or remote inference. That is too model-centric for the architecture we
actually want.

Natural language is one representation used at the boundary between a person and Mind. It is not
where Mind's meaning, memory, reasoning, or learning lives. A future implementation may use grammar,
semantic parsing, small specialized neural models, a generative model, or a combination of them.
None of those choices may redefine the cognitive substrate.

Remote inference also creates an unnecessary hidden dependency on a provider, network availability,
provider policy, and data egress. Core Cybou cognition must remain local and offline-capable.

## Decision

### Cognition does not depend on a generative model

The following distinctions are normative:

```text
language          ≠ cognition
utterance         ≠ meaning
model context     ≠ memory
generation        ≠ reasoning
model output      ≠ knowledge
model confidence  ≠ epistemic confidence
model proposal    ≠ authorization
```

A supported Cybou installation may have **no generative model at all**.

`NoModel` is therefore a valid configuration, not a degraded error state for core cognition.
Specific language features may be unavailable or less fluent, but identity, biography, epistemics,
context, intentions, lifecycle, learning, and authorization boundaries must continue to exist.

### Language is a boundary capability

The target relationship is:

```text
human expression
      │
      ▼
Language / Meaning Interface
      │
      ▼
structured meaning / CognitiveAct
      │
      ▼
Mind
      │
      ▼
ResponsePlan
      │
      ▼
Language Realization
      │
      ▼
human expression
```

The interface may be implemented by deterministic grammar, semantic parsers, classifiers, ranking
models, local generative models, or other local techniques. The implementation is replaceable.

Meaning objects and cognitive acts are specified separately by ADR-0031.

### Models are optional local tools

A local model MAY:

- parse, classify, or rank candidate interpretations;
- help resolve references when its output remains inspectable and non-authoritative;
- transform selected typed context into candidate hypotheses or proposals;
- assist planning under explicit criticism and authorization boundaries;
- formulate or paraphrase a `ResponsePlan`;
- summarize or translate typed state without becoming its owner;
- participate in optional learned components under ADR-0032 and ADR-0033.

A model MUST NOT become:

- identity authority;
- biography owner;
- canonical Journal writer;
- epistemic authority;
- intention owner;
- context owner;
- authorization authority;
- privileged executor;
- the only location where commitments, learned skills, or continuity exist.

Model-derived durable contributions enter the typed protocol and remain subject to the same
causality, evidence, privacy, retention, and erasure rules as other derived contributions.

### Remote inference is outside core Cybou cognition

Core Cybou does not require or silently invoke remote inference.

```text
core cognition → local only
```

A future external plugin or integration may deliberately cross a network/trust boundary, but that is
an external delivery/egress capability governed by its own policy. It is not a fallback model hidden
inside Mind, and disabling it must not remove core cognition.

The core language/meaning path must be testable with networking unavailable.

### Context delivery remains explicit

A language or model implementation does not perform unrestricted retrieval from Journal or context
storage. It consumes only context explicitly supplied through Mind's context-selection and delivery
boundaries.

Conceptually:

```text
ContextQuery
   ↓
contextd
   ↓
ContextBundle
   ↓
DeliveryPlan / consumer policy
   ↓
optional language or model implementation
```

The implementation cannot gain memory ownership merely because it can process text.

### Replacement does not change identity

Replacing, upgrading, disabling, or removing any language/model implementation MUST NOT, by itself:

- create a new Cybou identity;
- erase accepted biography;
- erase open commitments;
- change epistemic authority;
- invalidate learned state that does not depend on that implementation;
- grant or revoke execution authority.

## Consequences

Cybou can become useful before a generative model exists.

Language research and model technology may evolve independently from the storage, evidence,
continuity, learning, and authorization architecture.

A small semantic parser can coexist with a larger local model; either can later be replaced without
moving memory into hidden model context.

The design requires explicit meaning representation and response planning instead of treating a
prompt and generated text as the cognitive protocol.

Remote-model convenience is intentionally not a core feature. External egress remains possible only
through a separate, visible integration boundary.

## Relationship to learning

Learning is not defined as changing model parameters. ADR-0032 defines layered lifelong learning,
and ADR-0033 defines provenance and lifecycle rules for learned artifacts, including optional neural
artifacts.

A language implementation may learn, but Mind's learning architecture does not depend on it.

## Relationship to action

A language or model implementation may interpret, propose, explain, or criticize. Authorization and
execution remain separate under ADR-0022.

The prohibited shortcut remains:

```text
language/model implementation → privileged mutation
```

## Acceptance direction

M8 should demonstrate at least:

- Mind starts and remains cognitively usable with no generative model installed;
- a natural-language implementation can be removed without changing identity, biography, or open
  commitments;
- a language/model implementation cannot directly own or write canonical Journal storage;
- a language/model implementation receives explicit delivered context rather than unrestricted
  retrieval access;
- model-derived durable contributions use typed protocols and preserve provenance;
- model output does not bypass epistemic or authorization boundaries;
- the core meaning path is testable with network access disabled.

## Alternatives Considered

### Central LLM agent

Rejected because one probabilistic replaceable model would become the de facto owner of memory,
meaning, planning, and action.

### Mandatory local LLM

Rejected because generative fluency is not a necessary condition for cognition. It would make model
availability a hidden liveness dependency for Mind.

### Remote-model fallback

Rejected for core cognition because loss of network/provider access must not change whether Cybou can
remember, understand structured state, learn, or reason over its own Mind.

### LLM with direct Journal or context retrieval

Rejected because it would make memory architecture an implementation detail of the current model.

### LLM with direct privileged shell access

Rejected because uncertain interpretation or generation must never equal execution authority.

## Related documents

- `../MIND_MODEL.md`
- `../ROADMAP.md`
- `ADR-0001-system-architecture.md`
- `ADR-0002-cognitive-causality-and-journal-invariants.md`
- `ADR-0022-authorized-action-boundary.md`
- `ADR-0029-associative-context-projection.md`
- `ADR-0030-transparent-context-delivery.md`
- `ADR-0031-structured-meaning-and-cognitive-acts.md`
- `ADR-0032-layered-lifelong-learning.md`
- `ADR-0033-learned-artifact-governance.md`
