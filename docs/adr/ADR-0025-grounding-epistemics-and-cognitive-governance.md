<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# ADR-0025: Grounding, Epistemics, and Cognitive Governance

## Status

Proposed

## Context

Durable causal history is necessary but does not by itself distinguish observation, testimony,
inference, assumption, contradiction, or currently valid knowledge. Nor does it define retention,
resource regulation, competing priorities, or the values used to compare future plans.

Adding language or action before these boundaries would let fluent model output hide missing
provenance, uncertainty, and policy.

## Decision

The following are explicit capabilities with typed boundaries, not hidden language-model behavior:

### Perception and provenance

Perception adapters transform Body/user input into candidate observations. Every accepted
observation identifies source, acquisition time, freshness, privacy, and available integrity or
trust evidence. Adapters are replaceable faculties and never write Journal directly.

### Epistemic projection and reconciliation

Mind distinguishes observed, reported, inferred, assumed, disputed, superseded, and unknown state.
An epistemic projection links claims to supporting and contradicting contributions, scope,
confidence, freshness, and provenance. It is derived from Journal; it is not a replacement Journal.

Contradictions become explicit state. Reconciliation records why one projection changed and retains
the conflicting evidence rather than silently applying last-write-wins.

### Retention and forgetting

Retention is policy, not accidental database growth. It distinguishes durable historical record,
active projection, archive, expiry, user-requested erasure, and derived material affected by
erasure. Forgetting MUST preserve auditable semantics without retaining the sensitive content it
claims to erase. Exact tombstone, key-destruction, and replication behavior require a dedicated
implementation decision before M7 completion.

### Homeostasis and metacognition

Mind exposes typed pressure/freshness signals such as storage growth, backlog, stale projection,
latency, unresolved contradiction, calibration drift, and missing capability. Metacognition reports
what is unknown, stale, assumed, degraded, or unsupported; it must not fabricate confidence from a
fluent explanation.

### Executive attention and value constraints

Workspace admission grows into an explicit, bounded scheduling policy for interruption, deferral,
return, and competing intentions. Candidate priorities are evaluated against typed constraints such
as user authority, safety, privacy, reversibility, cost, urgency, evidence quality, and resource
budget. These constraints guide criticism and planning but do not grant execution authority.

## Ownership rule

These capabilities may eventually require new projections or services, but a process is introduced
only after its state ownership and failure semantics are specified. No `braind`, `sleepd`, or LLM
may become a shared mutable owner for all of them.

All durable derived contributions cross Event1 with causes/evidence and inherited privacy.
Perception does not equal truth, epistemic confidence does not equal authorization, and value score
does not equal permission to act.

```text
fluency            ≠ understanding
interpretation     ≠ truth
learning frequency ≠ epistemic force
learned preference ≠ permission
learned skill      ≠ authorization
```

## Ordering

The target progression is:

```text
continuity and lifecycle
→ consolidation and recovery
→ degraded modes and homeostasis
→ grounded perception and epistemic reconciliation
→ retention and cognitive governance
→ associative context and semantic activation
→ distributed continuity
→ structured language and meaning
→ lifelong learning
→ authorized action
```

### Amendment: meaning and learning governance

Natural-language interpretation is derived state. What was uttered, what it likely meant, what
epistemic force it carries and whether it is authorized are separate typed stages, and an ambiguous
reference stays ambiguous rather than being resolved into mutating authority.
[ADR-0031](ADR-0031-structured-meaning-and-cognitive-acts.md) fixes that boundary.

Learning is layered and evidence-bearing rather than hidden adaptation. Repetition does not become
truth, learned preference does not become permission, and a learned procedure does not become
authorization. Candidates cite accepted evidence and promoted artifacts stay subordinate to
epistemic and action authority. [ADR-0032](ADR-0032-layered-lifelong-learning.md) and
[ADR-0033](ADR-0033-learned-artifact-governance.md) fix those.

Both are placed after association for the same reason association was placed before language: a
faculty that arrives first decides the architecture around it by default rather than by decision.

### Amendment: associative retrieval and semantic activation

Mind **may** derive associations between concepts, episodes, preferences, claims and current tasks.

**Associative relevance is not epistemic confidence.** An association may cause information to be
considered; it may never upgrade an unknown, stale or disputed claim into knowledge. Associative
projections are reconstructible and bounded, and their owner is fixed by
[ADR-0029](ADR-0029-associative-context-projection.md).

Associative context is placed before optional language deliberately. A language faculty that arrived
first would answer questions by retrieving whatever it could reach, and the retrieval architecture
would then be whatever that model happened to do — decided by default rather than by decision.

## Consequences

Language models receive selected, provenance-bearing context instead of an undifferentiated memory
dump. Future plans can be criticized against explicit values without allowing those values to
bypass authorization. Privacy gains a lifecycle rather than only a classification label.

This adds substantial schema and policy work, so M7 is a prototype milestone and must specify a
minimal vertical slice rather than pretending to solve general knowledge representation.

## Related documents

- `../MIND_MODEL.md`
- `../ROADMAP.md`
- `ADR-0002-cognitive-causality-and-journal-invariants.md`
- `ADR-0014-workspace-admission-and-global-attention.md`
- `ADR-0018-privacy-classification-and-replication.md`
- `ADR-0021-language-models-are-optional-faculties.md`
- `ADR-0022-authorized-action-boundary.md`

